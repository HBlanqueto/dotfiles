//! Paths of the external data, derived from `SYSTEM_ROOT`.
use std::path::PathBuf;

pub fn root() -> String {
    let system = std::env::var("SYSTEM_ROOT").unwrap_or_default();
    if system.is_empty() {
        std::env::var("NIXDOTS_ROOT").unwrap_or_default()
    } else {
        system
    }
}

pub fn persist() -> PathBuf {
    let root = root();
    let system = PathBuf::from(format!("{root}/persist/keel"));
    let legacy = PathBuf::from(format!("{root}/persist/nixdots"));
    if !system.exists() && legacy.exists() {
        legacy
    } else {
        system
    }
}

pub fn secrets() -> PathBuf {
    PathBuf::from(format!("{}/persist/secrets", root()))
}
