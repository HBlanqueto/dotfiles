//! Backend homed: crea y actualiza homes vía `org.freedesktop.home1`.

use std::io::Read;
use std::path::PathBuf;

use keel_userd_core::ids::Username;
use keel_userd_core::spec::UserSpec;
use zbus::Connection;
use zeroize::Zeroizing;

use crate::Error;
use crate::config::HomedStorage;

const MODHEX: &[u8; 16] = b"cbdefghijklnrtuv";

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

/// Ruta de los datos de homed para `directory` y `subvolume`.
#[must_use]
pub fn image_path(name: &Username) -> PathBuf {
    PathBuf::from("/home").join(format!("{}.homedir", name.as_str()))
}

fn recovery_key() -> Result<Zeroizing<String>, Error> {
    let mut bytes = [0_u8; 32];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    let mut key = String::with_capacity(71);
    for (index, byte) in bytes.iter().enumerate() {
        key.push(char::from(MODHEX[usize::from(*byte >> 4)]));
        key.push(char::from(MODHEX[usize::from(*byte & 0x0f)]));
        if index % 4 == 3 && index != 31 {
            key.push('-');
        }
    }
    Ok(Zeroizing::new(key))
}

pub async fn create(
    connection: &Connection,
    spec: &UserSpec,
    password_hash: Option<&str>,
    password: Option<&str>,
    storage: HomedStorage,
) -> Result<Option<Zeroizing<String>>, Error> {
    let uid = spec
        .identity
        .uid
        .ok_or_else(|| Error::Identity("homed requires a UID".into()))?;
    let groups = crate::identity::groups(spec);
    let recovery_key = recovery_key()?;
    let recovery_hash = crate::password::hash_secret(recovery_key.clone()).await?;
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

    let mut privileged = serde_json::json!({
        "recoveryKey": [{ "type": "modhex64", "hashedPassword": recovery_hash }],
    });
    if let Some(hash) = password_hash {
        privileged["hashedPassword"] = serde_json::json!([hash]);
    }
    record["privileged"] = privileged;
    record["recoveryKeyType"] = serde_json::json!(["modhex64"]);

    let mut secrets = vec![recovery_key.as_str()];
    if let Some(password) = password {
        secrets.push(password);
    }
    record["secret"] = serde_json::json!({ "password": secrets });

    let record = serde_json::to_string(&record)?;
    let proxy = Home1ManagerProxy::new(connection)
        .await
        .map_err(|error| Error::Identity(error.to_string()))?;
    proxy
        .create_home(&record)
        .await
        .map_err(|error| Error::Identity(format!("CreateHome: {error}")))?;
    Ok(Some(recovery_key))
}

pub async fn set_password(
    connection: &Connection,
    name: &Username,
    password: &str,
    old_secrets: &[&str],
) -> Result<(), Error> {
    let new_secret = serde_json::to_string(&serde_json::json!({
        "password": [password],
    }))?;
    let old_secret = serde_json::to_string(&serde_json::json!({
        "password": old_secrets,
    }))?;
    let proxy = Home1ManagerProxy::new(connection)
        .await
        .map_err(|error| Error::Identity(error.to_string()))?;
    proxy
        .change_password_home(name.as_str(), &new_secret, &old_secret)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_key_is_modhex_formatted() {
        let key = recovery_key().expect("recovery key");
        assert_eq!(key.len(), 71);
        for (index, character) in key.chars().enumerate() {
            if index % 9 == 8 {
                assert_eq!(character, '-');
            } else {
                assert!(MODHEX.contains(&(character as u8)), "{character}");
            }
        }
    }
}
