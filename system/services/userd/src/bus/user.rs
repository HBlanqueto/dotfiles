use std::sync::Arc;

use keel_userd_core::backend::IdentityBackendKind;
use zbus::interface;
use zbus::message::Header;

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

    async fn set_password(
        &self,
        #[zbus(header)] header: Header<'_>,
        #[zbus(signal_emitter)] emitter: zbus::object_server::SignalEmitter<'_>,
        password: String,
    ) -> Result<(), DbusError> {
        if !self.view.managed || self.view.backend == IdentityBackendKind::Declared {
            return Err(DbusError::ReadOnly(
                "declared users have read-only identity".into(),
            ));
        }
        if password.is_empty() {
            return Err(DbusError::InvalidConfig("password cannot be empty".into()));
        }

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
                identity::homed::set_password(connection, &self.view.name, &password)
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
}
