use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use keel_userd_core::ids::Username;
use keel_userd_core::spec::UserSpec;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use crate::Error;

/// Evento que el daemon convierte en señal D-Bus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreEvent {
    ConfigRejected { name: String, reason: String },
}

/// Última configuración válida de cada usuario, y su escritura atómica.
pub struct UserStore {
    users_dir: PathBuf,
    users: RwLock<BTreeMap<Username, UserSpec>>,
    events: UnboundedSender<StoreEvent>,
    watcher: Mutex<Option<RecommendedWatcher>>,
}

impl UserStore {
    /// Prepara el directorio y carga lo que ya hubiera. La config inválida no
    /// entra en memoria: se emite `ConfigRejected` y se conserva lo anterior.
    pub fn open(users_dir: PathBuf) -> Result<(Self, UnboundedReceiver<StoreEvent>), Error> {
        std::fs::create_dir_all(&users_dir)?;
        std::fs::set_permissions(&users_dir, std::fs::Permissions::from_mode(0o700))?;

        let (events, receiver) = unbounded_channel();
        let store = Self {
            users_dir,
            users: RwLock::new(BTreeMap::new()),
            events,
            watcher: Mutex::new(None),
        };
        store.load_all();
        Ok((store, receiver))
    }

    /// Arranca el watcher inotify sobre el directorio de usuarios.
    pub fn start_watcher(self: &Arc<Self>) -> Result<(), Error> {
        let weak = Arc::downgrade(self);
        let mut watcher =
            notify::recommended_watcher(move |result: notify::Result<Event>| match result {
                Ok(event) => {
                    if let Some(store) = weak.upgrade() {
                        store.handle_event(&event);
                    }
                }
                Err(error) => tracing::warn!(%error, "inotify error"),
            })
            .map_err(std::io::Error::other)?;

        watcher
            .watch(&self.users_dir, RecursiveMode::NonRecursive)
            .map_err(std::io::Error::other)?;

        let mut slot = self
            .watcher
            .lock()
            .map_err(|_| Error::Config("watcher mutex poisoned".into()))?;
        *slot = Some(watcher);
        Ok(())
    }

    #[must_use]
    pub fn snapshot(&self) -> Vec<UserSpec> {
        match self.users.read() {
            Ok(users) => users.values().cloned().collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Escribe el KDL de forma atómica y con permisos 0600.
    pub fn write(&self, spec: &UserSpec) -> Result<(), Error> {
        let text = spec.to_kdl_string()?;
        let final_path = self.users_dir.join(format!("{}.kdl", spec.name));
        let temporary = self.users_dir.join(format!(
            ".{}.kdl.{}.{}.tmp",
            spec.name,
            std::process::id(),
            next_temp_id()
        ));

        let write = || -> Result<(), Error> {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&temporary)?;
            file.write_all(text.as_bytes())?;
            file.sync_all()?;
            file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
            drop(file);

            std::fs::rename(&temporary, &final_path)?;
            File::open(&self.users_dir)?.sync_all()?;
            Ok(())
        };

        if let Err(error) = write() {
            let _ = std::fs::remove_file(&temporary);
            return Err(error);
        }

        self.insert(spec.clone());
        Ok(())
    }

    /// Borra el KDL del usuario y su copia en memoria.
    pub fn remove(&self, name: &Username) -> Result<(), Error> {
        let path = self.users_dir.join(format!("{name}.kdl"));
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        self.users
            .write()
            .map_err(|_| Error::LockPoisoned("users".into()))?
            .remove(name);
        Ok(())
    }

    fn load_all(&self) {
        let Ok(entries) = std::fs::read_dir(&self.users_dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|extension| extension == "kdl") {
                self.reload(&path);
            }
        }
    }

    fn handle_event(&self, event: &Event) {
        match event.kind {
            EventKind::Access(_) => {}
            EventKind::Remove(_) => {
                for path in &event.paths {
                    if let Some(name) = kdl_stem(path).map(Username::new)
                        && let Ok(name) = name
                        && let Ok(mut users) = self.users.write()
                    {
                        users.remove(&name);
                    }
                }
            }
            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Any | EventKind::Other => {
                for path in &event.paths {
                    if path.extension().is_some_and(|extension| extension == "kdl") {
                        self.reload(path);
                    }
                }
            }
        }
    }

