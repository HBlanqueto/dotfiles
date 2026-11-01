//! Machine data stored outside the repo (`machine.toml`).
use crate::store;
use color_eyre::eyre::Result;
use std::path::{Path, PathBuf};

pub fn path() -> PathBuf {
    crate::paths::persist().join("machine.toml")
}

pub fn merge_fields(fields: serde_json::Map<String, serde_json::Value>) -> Result<()> {
    merge_fields_at(&path(), fields)
}

pub fn merge_fields_at(
    path: &Path,
    fields: serde_json::Map<String, serde_json::Value>,
) -> Result<()> {
    let mut doc = store::load_document(path)?;
    for (key, field) in fields {
        match store::item_from_json(&field) {
            Some(item) => doc[key.as_str()] = item,
            None => {
                doc.remove(&key);
            }
        }
    }
    store::write_document_atomic(path, &doc)
}
