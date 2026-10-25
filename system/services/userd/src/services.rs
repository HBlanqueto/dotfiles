//! Estado compartido por el bus y los handlers de jobs.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use keel_userd_core::backend::IdentityBackendKind;
use keel_userd_core::ids::{JobId, Uid, Username};
use keel_userd_core::names::{USER_INTERFACE, user_path};
use keel_userd_core::spec::UserSpec;
use zbus::Connection;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::OwnedObjectPath;

use crate::Error;
use crate::bus::{Manager, User};
use crate::config::{Config, DeclaredUser};
use crate::registry::Registry;
use crate::store::UserStore;

/// Vista de un usuario para el bus, venga de KDL o de `users.users`.
#[derive(Debug, Clone)]
pub struct UserView {
    pub name: Username,
    pub uid: Uid,
    pub backend: IdentityBackendKind,
    pub managed: bool,
    pub admin: bool,
    pub real_name: Option<String>,
    pub shell: Option<String>,
    pub home: PathBuf,
    pub state: &'static str,
}

impl UserView {
    #[must_use]
    pub fn path(&self) -> String {
        user_path(self.uid)
    }
}

pub struct Services {
    pub config: Arc<Config>,
    pub store: Arc<UserStore>,
    pub registry: Arc<Registry>,
    connection: RwLock<Option<Connection>>,
    passwords: tokio::sync::Mutex<HashMap<JobId, String>>,
}

impl Services {
    #[must_use]
    pub fn new(config: Arc<Config>, store: Arc<UserStore>, registry: Arc<Registry>) -> Self {
        Self {
            config,
            store,
            registry,
            connection: RwLock::new(None),
            passwords: tokio::sync::Mutex::new(HashMap::new()),
        }
    }

    pub fn set_connection(&self, connection: Connection) -> Result<(), Error> {
        *self
            .connection
            .write()
            .map_err(|_| Error::LockPoisoned("services connection".into()))? = Some(connection);
        Ok(())
    }

    pub fn connection(&self) -> Result<Option<Connection>, Error> {
        Ok(self
            .connection
            .read()
            .map_err(|_| Error::LockPoisoned("services connection".into()))?
            .clone())
    }

    pub async fn remember_password(&self, id: JobId, password: String) {
        self.passwords.lock().await.insert(id, password);
    }

    pub async fn take_password(&self, id: JobId) -> Option<String> {
        self.passwords.lock().await.remove(&id)
    }

    pub fn views(&self) -> Vec<UserView> {
        let mut views: Vec<UserView> = self
            .store
            .snapshot()
            .into_iter()
            .filter_map(|spec| self.managed_view(&spec))
            .collect();
        views.extend(self.config.declared_users.iter().map(declared_view));
        views.sort_by_key(|view| view.uid);
        views
    }

    pub fn find_by_name(&self, name: &Username) -> Option<UserView> {
        self.views().into_iter().find(|view| &view.name == name)
    }

    pub fn find_by_uid(&self, uid: Uid) -> Option<UserView> {
        self.views().into_iter().find(|view| view.uid == uid)
    }

    pub fn find_by_path(&self, path: &str) -> Option<UserView> {
        self.views().into_iter().find(|view| view.path() == path)
    }

    pub fn declared(&self, name: &Username) -> Option<&DeclaredUser> {
        self.config
            .declared_users
            .iter()
            .find(|user| &user.name == name)
    }

    pub fn managed(&self, name: &Username) -> Option<UserSpec> {
        self.store
            .snapshot()
            .into_iter()
            .find(|spec| &spec.name == name)
    }

    pub async fn register_user(self: &Arc<Self>, view: &UserView) -> Result<(), Error> {
        let Some(connection) = self.connection()? else {
            return Ok(());
        };
        connection
            .object_server()
            .at(view.path(), User::new(Arc::clone(self), view.clone()))
            .await?;
        Ok(())
    }

    pub async fn unregister_user(&self, uid: Uid) -> Result<(), Error> {
        let Some(connection) = self.connection()? else {
            return Ok(());
        };
        connection
            .object_server()
            .remove::<User, _>(user_path(uid))
            .await?;
        Ok(())
    }

    pub async fn emit_user_added(&self, uid: Uid) -> Result<(), Error> {
        let Some(connection) = self.connection()? else {
            return Ok(());
        };
        let path = OwnedObjectPath::try_from(user_path(uid))
            .map_err(|error| Error::Config(error.to_string()))?;
        let emitter = SignalEmitter::new(&connection, keel_userd_core::names::ROOT_PATH)?;
        Manager::user_added(&emitter, path).await?;
        Ok(())
    }

    pub async fn emit_user_removed(&self, uid: Uid) -> Result<(), Error> {
        let Some(connection) = self.connection()? else {
            return Ok(());
        };
        let path = OwnedObjectPath::try_from(user_path(uid))
            .map_err(|error| Error::Config(error.to_string()))?;
        let emitter = SignalEmitter::new(&connection, keel_userd_core::names::ROOT_PATH)?;
        Manager::user_removed(&emitter, path).await?;
        Ok(())
    }

    fn managed_view(&self, spec: &UserSpec) -> Option<UserView> {
        let uid = spec
            .identity
            .uid
            .or_else(|| self.registry.uid_for(spec.name.as_str()).ok().flatten())?;
        Some(UserView {
            name: spec.name.clone(),
            uid,
            backend: spec.identity.backend.unwrap_or(IdentityBackendKind::Userdb),
            managed: true,
            admin: spec.identity.admin,
            real_name: spec.identity.real_name.clone(),
            shell: spec.identity.shell.clone(),
            home: PathBuf::from("/home").join(spec.name.as_str()),
            state: "active",
        })
    }
}

fn declared_view(user: &DeclaredUser) -> UserView {
    UserView {
        name: user.name.clone(),
        uid: Uid::new(user.uid),
        backend: IdentityBackendKind::Declared,
        managed: false,
        admin: user.admin,
        real_name: Some(user.real_name.clone()).filter(|name| !name.is_empty()),
        shell: Some(user.shell.clone()).filter(|shell| !shell.is_empty()),
        home: PathBuf::from(&user.home),
        state: "active",
    }
}

/// Nombre de la interfaz, para el test de conformidad.
#[must_use]
pub const fn interface() -> &'static str {
    USER_INTERFACE
}
