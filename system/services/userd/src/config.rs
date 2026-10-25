use std::path::{Path, PathBuf};

use keel_userd_core::backend::{IdentityBackendKind, PackageBackendKind};
use keel_userd_core::ids::Username;
use serde::Deserialize;

use crate::Error;

/// Rango de UIDs asignables.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(default)]
pub struct UidRange {
    pub from: u32,
    pub to: u32,
}

impl Default for UidRange {
    fn default() -> Self {
        Self {
            from: 30_000,
            to: 59_999,
        }
    }
}

impl UidRange {
    #[must_use]
    pub fn contains(self, uid: u32) -> bool {
        (self.from..=self.to).contains(&uid)
    }
}

/// Almacenamiento de los homes de homed.
#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HomedStorage {
    Directory,
    #[default]
    Subvolume,
}

impl HomedStorage {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Directory => "directory",
            Self::Subvolume => "subvolume",
        }
    }
}

/// Usuario declarado en NixOS. Su identidad es de solo lectura.
#[derive(Debug, Clone, Deserialize)]
pub struct DeclaredUser {
    pub name: Username,
    pub uid: u32,
    #[serde(default)]
    pub real_name: String,
    #[serde(default)]
    pub admin: bool,
    #[serde(default)]
    pub home: String,
    #[serde(default)]
    pub shell: String,
}

fn default_shell() -> String {
    "/bin/sh".to_owned()
}

fn default_jobs_retention() -> u64 {
    7 * 24 * 60 * 60
}

/// Configuración generada por el módulo NixOS en `/etc/keel/userd.json`.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub state_dir: PathBuf,
    pub userdb_dir: PathBuf,
    pub inputs_json: PathBuf,
    pub identity_priority: Vec<IdentityBackendKind>,
    pub package_priority: Vec<PackageBackendKind>,
    pub uid_range: UidRange,
    pub homed_storage: HomedStorage,
    pub declared_users: Vec<DeclaredUser>,
    pub max_builds: usize,
    #[serde(default = "default_shell")]
    pub default_shell: String,
    #[serde(default = "default_jobs_retention", rename = "jobs_retention_seconds")]
    pub jobs_retention: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            state_dir: PathBuf::from("/var/lib/keel-userd"),
            userdb_dir: PathBuf::from("/etc/userdb"),
            inputs_json: PathBuf::from("/etc/keel/userd-inputs.json"),
            identity_priority: vec![IdentityBackendKind::Homed, IdentityBackendKind::Declared],
            package_priority: vec![
                PackageBackendKind::NixProfile,
                PackageBackendKind::HomeManager,
                PackageBackendKind::Flatpak,
            ],
            uid_range: UidRange::default(),
            homed_storage: HomedStorage::Subvolume,
            declared_users: Vec::new(),
            max_builds: 2,
            default_shell: default_shell(),
            jobs_retention: default_jobs_retention(),
        }
    }
}

impl Config {
    /// Carga el JSON del módulo. Si no existe, usa los valores por defecto
    /// (útil para desarrollo y para el test de la VM).
    pub fn load(path: &Path) -> Result<Self, Error> {
        match std::fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|error| Error::Config(format!("{}: {error}", path.display()))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                tracing::warn!(path = %path.display(), "config not found, using defaults");
                Ok(Self::default())
            }
            Err(error) => Err(error.into()),
        }
    }

    #[must_use]
    pub fn users_dir(&self) -> PathBuf {
        self.state_dir.join("users")
    }

    #[must_use]
    pub fn archive_dir(&self) -> PathBuf {
        PathBuf::from("/home/.keel-archive")
    }

    #[must_use]
    pub fn journal_path(&self) -> PathBuf {
        self.state_dir.join("state.redb")
    }
}
