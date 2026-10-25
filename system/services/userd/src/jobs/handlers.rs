//! Handlers de los jobs de identidad.

use std::future::Future;
use std::pin::Pin;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use keel_userd_core::backend::IdentityBackendKind;
use keel_userd_core::ids::{Uid, Username};
use keel_userd_core::job::{JobFailure, JobRecord};
use keel_userd_core::spec::{IdentitySpec, UserSpec};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::Error;
use crate::identity::{homed, userdb};
use crate::jobs::Scheduler;
use crate::services::Services;

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUserPayload {
    pub name: Username,
    pub uid: u32,
    pub backend: IdentityBackendKind,
    #[serde(default)]
    pub real_name: Option<String>,
    #[serde(default)]
    pub admin: bool,
    #[serde(default)]
    pub groups: Vec<String>,
    #[serde(default)]
    pub shell: Option<String>,
    #[serde(default)]
    pub password_hash: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ArchiveUserPayload {
    pub name: Username,
    pub keep_home: bool,
}

pub struct CreateUserHandler {
    services: Arc<Services>,
    scheduler: Arc<Scheduler>,
}

impl CreateUserHandler {
    #[must_use]
    pub fn new(services: Arc<Services>, scheduler: Arc<Scheduler>) -> Self {
        Self {
            services,
            scheduler,
        }
    }

    async fn create(&self, job: &JobRecord) -> Result<(), Error> {
        let payload: CreateUserPayload = serde_json::from_str(
            job.payload
                .as_deref()
                .ok_or_else(|| Error::Config("missing CreateUser payload".into()))?,
        )?;

        if self.services.declared(&payload.name).is_some() {
            return Err(Error::Forbidden(
                "user name belongs to a declared user".into(),
            ));
        }

        let uid = Uid::new(payload.uid);
        let spec = if let Some(existing) = self.services.managed(&payload.name) {
            if existing.identity.uid != Some(uid)
                || existing.identity.backend != Some(payload.backend)
            {
                return Err(Error::Identity(
                    "user already exists with a different identity".into(),
                ));
            }
            existing
        } else {
            let mut spec = UserSpec::new(payload.name.clone());
            spec.identity = IdentitySpec {
                backend: Some(payload.backend),
                uid: Some(uid),
                real_name: payload.real_name.clone(),
                shell: payload
                    .shell
                    .clone()
                    .or_else(|| Some(self.services.config.default_shell.clone())),
                groups: payload.groups.clone(),
                admin: payload.admin,
            };

            match payload.backend {
                IdentityBackendKind::Userdb => {
                    userdb::Userdb::new(self.services.config.userdb_dir.clone())
                        .create(&spec, payload.password_hash.as_deref())?;
                }
                IdentityBackendKind::Homed => {
                    let connection = self
                        .services
                        .connection()?
                        .ok_or_else(|| Error::Identity("D-Bus connection unavailable".into()))?;
                    let password = self.services.take_password(job.id).await;
                    if payload.password_hash.is_some() && password.is_none() {
                        return Err(Error::Identity(
                            "contraseña perdida tras el reinicio, repite CreateUser".into(),
                        ));
                    }
                    let recovery_key = homed::create(
                        &connection,
                        &spec,
                        payload.password_hash.as_deref(),
                        password.as_deref(),
                        self.services.config.homed_storage,
                    )
                    .await?;
                    if let Some(recovery_key) = recovery_key {
                        self.scheduler.set_result(job.id, recovery_key)?;
                    }
                }
                IdentityBackendKind::Declared => {
                    return Err(Error::Forbidden("declared users are read-only".into()));
                }
            }

            self.services.store.write(&spec)?;
            spec
        };

        let view = self
            .services
            .find_by_name(&spec.name)
            .ok_or_else(|| Error::Identity("user view disappeared".into()))?;
        self.services.register_user(&view).await?;
        self.services.emit_user_added(view.uid).await?;
        Ok(())
    }
}

impl super::Handler for CreateUserHandler {
    fn run<'a>(
        &'a self,
        job: &'a JobRecord,
    ) -> Pin<Box<dyn Future<Output = Result<(), JobFailure>> + Send + 'a>> {
        Box::pin(async move {
            self.create(job)
                .await
                .map_err(|error| JobFailure::new(error.to_string()))
        })
    }
}

pub struct ArchiveUserHandler {
    services: Arc<Services>,
}

