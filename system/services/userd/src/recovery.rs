//! Custodia de la clave de recuperación de homed.
//!
//! homed exige el texto plano de la clave en cada `ChangePasswordHome`, así
//! que se guarda cifrada con la clave del host de `systemd-creds` (sin TPM).

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use keel_userd_core::ids::Username;
use zeroize::Zeroizing;

use crate::Error;
use crate::identity::homed;

/// Archivos `.cred` bajo `<stateDir>/recovery`.
pub struct RecoveryStore {
    dir: PathBuf,
}

impl RecoveryStore {
    #[must_use]
    pub fn new(state_dir: &Path) -> Self {
        Self {
            dir: state_dir.join("recovery"),
        }
    }

    fn credential_name(name: &Username) -> String {
        format!("keel-userd-recovery-{name}")
    }

    fn path(&self, name: &Username) -> PathBuf {
        self.dir.join(format!("{name}.cred"))
    }

    /// Cifra `key` y la guarda con `root:root 0600`; el directorio, `0700`.
    pub fn store(&self, name: &Username, key: &Zeroizing<String>) -> Result<(), Error> {
        fs::create_dir_all(&self.dir)?;
        fs::set_permissions(&self.dir, PermissionsExt::from_mode(0o700))?;
        std::os::unix::fs::chown(&self.dir, Some(0), Some(0))?;

        let ciphertext = encrypt(name, key)?;
        let path = self.path(name);
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&path)?;
        file.write_all(&ciphertext)?;
        file.sync_all()?;
        fs::set_permissions(&path, PermissionsExt::from_mode(0o600))?;
        std::os::unix::fs::chown(&path, Some(0), Some(0))?;
        Ok(())
    }

    /// Descifra la clave. El resultado se zeroiza al soltarlo.
    pub fn load(&self, name: &Username) -> Result<Zeroizing<String>, Error> {
        let path = self.path(name);
        if !path.is_file() {
            return Err(Error::RecoveryMissing(name.clone()));
        }

        let output = Command::new("systemd-creds")
            .args([
                "decrypt",
                "--name",
                &Self::credential_name(name),
                "--newline=no",
            ])
            .arg(&path)
            .arg("-")
            .output()
            .map_err(|error| {
                Error::Recovery(format!("no se pudo ejecutar systemd-creds: {error}"))
            })?;
        if !output.status.success() {
            return Err(Error::Recovery(format!(
                "no se pudo descifrar {}: {}",
                path.display(),
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }

        let mut bytes = Zeroizing::new(output.stdout);
        while matches!(bytes.last(), Some(b'\n' | b'\r')) {
            bytes.pop();
        }
        let key = std::str::from_utf8(&bytes)
            .map_err(|_| Error::Recovery("la clave descifrada no es UTF-8".into()))?;
        if !homed::is_recovery_key(key) {
            return Err(Error::Recovery(
                "la clave descifrada no tiene formato de recovery key".into(),
            ));
        }
        Ok(Zeroizing::new(key.to_owned()))
    }

    /// Borra la clave custodiada; no falla si ya no existe.
    pub fn remove(&self, name: &Username) -> Result<(), Error> {
        match fs::remove_file(self.path(name)) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }
}

fn encrypt(name: &Username, key: &Zeroizing<String>) -> Result<Vec<u8>, Error> {
    let mut child = Command::new("systemd-creds")
        .args([
            "encrypt",
            "--name",
            &RecoveryStore::credential_name(name),
            "--with-key=host",
            "-",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| Error::Recovery(format!("no se pudo ejecutar systemd-creds: {error}")))?;

    let Some(mut stdin) = child.stdin.take() else {
        let _ = child.kill();
        return Err(Error::Recovery("systemd-creds stdin unavailable".into()));
    };
    if let Err(error) = stdin.write_all(key.as_bytes()) {
        let _ = child.kill();
        return Err(error.into());
    }
    drop(stdin);

    let output = child
        .wait_with_output()
        .map_err(|error| Error::Recovery(format!("systemd-creds falló: {error}")))?;
    if !output.status.success() {
        return Err(Error::Recovery(format!(
            "systemd-creds encrypt falló: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(output.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_cred_is_a_clear_error() {
        let dir = std::env::temp_dir().join(format!(
            "keel-recovery-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let store = RecoveryStore::new(&dir);
        let name = Username::new("legacy").expect("valid name");
        let error = store.load(&name).expect_err("missing credential");
        assert!(matches!(error, Error::RecoveryMissing(_)));
        assert!(error.to_string().contains("ResetPassword"));
        std::fs::remove_dir_all(&dir).ok();
    }
}
