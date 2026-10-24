use std::path::Path;

use keel_userd_core::backend::{
    Availability, BackendKind, IdentityBackendKind, PackageBackendKind, UnavailableReason,
};
use rustix::fs::{Access, access};
use zbus::Connection;
use zbus::names::BusName;

use crate::config::Config;

/// Backend con su disponibilidad actual.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendStatus {
    pub kind: BackendKind,
    pub availability: Availability,
}

/// Sondea todos los backends conocidos, en el orden de prioridad configurado.
pub async fn probe_all(connection: Option<&Connection>, config: &Config) -> Vec<BackendStatus> {
    let mut kinds = Vec::new();
    for kind in &config.identity_priority {
        push_unique(&mut kinds, BackendKind::Identity(*kind));
    }
    for kind in &config.package_priority {
        push_unique(&mut kinds, BackendKind::Package(*kind));
    }
    for kind in [
        IdentityBackendKind::Homed,
        IdentityBackendKind::Userdb,
        IdentityBackendKind::Declared,
    ] {
        push_unique(&mut kinds, BackendKind::Identity(kind));
    }
    for kind in [
        PackageBackendKind::NixProfile,
        PackageBackendKind::HomeManager,
        PackageBackendKind::Flatpak,
    ] {
        push_unique(&mut kinds, BackendKind::Package(kind));
    }

    let mut statuses = Vec::with_capacity(kinds.len());
    for kind in kinds {
        statuses.push(BackendStatus {
            kind,
            availability: probe_kind(kind, connection, config).await,
        });
    }
    statuses
}

fn push_unique(kinds: &mut Vec<BackendKind>, kind: BackendKind) {
    if !kinds.contains(&kind) {
        kinds.push(kind);
    }
}

pub async fn probe_kind(
    kind: BackendKind,
    connection: Option<&Connection>,
    config: &Config,
) -> Availability {
    match kind {
        BackendKind::Identity(IdentityBackendKind::Homed) => {
            dbus_name(connection, "org.freedesktop.home1").await
        }
        BackendKind::Identity(IdentityBackendKind::Userdb) => probe_userdb(&config.userdb_dir),
        BackendKind::Identity(IdentityBackendKind::Declared) => Availability::Available,
        BackendKind::Package(PackageBackendKind::NixProfile) => probe_nix_profile(),
        BackendKind::Package(PackageBackendKind::HomeManager) => probe_home_manager(config),
        BackendKind::Package(PackageBackendKind::Flatpak) => {
            if !in_path("flatpak") {
                return Availability::Unavailable(UnavailableReason::NotInstalled);
            }
            dbus_name(connection, "org.freedesktop.Flatpak.SystemHelper").await
        }
    }
}

fn probe_nix_profile() -> Availability {
    if nix_daemon_available() {
        Availability::Available
    } else {
        Availability::Unavailable(UnavailableReason::NotRunning)
    }
}

/// home-manager necesita tanto sus inputs como el daemon de Nix.
fn probe_home_manager(config: &Config) -> Availability {
    if !config.inputs_json.exists() {
        Availability::Unavailable(UnavailableReason::NotInstalled)
    } else if nix_daemon_available() {
        Availability::Available
    } else {
        Availability::Unavailable(UnavailableReason::NotRunning)
    }
}

fn nix_daemon_available() -> bool {
    Path::new("/nix/var/nix/daemon-socket/socket").exists()
}

/// userdb necesita escribir `/etc/userdb`; si no se puede, queda no disponible.
fn probe_userdb(directory: &Path) -> Availability {
    match access(directory, Access::WRITE_OK) {
        Ok(()) => Availability::Available,
        Err(rustix::io::Errno::ACCESS | rustix::io::Errno::PERM) => {
            Availability::Unavailable(UnavailableReason::NoPermission)
        }
        Err(error) => Availability::Unavailable(UnavailableReason::Io(format!(
            "{}: {error}",
            directory.display()
        ))),
    }
}

async fn dbus_name(connection: Option<&Connection>, name: &str) -> Availability {
    let Some(connection) = connection else {
        return Availability::Unavailable(UnavailableReason::NotRunning);
    };

    let proxy = match zbus::fdo::DBusProxy::new(connection).await {
        Ok(proxy) => proxy,
        Err(error) => return Availability::Unavailable(UnavailableReason::Io(error.to_string())),
    };

    let bus_name = match BusName::try_from(name) {
        Ok(bus_name) => bus_name,
        Err(error) => return Availability::Unavailable(UnavailableReason::Io(error.to_string())),
    };

    match proxy.name_has_owner(bus_name).await {
        Ok(true) => return Availability::Available,
        Ok(false) => {}
        Err(error) => return Availability::Unavailable(UnavailableReason::Io(error.to_string())),
    }

    match proxy.list_activatable_names().await {
        Ok(names) if names.iter().any(|candidate| candidate.as_str() == name) => {
            Availability::Available
        }
        Ok(_) => Availability::Unavailable(UnavailableReason::NotRunning),
        Err(error) => Availability::Unavailable(UnavailableReason::Io(error.to_string())),
    }
}

fn in_path(program: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|directory| directory.join(program).is_file())
    })
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn userdb_requires_a_writable_directory() {
        assert!(matches!(
            probe_userdb(Path::new("/nonexistent-userd-userdb")),
            Availability::Unavailable(_)
        ));

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("userd-userdb-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("temp dir");
        assert_eq!(probe_userdb(&directory), Availability::Available);
        std::fs::remove_dir_all(&directory).ok();
    }
}