    fn reload(&self, path: &Path) {
        let Some(stem) = kdl_stem(path) else {
            return;
        };

        let Ok(name) = Username::new(stem) else {
            self.reject(stem, "invalid user name in file name");
            return;
        };

        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if let Ok(mut users) = self.users.write() {
                    users.remove(&name);
                }
                return;
            }
            Err(error) => {
                self.reject(name.as_str(), &error.to_string());
                return;
            }
        };

        match UserSpec::from_kdl(&text) {
            Ok(spec) if spec.name == name => self.insert(spec),
            Ok(spec) => self.reject(
                name.as_str(),
                &format!("user name does not match file name: {}", spec.name),
            ),
            Err(error) => self.reject(name.as_str(), &error.to_string()),
        }
    }

    fn insert(&self, spec: UserSpec) {
        if let Ok(mut users) = self.users.write() {
            users.insert(spec.name.clone(), spec);
        }
    }

    fn reject(&self, name: &str, reason: &str) {
        tracing::warn!(name, reason, "rejected user configuration");
        let _ = self.events.send(StoreEvent::ConfigRejected {
            name: name.to_owned(),
            reason: reason.to_owned(),
        });
    }
}

fn kdl_stem(path: &Path) -> Option<&str> {
    if path.extension().is_some_and(|extension| extension == "kdl") {
        path.file_stem().and_then(|stem| stem.to_str())
    } else {
        None
    }
}

fn next_temp_id() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use keel_userd_core::backend::{IdentityBackendKind, PackageBackendKind};
    use keel_userd_core::ids::Uid;
    use keel_userd_core::spec::{IdentitySpec, PackageRef, PackagesSpec};

    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("userd-{label}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&path).expect("temp dir");
        path
    }

    fn spec() -> UserSpec {
        let mut spec = UserSpec::new(Username::new("s4rch").expect("valid name"));
        spec.identity = IdentitySpec {
            backend: Some(IdentityBackendKind::Homed),
            uid: Some(Uid::new(60_001)),
            real_name: Some("Sergio".into()),
            ..IdentitySpec::default()
        };
        spec.packages = PackagesSpec {
            packages: vec![PackageRef {
                id: "ripgrep".into(),
                backend: PackageBackendKind::NixProfile,
                attr: Some("nixpkgs#ripgrep".into()),
                remote: None,
            }],
        };
        spec
    }

    #[test]
    fn atomic_write_round_trips_with_0600() {
        let dir = temp_dir("write");
        let (store, _events) = UserStore::open(dir.join("users")).expect("store");
        let expected = spec();
        store.write(&expected).expect("write");

        let path = dir.join("users/s4rch.kdl");
        let mode = std::fs::metadata(&path)
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);

        let text = std::fs::read_to_string(&path).expect("read");
        let parsed = UserSpec::from_kdl(&text).expect("parse");
        assert_eq!(parsed, expected);
        assert_eq!(store.snapshot(), vec![expected]);

        std::fs::remove_dir_all(dir).ok();
    }

    #[tokio::test]
    async fn watcher_keeps_last_valid_config() {
        let dir = temp_dir("watch");
        let (store, mut events) = UserStore::open(dir.join("users")).expect("store");
        let store = Arc::new(store);
        store.start_watcher().expect("watcher");

        let expected = spec();
        store.write(&expected).expect("write");

        let path = dir.join("users/s4rch.kdl");
        std::fs::write(&path, "version 99\nuser \"s4rch\" {\n}\n").expect("invalid write");

        let event = tokio::time::timeout(Duration::from_secs(5), events.recv())
            .await
            .expect("timeout waiting for rejection")
            .expect("event");
        match event {
            StoreEvent::ConfigRejected { name, reason } => {
                assert_eq!(name, "s4rch");
                assert!(reason.contains("schema version"), "{reason}");
            }
        }

        assert_eq!(store.snapshot(), vec![expected]);
        drop(store);
        std::fs::remove_dir_all(dir).ok();
    }
}
