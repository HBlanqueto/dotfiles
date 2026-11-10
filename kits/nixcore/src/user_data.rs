//! Per-user account data stored outside the repo.
use crate::store;
use color_eyre::eyre::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use toml_edit::{value, Item, Table};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct A11y {
    #[serde(default)]
    pub large_text: bool,
    #[serde(default)]
    pub high_contrast: bool,
    #[serde(default)]
    pub screen_reader: bool,
    #[serde(default)]
    pub reduce_motion: bool,
    #[serde(default)]
    pub screen_keyboard: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Git {
    pub name: String,
    pub email: String,
}

fn default_true() -> bool {
    true
}

fn default_version() -> u32 {
    1
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserData {
    #[serde(default = "default_version")]
    pub version: u32,
    pub username: String,
    pub full_name: String,
    #[serde(default = "default_true")]
    pub admin: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prefs {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_scheme: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent_color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub a11y: Option<A11y>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git: Option<Git>,
}

pub fn users_dir() -> PathBuf {
    crate::paths::persist().join("users")
}

pub fn path_for(username: &str) -> PathBuf {
    users_dir().join(format!("{username}.toml"))
}

pub fn prefs_path_for(username: &str) -> PathBuf {
    users_dir().join(format!("{username}.prefs.toml"))
}

fn set_str(doc: &mut toml_edit::DocumentMut, key: &str, s: Option<&str>) {
    match s {
        Some(v) => doc[key] = value(v),
        None => {
            doc.remove(key);
        }
    }
}

impl UserData {
    pub fn read(username: &str) -> Result<Option<Self>> {
        let path = path_for(username);
        if !path.exists() && !store::json_sibling(&path).exists() {
            return Ok(None);
        }
        let doc = store::load_document(&path)?;
        let username = doc
            .get("username")
            .and_then(|i| i.as_str())
            .unwrap_or(username)
            .to_string();
        Ok(Some(Self {
            version: doc.get("version").and_then(|i| i.as_integer()).unwrap_or(1) as u32,
            username,
            full_name: doc
                .get("fullName")
                .and_then(|i| i.as_str())
                .unwrap_or("")
                .to_string(),
            admin: doc.get("admin").and_then(|i| i.as_bool()).unwrap_or(false),
        }))
    }

    pub fn list() -> Result<Vec<Self>> {
        let mut users = Vec::new();
        let Ok(entries) = std::fs::read_dir(users_dir()) else {
            return Ok(users);
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if !name.ends_with(".toml") || name.ends_with(".prefs.toml") {
                continue;
            }
            let username = name.trim_end_matches(".toml");
            if let Some(user) = Self::read(username)? {
                users.push(user);
            }
        }
        users.sort_by(|a, b| a.username.cmp(&b.username));
        Ok(users)
    }

    pub fn write(&self) -> Result<()> {
        let path = path_for(&self.username);
        let mut doc = store::load_document(&path)?;
        doc["version"] = value(self.version as i64);
        doc["username"] = value(&self.username);
        doc["fullName"] = value(&self.full_name);
        doc["admin"] = value(self.admin);
        for key in [
            "template",
            "personalConfig",
            "git",
            "colorScheme",
            "accentColor",
            "avatarColor",
            "avatarPath",
            "a11y",
        ] {
            doc.remove(key);
        }
        store::write_document_atomic(&path, &doc)
    }
}

fn a11y_table(a11y: &A11y) -> Table {
    let mut table = Table::new();
    table["largeText"] = value(a11y.large_text);
    table["highContrast"] = value(a11y.high_contrast);
    table["screenReader"] = value(a11y.screen_reader);
    table["reduceMotion"] = value(a11y.reduce_motion);
    table["screenKeyboard"] = value(a11y.screen_keyboard);
    table
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            version: 1,
            color_scheme: None,
            accent_color: None,
            avatar_color: None,
            avatar_path: None,
            a11y: None,
            git: None,
        }
    }
}

impl Prefs {
    pub fn write(&self, username: &str) -> Result<()> {
        let path = prefs_path_for(username);
        self.write_seed(&path)?;
        store::set_mode(&path, 0o600)?;
        if let Some((uid, gid)) = owner(username) {
            std::os::unix::fs::chown(&path, Some(uid), Some(gid))?;
        }
        Ok(())
    }

    pub fn write_home_file(&self, path: &Path) -> Result<()> {
        self.write_seed(path)?;
        store::set_mode(path, 0o600)?;
        Ok(())
    }

    fn write_seed(&self, path: &Path) -> Result<()> {
        let mut doc = store::load_document(path)?;
        doc["version"] = value(self.version as i64);
        set_str(&mut doc, "colorScheme", self.color_scheme.as_deref());
        set_str(&mut doc, "accentColor", self.accent_color.as_deref());
        set_str(&mut doc, "avatarColor", self.avatar_color.as_deref());
        set_str(&mut doc, "avatarPath", self.avatar_path.as_deref());
        match &self.a11y {
            Some(a11y) => doc["a11y"] = Item::Table(a11y_table(a11y)),
            None => {
                doc.remove("a11y");
            }
        }
        match &self.git {
            Some(git) => {
                let mut table = Table::new();
                table["name"] = value(&git.name);
                table["email"] = value(&git.email);
                doc["git"] = Item::Table(table);
            }
            None => {
                doc.remove("git");
            }
        }
        store::write_document_atomic(path, &doc)
    }

    pub fn read(username: &str) -> Result<Option<Self>> {
        let path = prefs_path_for(username);
        if !path.exists() {
            return Ok(None);
        }
        let doc = store::load_document(&path)?;
        let get = |key: &str| doc.get(key).and_then(|i| i.as_str()).map(str::to_string);
        let a11y = doc.get("a11y").and_then(|i| i.as_table()).map(|t| A11y {
            large_text: t
                .get("largeText")
                .and_then(|i| i.as_bool())
                .unwrap_or(false),
            high_contrast: t
                .get("highContrast")
                .and_then(|i| i.as_bool())
                .unwrap_or(false),
            screen_reader: t
                .get("screenReader")
                .and_then(|i| i.as_bool())
                .unwrap_or(false),
            reduce_motion: t
                .get("reduceMotion")
                .and_then(|i| i.as_bool())
                .unwrap_or(false),
            screen_keyboard: t
                .get("screenKeyboard")
                .and_then(|i| i.as_bool())
                .unwrap_or(false),
        });
        let git = doc.get("git").and_then(|i| i.as_table()).and_then(|t| {
            Some(Git {
                name: t.get("name")?.as_str()?.to_string(),
                email: t.get("email")?.as_str()?.to_string(),
            })
        });
        Ok(Some(Self {
            version: doc.get("version").and_then(|i| i.as_integer()).unwrap_or(1) as u32,
            color_scheme: get("colorScheme"),
            accent_color: get("accentColor"),
            avatar_color: get("avatarColor"),
            avatar_path: get("avatarPath"),
            a11y,
            git,
        }))
    }
}

fn owner(username: &str) -> Option<(u32, u32)> {
    let passwd = std::fs::read_to_string("/etc/passwd").ok()?;
    passwd.lines().find_map(|line| {
        let fields: Vec<&str> = line.split(':').collect();
        if fields.len() > 3 && fields[0] == username {
            Some((fields[2].parse().ok()?, fields[3].parse().ok()?))
        } else {
            None
        }
    })
}