impl ArchiveUserHandler {
    #[must_use]
    pub fn new(services: Arc<Services>) -> Self {
        Self { services }
    }

    async fn archive(&self, job: &JobRecord) -> Result<(), Error> {
        let payload: ArchiveUserPayload = serde_json::from_str(
            job.payload
                .as_deref()
                .ok_or_else(|| Error::Config("missing ArchiveUser payload".into()))?,
        )?;
        let view = self
            .services
            .find_by_name(&payload.name)
            .ok_or_else(|| Error::Forbidden("user does not exist".into()))?;
        if !view.managed || view.backend == IdentityBackendKind::Declared {
            return Err(Error::Forbidden("declared users are read-only".into()));
        }

        let admins = self
            .services
            .views()
            .into_iter()
            .filter(|user| user.admin)
            .count();
        if view.admin && admins <= 1 {
            return Err(Error::Forbidden("cannot archive the last admin".into()));
        }
        if is_busy(&view.name).await? {
            return Err(Error::Busy(format!(
                "user {} has running processes",
                view.name
            )));
        }

        if view.backend == IdentityBackendKind::Userdb {
            let home = userdb::Userdb::home(&view.name);
            if home.exists() {
                if payload.keep_home {
                    // Bloquea el home en /home: root puede acceder, el usuario no.
                    std::os::unix::fs::chown(&home, Some(0), Some(0))?;
                    std::fs::set_permissions(
                        &home,
                        std::os::unix::fs::PermissionsExt::from_mode(0o700),
                    )?;
                } else {
                    self.move_home(&view.name, &home)?;
                }
            }
        }

        match view.backend {
            IdentityBackendKind::Userdb => {
                userdb::Userdb::new(self.services.config.userdb_dir.clone())
                    .remove(&view.name, view.uid)?;
            }
            IdentityBackendKind::Homed => {
                if !payload.keep_home {
                    let home = homed::image_path(&view.name);
                    if home.exists() {
                        self.move_home(&view.name, &home)?;
                    }
                }
                let connection = self
                    .services
                    .connection()?
                    .ok_or_else(|| Error::Identity("D-Bus connection unavailable".into()))?;
                homed::unregister(&connection, &view.name).await?;
            }
            IdentityBackendKind::Declared => {
                return Err(Error::Forbidden("declared users are read-only".into()));
            }
        }

        self.services.store.remove(&view.name)?;
        self.services.registry.release(view.name.as_str())?;
        self.services.unregister_user(view.uid).await?;
        self.services.emit_user_removed(view.uid).await?;
        Ok(())
    }

    fn move_home(&self, name: &Username, home: &std::path::Path) -> Result<(), Error> {
        let archive = self.services.config.archive_dir();
        std::fs::create_dir_all(&archive)?;
        std::fs::set_permissions(
            &archive,
            std::os::unix::fs::PermissionsExt::from_mode(0o700),
        )?;
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let target = archive.join(format!("{name}-{nanos}"));
        if let Err(error) = std::fs::rename(home, &target) {
            if error.kind() == std::io::ErrorKind::CrossesDevices
                || error.raw_os_error() == Some(18)
            {
                return Err(Error::Identity(
                    "home and archive are on different filesystems; move it manually".into(),
                ));
            }
            return Err(error.into());
        }
        Ok(())
    }
}

impl super::Handler for ArchiveUserHandler {
    fn run<'a>(
        &'a self,
        job: &'a JobRecord,
    ) -> Pin<Box<dyn Future<Output = Result<(), JobFailure>> + Send + 'a>> {
        Box::pin(async move {
            self.archive(job)
                .await
                .map_err(|error| JobFailure::new(error.to_string()))
        })
    }
}

async fn is_busy(name: &Username) -> Result<bool, Error> {
    let name = name.to_string();
    tokio::task::spawn_blocking(move || {
        let output = Command::new("loginctl")
            .args(["show-user", &name, "-p", "State", "--value"])
            .output();
        match output {
            Ok(output) if output.status.success() => {
                let state = String::from_utf8_lossy(&output.stdout).trim().to_owned();
                Ok(matches!(
                    state.as_str(),
                    "active" | "online" | "lingering" | "closing"
                ))
            }
            Ok(_) => Ok(false),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(Error::Io(error)),
        }
    })
    .await
    .map_err(|error| Error::Identity(error.to_string()))?
}
