//! Nombres públicos de D-Bus y polkit.
//!
//! Cada nombre vive aquí una sola vez. Cambiar el bus a `org.nixos.*` es
//! cambiar [`BUS_NAME`] y los nombres de interfaz derivados; los XML de
//! `interfaces/` y el test de conformidad comprueban que no se desincronicen.

use crate::ids::JobId;

/// Nombre del servicio en el bus del sistema.
pub const BUS_NAME: &str = "io.github.HBlanqueto.Keel.Userd1";
/// Ruta raíz de los objetos.
pub const ROOT_PATH: &str = "/io/github/HBlanqueto/Keel/Userd1";
/// Prefijo de los errores D-Bus propios.
pub const ERROR_NAMESPACE: &str = "io.github.HBlanqueto.Keel.Userd1.Error";
/// Prefijo de las acciones polkit.
pub const POLKIT_PREFIX: &str = "io.github.HBlanqueto.keel.userd";

/// Interfaz del gestor.
pub const MANAGER_INTERFACE: &str = "io.github.HBlanqueto.Keel.Userd1.Manager";
/// Interfaz de un job.
pub const JOB_INTERFACE: &str = "io.github.HBlanqueto.Keel.Userd1.Job";
/// Interfaz de un usuario.
pub const USER_INTERFACE: &str = "io.github.HBlanqueto.Keel.Userd1.User";

/// Ruta del objeto de un job.
#[must_use]
pub fn job_path(id: JobId) -> String {
    format!("{ROOT_PATH}/Job/{}", id.get())
}

/// Ruta del objeto de un usuario.
#[must_use]
pub fn user_path(uid: crate::ids::Uid) -> String {
    format!("{ROOT_PATH}/User/{}", uid.get())
}

/// Errores propios del protocolo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    NotFound,
    PermissionDenied,
    InvalidConfig,
    BackendUnavailable,
    ReadOnly,
    Conflict,
    Failed,
}

impl ErrorKind {
    #[must_use]
    pub const fn suffix(self) -> &'static str {
        match self {
            Self::NotFound => "NotFound",
            Self::PermissionDenied => "PermissionDenied",
            Self::InvalidConfig => "InvalidConfig",
            Self::BackendUnavailable => "BackendUnavailable",
            Self::ReadOnly => "ReadOnly",
            Self::Conflict => "Conflict",
            Self::Failed => "Failed",
        }
    }

    #[must_use]
    pub fn name(self) -> String {
        format!("{ERROR_NAMESPACE}.{}", self.suffix())
    }
}

/// Acciones polkit del daemon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolkitAction {
    UsersManage,
    HomeModifyOwn,
    HomeModifyAny,
    PackagesInstallOwn,
    PackagesInstallAny,
    CapabilitiesToggle,
    UsersPasswordOwn,
}

impl PolkitAction {
    #[must_use]
    pub const fn suffix(self) -> &'static str {
        match self {
            Self::UsersManage => "users.manage",
            Self::HomeModifyOwn => "home.modify-own",
            Self::HomeModifyAny => "home.modify-any",
            Self::PackagesInstallOwn => "packages.install-own",
            Self::PackagesInstallAny => "packages.install-any",
            Self::CapabilitiesToggle => "capabilities.toggle",
            Self::UsersPasswordOwn => "users.password-own",
        }
    }

    #[must_use]
    pub fn name(self) -> String {
        format!("{POLKIT_PREFIX}.{}", self.suffix())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_namespaced() {
        assert!(MANAGER_INTERFACE.starts_with(BUS_NAME));
        assert!(JOB_INTERFACE.starts_with(BUS_NAME));
        assert!(ERROR_NAMESPACE.starts_with(BUS_NAME));
        assert!(POLKIT_PREFIX.starts_with("io.github.HBlanqueto."));
        assert_eq!(job_path(JobId::new(7)), format!("{ROOT_PATH}/Job/7"));
        assert_eq!(
            ErrorKind::InvalidConfig.name(),
            format!("{ERROR_NAMESPACE}.InvalidConfig")
        );
        assert_eq!(
            PolkitAction::UsersManage.name(),
            format!("{POLKIT_PREFIX}.users.manage")
        );
        assert_eq!(
            PolkitAction::UsersPasswordOwn.name(),
            format!("{POLKIT_PREFIX}.users.password-own")
        );
        assert_eq!(
            user_path(crate::ids::Uid::new(60_001)),
            format!("{ROOT_PATH}/User/60001")
        );
    }
}
