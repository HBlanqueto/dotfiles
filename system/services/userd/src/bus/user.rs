use std::sync::Arc;

use keel_userd_core::backend::IdentityBackendKind;
use zbus::interface;
use zbus::message::Header;
use zeroize::Zeroizing;

use crate::authz;
use crate::bus::DbusError;
use crate::identity::{self};
use crate::password;
use crate::services::{Services, UserView};

/// `io.github.HBlanqueto.Keel.Userd1.User` en `/…/User/<uid>`.
pub struct User {
    services: Arc<Services>,
    view: UserView,
}

fn custody_error(error: crate::Error) -> DbusError {
    match &error {
        crate::Error::RecoveryMissing(_) => DbusError::InvalidConfig(error.to_string()),
        _ => DbusError::Failed(error.to_string()),
    }
}

impl User {
    #[must_use]
    pub fn new(services: Arc<Services>, view: UserView) -> Self {
        Self { services, view }
    }
}

#[interface(name = "io.github.HBlanqueto.Keel.Userd1.User")]
impl User {
    #[zbus(property)]
    fn name(&self) -> String {
        self.view.name.to_string()
    }

    #[zbus(property)]
    fn uid(&self) -> u32 {
        self.view.uid.get()
    }

    #[zbus(property)]
    fn backend(&self) -> String {
        self.view.backend.as_str().to_owned()
    }

    #[zbus(property)]
    fn managed(&self) -> bool {
        self.view.managed
    }

    #[zbus(property)]
    fn state(&self) -> String {
        self.view.state.to_owned()
    }

    async fn reset_password(
        &self,
        #[zbus(header)] header: Header<'_>,
        #[zbus(signal_emitter)] emitter: zbus::object_server::SignalEmitter<'_>,
        recovery_key: String,
        new_password: String,
    ) -> Result<(), DbusError> {
        if !self.view.managed || self.view.backend == IdentityBackendKind::Declared {
            return Err(DbusError::ReadOnly(
                "declared users have read-only identity".into(),
            ));
        }
        if self.view.backend != IdentityBackendKind::Homed {
            return Err(DbusError::InvalidConfig(
                "recovery key reset is only available for homed users".into(),
            ));
        }
        if recovery_key.is_empty() || new_password.is_empty() {
            return Err(DbusError::InvalidConfig(
                "recovery key and new password cannot be empty".into(),
            ));
        }
        let recovery_key = Zeroizing::new(recovery_key);
        let new_password = Zeroizing::new(new_password);

        let connection = emitter.connection();
        let caller = authz::caller_uid(connection, &header)
            .await
            .map_err(|error| DbusError::Failed(error.to_string()))?;
        if caller != self.view.uid.get() {
            match authz::check(
                connection,
                &header,
                keel_userd_core::names::PolkitAction::UsersManage,
            )
            .await
            {
                Ok(true) => {}
                Ok(false) => {
                    return Err(DbusError::PermissionDenied(
                        "not authorized to reset this password".into(),
                    ));
                }
                Err(error) => return Err(DbusError::Failed(error.to_string())),
            }
        }

        identity::homed::set_password(
            connection,
            &self.view.name,
            new_password.as_str(),
            &[recovery_key.as_str()],
        )
        .await
        .map_err(|error| DbusError::Failed(error.to_string()))
    }

    async fn set_password(
        &self,
        #[zbus(header)] header: Header<'_>,
        #[zbus(signal_emitter)] emitter: zbus::object_server::SignalEmitter<'_>,
        password: String,
        old_password: String,
    ) -> Result<(), DbusError> {
        if !self.view.managed || self.view.backend == IdentityBackendKind::Declared {
            return Err(DbusError::ReadOnly(
                "declared users have read-only identity".into(),
            ));
        }
        if password.is_empty() {
            return Err(DbusError::InvalidConfig("password cannot be empty".into()));
        }
        if old_password.is_empty() {
            return Err(DbusError::InvalidConfig(
                "old password cannot be empty".into(),
            ));
        }
        let password = Zeroizing::new(password);
        let old_password = Zeroizing::new(old_password);

        let connection = emitter.connection();
        let caller = authz::caller_uid(connection, &header)
            .await
            .map_err(|error| DbusError::Failed(error.to_string()))?;
        let action = if caller == self.view.uid.get() {
            keel_userd_core::names::PolkitAction::UsersPasswordOwn
        } else {
            keel_userd_core::names::PolkitAction::UsersManage
        };
        match authz::check(connection, &header, action).await {
            Ok(true) => {}
            Ok(false) => {
                return Err(DbusError::PermissionDenied(
                    "not authorized to change this password".into(),
                ));
            }
            Err(error) => return Err(DbusError::Failed(error.to_string())),
        }

        match self.view.backend {
            IdentityBackendKind::Userdb => {
                let hash = password::hash(password)
                    .await
                    .map_err(|error| DbusError::Failed(error.to_string()))?;
                identity::userdb::Userdb::new(self.services.config.userdb_dir.clone())
                    .set_password(&self.view.name, self.view.uid, &hash)
                    .map_err(|error| DbusError::Failed(error.to_string()))?;
            }
            IdentityBackendKind::Homed => {
                let recovery_key = self
                    .services
                    .recovery
                    .load(&self.view.name)
                    .map_err(custody_error)?;
                identity::homed::set_password(
                    connection,
                    &self.view.name,
                    password.as_str(),
                    &[old_password.as_str(), recovery_key.as_str()],
                )
                .await
                .map_err(|error| DbusError::Failed(error.to_string()))?;
            }
            IdentityBackendKind::Declared => {
                return Err(DbusError::ReadOnly(
                    "declared users have read-only identity".into(),
                ));
            }
        }
        Ok(())
    }

    async fn admin_reset_password(
        &self,
        #[zbus(header)] header: Header<'_>,
        #[zbus(signal_emitter)] emitter: zbus::object_server::SignalEmitter<'_>,
        new_password: String,
    ) -> Result<(), DbusError> {
        if !self.view.managed || self.view.backend == IdentityBackendKind::Declared {
            return Err(DbusError::ReadOnly(
                "declared users have read-only identity".into(),
            ));
        }
        if self.view.backend != IdentityBackendKind::Homed {
            return Err(DbusError::InvalidConfig(
                "admin reset is only available for homed users".into(),
            ));
        }
        if new_password.is_empty() {
            return Err(DbusError::InvalidConfig(
                "new password cannot be empty".into(),
            ));
        }
        let new_password = Zeroizing::new(new_password);

        let connection = emitter.connection();
        match authz::check(
            connection,
            &header,
            keel_userd_core::names::PolkitAction::UsersManage,
        )
        .await
        {
            Ok(true) => {}
            Ok(false) => {
                return Err(DbusError::PermissionDenied(
                    "not authorized to reset this password".into(),
                ));
            }
            Err(error) => return Err(DbusError::Failed(error.to_string())),
        }

        let recovery_key = self
            .services
            .recovery
            .load(&self.view.name)
            .map_err(custody_error)?;
        identity::homed::set_password(
            connection,
            &self.view.name,
            new_password.as_str(),
            &[recovery_key.as_str()],
        )
        .await
        .map_err(|error| DbusError::Failed(error.to_string()))
    }
}
