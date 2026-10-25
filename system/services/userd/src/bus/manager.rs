use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use keel_userd_core::backend::IdentityBackendKind;
use keel_userd_core::ids::{Uid, Username};
use keel_userd_core::job::{JobKind, JobRecord};
use keel_userd_core::names::{ROOT_PATH, job_path, user_path};
use zbus::object_server::SignalEmitter;
use zbus::zvariant::{OwnedObjectPath, OwnedValue};
use zbus::{Connection, interface};

use crate::authz;
use crate::backends;
use crate::config::UidRange;
use crate::jobs::{Scheduler, handlers::CreateUserPayload};
use crate::password;
use crate::services::Services;

const HOMED_UID_RANGE: UidRange = UidRange {
    from: 60_001,
    to: 60_513,
};

/// `io.github.HBlanqueto.Keel.Userd1.Manager` en la ruta raíz.
pub struct Manager {
    pub(crate) scheduler: Arc<Scheduler>,
    pub(crate) services: Arc<Services>,
}

impl Manager {
    #[must_use]
    pub fn new(scheduler: Arc<Scheduler>, services: Arc<Services>) -> Self {
        Self {
            scheduler,
            services,
        }
    }

    async fn authorize(
        &self,
        header: &zbus::message::Header<'_>,
        action: keel_userd_core::names::PolkitAction,
    ) -> Result<(), super::DbusError> {
        let connection = self
            .services
            .connection()
            .map_err(|error| super::DbusError::Failed(error.to_string()))?
            .ok_or_else(|| super::DbusError::Failed("D-Bus connection unavailable".into()))?;
        match authz::check(&connection, header, action).await {
            Ok(true) => Ok(()),
            Ok(false) => Err(super::DbusError::PermissionDenied(
                "not authorized to manage users".into(),
            )),
            Err(error) => Err(super::DbusError::Failed(error.to_string())),
        }
    }
}

#[interface(name = "io.github.HBlanqueto.Keel.Userd1.Manager")]
impl Manager {
    fn list_users(&self) -> Vec<OwnedObjectPath> {
        self.services
            .views()
            .into_iter()
            .filter_map(|view| OwnedObjectPath::try_from(view.path()).ok())
            .collect()
    }

    fn find_user_by_name(&self, name: String) -> Result<OwnedObjectPath, super::DbusError> {
        let name = Username::new(name)
            .map_err(|error| super::DbusError::InvalidConfig(error.to_string()))?;
        let view = self
            .services
            .find_by_name(&name)
            .ok_or_else(|| super::DbusError::NotFound(name.to_string()))?;
        OwnedObjectPath::try_from(view.path())
            .map_err(|error| super::DbusError::Failed(error.to_string()))
    }

