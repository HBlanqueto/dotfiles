//! Backend homed: crea y actualiza homes vía `org.freedesktop.home1`.

use std::path::PathBuf;

use keel_userd_core::ids::Username;
use keel_userd_core::spec::UserSpec;
use zbus::Connection;

use crate::Error;
use crate::config::HomedStorage;

#[zbus::proxy(
    interface = "org.freedesktop.home1.Manager",
    default_service = "org.freedesktop.home1",
    default_path = "/org/freedesktop/home1"
)]
trait Home1Manager {
    fn create_home(&self, user_record: &str) -> zbus::Result<()>;
    fn unregister_home(&self, user_name: &str) -> zbus::Result<()>;
    fn change_password_home(
        &self,
        user_name: &str,
        new_secret: &str,
        old_secret: &str,
    ) -> zbus::Result<()>;
}

#[must_use]
pub fn home(name: &Username) -> PathBuf {
    PathBuf::from("/home").join(name.as_str())
}

pub async fn create(
    connection: &Connection,
    spec: &UserSpec,
    password_hash: Option<&str>,
    password: Option<&str>,
    storage: HomedStorage,
) -> Result<(), Error> {
    let uid = spec
        .identity
        .uid
        .ok_or_else(|| Error::Identity("homed requires a UID".into()))?;
    let groups = crate::identity::groups(spec);
    let mut record = serde_json::json!({
        "userName": spec.name.as_str(),
        "uid": uid.get(),
        "gid": uid.get(),
        "realName": spec.identity.real_name.clone().unwrap_or_default(),
        "homeDirectory": home(&spec.name),
        "shell": spec.identity.shell.clone().unwrap_or_else(|| "/bin/sh".into()),
        "disposition": "regular",
        "storage": storage.as_str(),
        "memberOf": groups,
        "enforcePasswordPolicy": false,
    });

    if let Some(hash) = password_hash {
        record["privileged"] = serde_json::json!({ "hashedPassword": [hash] });
    }
    if let Some(password) = password {
        record["secret"] = serde_json::json!({ "password": [password] });
    }

    let record = serde_json::to_string(&record)?;
    let proxy = Home1ManagerProxy::new(connection)
        .await
        .map_err(|error| Error::Identity(error.to_string()))?;
    proxy
        .create_home(&record)
        .await
        .map_err(|error| Error::Identity(format!("CreateHome: {error}")))
}

pub async fn set_password(
    connection: &Connection,
    name: &Username,
    password: &str,
) -> Result<(), Error> {
    let secret = serde_json::to_string(&serde_json::json!({
        "password": [password],
    }))?;
    let proxy = Home1ManagerProxy::new(connection)
        .await
        .map_err(|error| Error::Identity(error.to_string()))?;
    proxy
        .change_password_home(name.as_str(), &secret, "{}")
        .await
        .map_err(|error| Error::Identity(format!("ChangePasswordHome: {error}")))
}

pub async fn unregister(connection: &Connection, name: &Username) -> Result<(), Error> {
    let proxy = Home1ManagerProxy::new(connection)
        .await
        .map_err(|error| Error::Identity(error.to_string()))?;
    proxy
        .unregister_home(name.as_str())
        .await
        .map_err(|error| Error::Identity(format!("UnregisterHome: {error}")))
}
