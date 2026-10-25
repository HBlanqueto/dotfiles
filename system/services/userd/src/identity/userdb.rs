//! Backend userdb: JSON user/group records en `/etc/userdb`.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt, chown, symlink};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use keel_userd_core::ids::{Uid, Username};
use keel_userd_core::spec::UserSpec;

use crate::Error;

/// Los GID de userdb se separan de los UID para no chocar con grupos del
/// sistema (p. ej. nixbld usa el 30000).
const GID_OFFSET: u32 = 200_000;

pub struct Userdb {
    dir: PathBuf,
}

impl Userdb {
    #[must_use]
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    #[must_use]
    pub fn home(name: &Username) -> PathBuf {
        PathBuf::from("/home").join(name.as_str())
    }

    /// Crea o repara los registros del usuario y su home.
    pub fn create(&self, spec: &UserSpec, password_hash: Option<&str>) -> Result<(), Error> {
        let uid = spec
            .identity
            .uid
            .ok_or_else(|| Error::Identity("userdb requires a UID".into()))?;
        let groups = crate::identity::groups(spec);
        let gid = uid
            .get()
            .checked_add(GID_OFFSET)
            .ok_or_else(|| Error::Identity("GID out of range".into()))?;
        std::fs::create_dir_all(&self.dir)?;
        std::fs::set_permissions(&self.dir, std::fs::Permissions::from_mode(0o755))?;

        let home = Self::home(&spec.name);
        let user = serde_json::json!({
            "userName": spec.name.as_str(),
            "uid": uid.get(),
            "gid": gid,
            "realName": spec.identity.real_name.clone().unwrap_or_default(),
            "homeDirectory": home,
            "shell": spec.identity.shell.clone().unwrap_or_else(|| "/bin/sh".into()),
            "disposition": "regular",
            "memberOf": groups,
        });
        self.write_json(&format!("{}.user", spec.name), &user, 0o644)?;
        self.link(
            &format!("{}.user", spec.name),
            &format!("{}.user", uid.get()),
        )?;

        let group = serde_json::json!({
            "groupName": spec.name.as_str(),
            "gid": gid,
            "description": format!("User {}", spec.name),
        });
        self.write_json(&format!("{}.group", spec.name), &group, 0o644)?;
        self.link(&format!("{}.group", spec.name), &format!("{gid}.group"))?;

        if let Some(hash) = password_hash {
            self.write_privileged(&spec.name, uid, hash)?;
        } else {
            remove_if_exists(&self.dir.join(format!("{}.user-privileged", spec.name)))?;
            remove_if_exists(&self.dir.join(format!("{}.user-privileged", uid.get())))?;
        }

        self.write_membership(&spec.name, spec.name.as_str())?;
        for group in groups {
            self.write_membership(&spec.name, &group)?;
        }

        create_home(&home, uid, gid)?;
        Ok(())
    }

    pub fn set_password(&self, name: &Username, uid: Uid, hash: &str) -> Result<(), Error> {
        self.write_privileged(name, uid, hash)
    }

    pub fn remove(&self, name: &Username, uid: Uid) -> Result<(), Error> {
        for file in [
            format!("{name}.user"),
            format!("{}.user", uid.get()),
            format!("{name}.user-privileged"),
            format!("{}.user-privileged", uid.get()),
            format!("{name}.group"),
            format!("{}.group", uid.get()),
        ] {
            remove_if_exists(&self.dir.join(file))?;
        }

        if let Ok(entries) = std::fs::read_dir(&self.dir) {
            let prefix = format!("{name}:");
            for entry in entries.flatten() {
                let file_name = entry.file_name();
                if file_name.to_string_lossy().starts_with(&prefix) {
                    remove_if_exists(&entry.path())?;
                }
            }
        }
        Ok(())
    }

    fn write_privileged(&self, name: &Username, uid: Uid, hash: &str) -> Result<(), Error> {
        // El archivo `.user-privileged` contiene exclusivamente la sección
        // `privileged` del registro JSON, tal como lo espera `nss-systemd`.
        let record = serde_json::json!({ "privileged": { "hashedPassword": [hash] } });
        self.write_json(&format!("{name}.user-privileged"), &record, 0o600)?;
        self.link(
            &format!("{name}.user-privileged"),
            &format!("{}.user-privileged", uid.get()),
        )
    }

    fn write_json(&self, file: &str, value: &serde_json::Value, mode: u32) -> Result<(), Error> {
        let bytes = serde_json::to_vec(value)?;
        write_atomic(&self.dir.join(file), &bytes, mode)
    }

    fn write_membership(&self, user: &Username, group: &str) -> Result<(), Error> {
        let path = self
            .dir
            .join(format!("{}:{group}.membership", user.as_str()));
        write_atomic(&path, b"{}", 0o644)
    }

    fn link(&self, target: &str, link: &str) -> Result<(), Error> {
        let path = self.dir.join(link);
        remove_if_exists(&path)?;
        symlink(target, path)?;
        Ok(())
    }
}

fn create_home(path: &Path, uid: Uid, gid: u32) -> Result<(), Error> {
    std::fs::create_dir_all(path)?;
    chown(path, Some(uid.get()), Some(gid))?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
    Ok(())
}

fn remove_if_exists(path: &Path) -> Result<(), Error> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn write_atomic(path: &Path, bytes: &[u8], mode: u32) -> Result<(), Error> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::Identity("record path has no parent".into()))?;
    let temporary = parent.join(format!(
        ".{}.tmp.{}.{}",
        path.file_name()
            .map_or_else(|| "record".into(), |name| name.to_string_lossy()),
        std::process::id(),
        next_id()
    ));

    let write = || -> Result<(), Error> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(mode)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        file.set_permissions(std::fs::Permissions::from_mode(mode))?;
        drop(file);
        std::fs::rename(&temporary, path)?;
        File::open(parent)?.sync_all()?;
        Ok(())
    };

    if let Err(error) = write() {
        let _ = std::fs::remove_file(&temporary);
        return Err(error);
    }
    Ok(())
}

fn next_id() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}
