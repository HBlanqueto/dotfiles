//! Selección y operaciones de los backends de identidad.

pub mod homed;
pub mod userdb;

use keel_userd_core::backend::{Availability, BackendKind, IdentityBackendKind};
use zbus::Connection;

use crate::Error;
use crate::backends;
use crate::config::Config;

pub const WHEEL: &str = "wheel";
const EXTRA_GROUPS: [&str; 1] = ["video"];

/// Grupos del usuario: los pedidos, `wheel` si es admin y los del sistema.
#[must_use]
pub fn groups(spec: &keel_userd_core::spec::UserSpec) -> Vec<String> {
    let mut groups = spec.identity.groups.clone();
    if spec.identity.admin {
        groups.push(WHEEL.to_owned());
    }
    groups.extend(EXTRA_GROUPS.iter().map(ToString::to_string));
    groups.sort();
    groups.dedup();
    groups.retain(|group| !group.is_empty());
    groups
}

/// Elige el backend: el pedido o el primero disponible según la prioridad.
/// `declared` nunca se elige para crear.
pub async fn choose(
    config: &Config,
    connection: Option<&Connection>,
    requested: Option<IdentityBackendKind>,
) -> Result<IdentityBackendKind, Error> {
    if let Some(kind) = requested {
        if kind == IdentityBackendKind::Declared {
            return Err(Error::Forbidden("declared users are read-only".into()));
        }
        if probe(config, connection, kind).await.is_available() {
            return Ok(kind);
        }
        return Err(Error::Identity(format!(
            "identity backend `{}` is not available",
            kind.as_str()
        )));
    }

    for kind in &config.identity_priority {
        if *kind == IdentityBackendKind::Declared {
            continue;
        }
        if probe(config, connection, *kind).await.is_available() {
            return Ok(*kind);
        }
    }

    Err(Error::Identity("no identity backend is available".into()))
}

async fn probe(
    config: &Config,
    connection: Option<&Connection>,
    kind: IdentityBackendKind,
) -> Availability {
    backends::probe_kind(BackendKind::Identity(kind), connection, config).await
}
