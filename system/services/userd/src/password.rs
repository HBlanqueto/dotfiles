//! Hash de contraseñas con `mkpasswd` (yescrypt). La contraseña solo viaja por
//! stdin y nunca se registra.

use std::io::Write;
use std::process::{Command, Stdio};

use crate::Error;

/// Hashea una contraseña en blocking; llamar desde `spawn_blocking`.
pub async fn hash(password: String) -> Result<String, Error> {
    tokio::task::spawn_blocking(move || hash_blocking(&password))
        .await
        .map_err(|error| Error::Password(error.to_string()))?
}

fn hash_blocking(password: &str) -> Result<String, Error> {
    let mut child = Command::new("mkpasswd")
        .args(["-m", "yescrypt", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| Error::Password(format!("could not run mkpasswd: {error}")))?;

    child
        .stdin
        .take()
        .ok_or_else(|| Error::Password("mkpasswd stdin unavailable".into()))?
        .write_all(password.as_bytes())
        .map_err(|error| Error::Password(error.to_string()))?;

    let output = child
        .wait_with_output()
        .map_err(|error| Error::Password(error.to_string()))?;
    if !output.status.success() {
        return Err(Error::Password(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