    async fn create_user(
        &self,
        #[zbus(header)] header: zbus::message::Header<'_>,
        name: String,
        options: HashMap<String, OwnedValue>,
    ) -> Result<OwnedObjectPath, super::DbusError> {
        self.authorize(&header, keel_userd_core::names::PolkitAction::UsersManage)
            .await?;

        let name = Username::new(name)
            .map_err(|error| super::DbusError::InvalidConfig(error.to_string()))?;
        if self.services.declared(&name).is_some() || self.services.managed(&name).is_some() {
            return Err(super::DbusError::Conflict(
                "user name is already in use".into(),
            ));
        }

        let options = parse_create_options(options)?;

        let connection = self
            .services
            .connection()
            .map_err(|error| super::DbusError::Failed(error.to_string()))?;
        let backend =
            crate::identity::choose(&self.services.config, connection.as_ref(), options.backend)
                .await
                .map_err(|error| super::DbusError::BackendUnavailable(error.to_string()))?;

        if backend == IdentityBackendKind::Userdb && options.password.is_some() {
            return Err(super::DbusError::InvalidConfig(
                "userdb no admite login por contraseña en NixOS; usa homed".into(),
            ));
        }

        let uid = if let Some(uid) = options.import_uid {
            if uid < 1000 {
                return Err(super::DbusError::InvalidConfig(
                    "imported UIDs below 1000 are reserved for system users".into(),
                ));
            }
            if uid >= 60_000 && backend != IdentityBackendKind::Homed {
                return Err(super::DbusError::InvalidConfig(
                    "UIDs >= 60000 are reserved for systemd-homed".into(),
                ));
            }
            if self
                .services
                .config
                .declared_users
                .iter()
                .any(|user| user.uid == uid)
            {
                return Err(super::DbusError::Conflict(
                    "UID is already used by a declared user".into(),
                ));
            }
            self.services
                .registry
                .reserve(name.as_str(), uid)
                .map_err(|error| super::DbusError::Conflict(error.to_string()))?;
            Uid::new(uid)
        } else {
            let range = match backend {
                IdentityBackendKind::Homed => HOMED_UID_RANGE,
                _ => self.services.config.uid_range,
            };
            self.services
                .registry
                .allocate(name.as_str(), range)
                .map_err(|error| super::DbusError::Failed(error.to_string()))?
        };

        let password_hash = if let Some(password) = &options.password {
            Some(
                password::hash(password.clone())
                    .await
                    .map_err(|error| super::DbusError::Failed(error.to_string()))?,
            )
        } else {
            None
        };

        let payload = CreateUserPayload {
            name: name.clone(),
            uid: uid.get(),
            backend,
            real_name: options.real_name,
            admin: options.admin,
            groups: options.groups,
            shell: options.shell,
            password_hash,
        };
        let payload = serde_json::to_string(&payload)
            .map_err(|error| super::DbusError::Failed(error.to_string()))?;

        let id = self.scheduler.next_id();
        let record = JobRecord::new(id, JobKind::CreateUser, user_path(uid), Some(name.clone()))
            .with_payload(payload);

        if backend == IdentityBackendKind::Homed
            && let Some(password) = &options.password
        {
            self.services.remember_password(id, password.clone()).await;
        }
        if let Err(error) = self.scheduler.enqueue(record).await {
            let _ = self.services.registry.release(name.as_str());
            return Err(super::DbusError::Failed(error.to_string()));
        }

        OwnedObjectPath::try_from(job_path(id))
            .map_err(|error| super::DbusError::Failed(error.to_string()))
    }

    async fn archive_user(
        &self,
        #[zbus(header)] header: zbus::message::Header<'_>,
        user: OwnedObjectPath,
        keep_home: bool,
    ) -> Result<OwnedObjectPath, super::DbusError> {
        self.authorize(&header, keel_userd_core::names::PolkitAction::UsersManage)
            .await?;

        let view = self
            .services
            .find_by_path(user.as_str())
            .ok_or_else(|| super::DbusError::NotFound(user.to_string()))?;
        if !view.managed || view.backend == IdentityBackendKind::Declared {
            return Err(super::DbusError::ReadOnly(
                "declared users are read-only".into(),
            ));
        }

        let payload = serde_json::to_string(&crate::jobs::handlers::ArchiveUserPayload {
            name: view.name.clone(),
            keep_home,
        })
        .map_err(|error| super::DbusError::Failed(error.to_string()))?;
        let id = self.scheduler.next_id();
        let record = JobRecord::new(
            id,
            JobKind::ArchiveUser,
            view.path(),
            Some(view.name.clone()),
        )
        .with_payload(payload);
        self.scheduler
            .enqueue(record)
            .await
            .map_err(|error| super::DbusError::Failed(error.to_string()))?;

        OwnedObjectPath::try_from(job_path(id))
            .map_err(|error| super::DbusError::Failed(error.to_string()))
    }

    fn list_jobs(&self) -> Result<Vec<OwnedObjectPath>, super::DbusError> {
        let records = self
            .scheduler
            .records()
            .map_err(|error| super::DbusError::Failed(error.to_string()))?;
        Ok(records
            .into_iter()
            .filter_map(|record| OwnedObjectPath::try_from(job_path(record.id)).ok())
            .collect())
    }

    async fn list_backends(&self) -> Vec<(String, String, bool)> {
        let connection = self.services.connection().ok().flatten();
        backends::probe_all(connection.as_ref(), &self.services.config)
            .await
            .into_iter()
            .map(|status| {
                let name = status.kind.as_str().to_owned();
                (name.clone(), name, status.availability.is_available())
            })
            .collect()
    }

