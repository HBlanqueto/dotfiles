//! Reads and rewrites the external TOML data.
use color_eyre::eyre::{eyre, Context, Result};
use std::path::Path;
use toml_edit::{Array, DocumentMut, Item, Table, Value};

pub fn json_sibling(path: &Path) -> std::path::PathBuf {
    path.with_extension("json")
}

pub fn load_document(path: &Path) -> Result<DocumentMut> {
    if path.exists() {
        return parse_toml(path);
    }
    let json = json_sibling(path);
    if json.exists() {
        let text = std::fs::read_to_string(&json)
            .wrap_err_with(|| format!("no se pudo leer {}", json.display()))?;
        let value: serde_json::Value = serde_json::from_str(&text)
            .wrap_err_with(|| format!("{} no tiene el formato esperado", json.display()))?;
        return Ok(document_from_json(&value));
    }
    Ok(DocumentMut::new())
}

pub fn migrate_to_toml(path: &Path) -> Result<bool> {
    if path.exists() {
        return Ok(false);
    }
    if !json_sibling(path).exists() {
        return Ok(false);
    }
    let doc = load_document(path)?;
    write_document_atomic(path, &doc)?;
    Ok(true)
}

fn parse_toml(path: &Path) -> Result<DocumentMut> {
    let text = std::fs::read_to_string(path)
        .wrap_err_with(|| format!("no se pudo leer {}", path.display()))?;
    text.parse::<DocumentMut>()
        .wrap_err_with(|| format!("{} no tiene el formato TOML esperado", path.display()))
}

pub fn document_from_json(value: &serde_json::Value) -> DocumentMut {
    let mut doc = DocumentMut::new();
    if let Some(object) = value.as_object() {
        for (key, field) in object {
            if let Some(item) = item_from_json(field) {
                doc[key] = item;
            }
        }
    }
    doc
}

pub fn item_from_json(value: &serde_json::Value) -> Option<Item> {
    match value {
        serde_json::Value::Null => None,
        serde_json::Value::Bool(b) => Some(Item::Value(Value::from(*b))),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Some(Item::Value(Value::from(i)))
            } else {
                n.as_f64().map(|f| Item::Value(Value::from(f)))
            }
        }
        serde_json::Value::String(s) => Some(Item::Value(Value::from(s.clone()))),
        serde_json::Value::Array(a) => {
            let mut array = Array::new();
            for entry in a {
                match item_from_json(entry) {
                    Some(Item::Value(v)) => array.push(v),
                    Some(Item::Table(t)) => array.push(t.into_inline_table()),
                    _ => {}
                }
            }
            Some(Item::Value(Value::Array(array)))
        }
        serde_json::Value::Object(o) => {
            let mut table = Table::new();
            for (key, field) in o {
                if let Some(item) = item_from_json(field) {
                    table[key] = item;
                }
            }
            Some(Item::Table(table))
        }
    }
}

pub fn write_document_atomic(path: &Path, doc: &DocumentMut) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| eyre!("{} no tiene directorio padre", path.display()))?;
    std::fs::create_dir_all(parent)?;
    if path.exists() {
        let bak = path.with_extension("toml.bak");
        std::fs::copy(path, &bak)?;
        set_mode(&bak, 0o644)?;
    }
    let tmp = path.with_extension("toml.tmp");
    std::fs::write(&tmp, doc.to_string())?;
    set_mode(&tmp, 0o644)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

pub fn set_mode(path: &Path, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use toml_edit::value;

    #[test]
    fn edit_preserves_comments_order_and_unknown_fields() {
        let source = "# hola\nusername = \"humbe\"\nunknown = 1\n[git]\nname = \"x\"\n";
        let mut doc: DocumentMut = source.parse().unwrap();
        doc["username"] = value("otro");
        doc["admin"] = value(true);
        let out = doc.to_string();
        assert!(out.contains("# hola"), "se perdió el comentario: {out}");
        assert!(out.contains("unknown = 1"), "se perdió una clave: {out}");
        assert!(out.contains("username = \"otro\""));
        assert!(out.contains("admin = true"));
        assert!(out.find("username").unwrap() < out.find("admin").unwrap());
    }

    #[test]
    fn json_values_become_toml() {
        let json = serde_json::json!({
            "hostName": "x",
            "encrypt": false,
            "extras": { "epson": true },
            "list": [ "a", "b" ],
            "absent": null,
        });
        let doc = document_from_json(&json);
        let out = doc.to_string();
        assert!(out.contains("hostName = \"x\""));
        assert!(out.contains("epson = true"));
        assert!(!out.contains("absent"));
    }
}
