//! Paths of the external data, derived from `KEEL_ROOT`.
use std::path::PathBuf;

pub fn root() -> String {
    let keel = std::env::var("KEEL_ROOT").unwrap_or_default();
    if keel.is_empty() {
        std::env::var("NIXDOTS_ROOT").unwrap_or_default()
    } else {
        keel
    }
}

pub fn persist() -> PathBuf {
    let root = root();
    let keel = PathBuf::from(format!("{root}/persist/keel"));
    let legacy = PathBuf::from(format!("{root}/persist/nixdots"));
    if !keel.exists() && legacy.exists() {
        legacy
    } else {
        keel
    }
}

pub fn secrets() -> PathBuf {
    PathBuf::from(format!("{}/persist/secrets", root()))
}