    #[zbus(signal)]
    pub async fn user_added(emitter: &SignalEmitter<'_>, path: OwnedObjectPath)
    -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn user_removed(
        emitter: &SignalEmitter<'_>,
        path: OwnedObjectPath,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn config_rejected(
        emitter: &SignalEmitter<'_>,
        name: &str,
        reason: &str,
    ) -> zbus::Result<()>;
}

/// Registra un objeto por cada job persistido.
pub async fn register_jobs(
    connection: &Connection,
    scheduler: &Arc<Scheduler>,
) -> Result<(), crate::Error> {
    for record in scheduler.records()? {
        let path = job_path(record.id);
        connection
            .object_server()
            .at(path, super::Job::new(Arc::clone(scheduler), record.id))
            .await
            .map_err(crate::Error::Dbus)?;
    }
    Ok(())
}

pub(crate) async fn reject(connection: &Connection, name: &str, reason: &str) -> zbus::Result<()> {
    let emitter = SignalEmitter::new(connection, ROOT_PATH)?;
    Manager::config_rejected(&emitter, name, reason).await
}

struct CreateOptions {
    real_name: Option<String>,
    backend: Option<IdentityBackendKind>,
    admin: bool,
    groups: Vec<String>,
    shell: Option<String>,
    password: Option<String>,
    import_uid: Option<u32>,
}

fn parse_create_options(
    mut options: HashMap<String, OwnedValue>,
) -> Result<CreateOptions, super::DbusError> {
    let real_name = take_string(&mut options, "real-name")?;
    let backend = take_string(&mut options, "backend")?
        .map(|value| IdentityBackendKind::from_str(&value))
        .transpose()
        .map_err(|error| super::DbusError::InvalidConfig(error.to_string()))?;
    let admin = take_bool(&mut options, "admin")?.unwrap_or(false);
    let groups = take_strings(&mut options, "groups")?.unwrap_or_default();
    let shell = take_string(&mut options, "shell")?;
    let password = take_string(&mut options, "password")?;
    let import_uid = take_u32(&mut options, "uid")?;
    let import = take_bool(&mut options, "import")?.unwrap_or(false);

    if groups.iter().any(|group| group == "wheel") {
        return Err(super::DbusError::InvalidConfig(
            "use admin=true to grant wheel; do not include it in groups".into(),
        ));
    }

    if !options.is_empty() {
        return Err(super::DbusError::InvalidConfig(format!(
            "unknown options: {}",
            options.keys().cloned().collect::<Vec<_>>().join(", ")
        )));
    }
    if import && import_uid.is_none() {
        return Err(super::DbusError::InvalidConfig(
            "import requires uid".into(),
        ));
    }
    if import_uid.is_some() && !import {
        return Err(super::DbusError::InvalidConfig(
            "uid is only accepted for imports".into(),
        ));
    }

    Ok(CreateOptions {
        real_name,
        backend,
        admin,
        groups,
        shell,
        password,
        import_uid,
    })
}

fn take_string(
    options: &mut HashMap<String, OwnedValue>,
    key: &str,
) -> Result<Option<String>, super::DbusError> {
    options
        .remove(key)
        .map(|value| {
            String::try_from(value)
                .map_err(|error| super::DbusError::InvalidConfig(error.to_string()))
        })
        .transpose()
}

fn take_bool(
    options: &mut HashMap<String, OwnedValue>,
    key: &str,
) -> Result<Option<bool>, super::DbusError> {
    options
        .remove(key)
        .map(|value| {
            bool::try_from(value)
                .map_err(|error| super::DbusError::InvalidConfig(error.to_string()))
        })
        .transpose()
}

fn take_u32(
    options: &mut HashMap<String, OwnedValue>,
    key: &str,
) -> Result<Option<u32>, super::DbusError> {
    options
        .remove(key)
        .map(|value| {
            u32::try_from(value).map_err(|error| super::DbusError::InvalidConfig(error.to_string()))
        })
        .transpose()
}

fn take_strings(
    options: &mut HashMap<String, OwnedValue>,
    key: &str,
) -> Result<Option<Vec<String>>, super::DbusError> {
    options
        .remove(key)
        .map(|value| {
            Vec::<String>::try_from(value)
                .map_err(|error| super::DbusError::InvalidConfig(error.to_string()))
        })
        .transpose()
}
