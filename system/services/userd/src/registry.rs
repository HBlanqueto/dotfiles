use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use keel_userd_core::ids::Uid;
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};

use crate::Error;
use crate::config::UidRange;

const ASSIGNED: TableDefinition<&str, u32> = TableDefinition::new("assignments");
const USED: TableDefinition<u32, &str> = TableDefinition::new("uids");

/// Registro persistente de UIDs. Un UID liberado queda como tombstone y no se
/// vuelve a asignar.
pub struct Registry {
    database: Arc<Database>,
    lock: Mutex<()>,
}

impl Registry {
    pub fn new(database: Arc<Database>) -> Result<Self, Error> {
        let transaction = database.begin_write()?;
        {
            transaction.open_table(ASSIGNED)?;
            transaction.open_table(USED)?;
        }
        transaction.commit()?;
        Ok(Self {
            database,
            lock: Mutex::new(()),
        })
    }

    /// Asigna el primer UID libre del rango. Si el nombre ya tiene uno, lo
    /// devuelve (idempotente).
    pub fn allocate(&self, name: &str, range: UidRange) -> Result<Uid, Error> {
        let _guard = self
            .lock
            .lock()
            .map_err(|_| Error::LockPoisoned("registry".into()))?;
        let transaction = self.database.begin_write()?;
        let uid;
        {
            let mut assigned = transaction.open_table(ASSIGNED)?;
            if let Some(existing) = assigned.get(name)? {
                return Ok(Uid::new(existing.value()));
            }
            let used = transaction.open_table(USED)?;
            let system = system_users()?;
            let mut free = None;
            for candidate in range.from..=range.to {
                if used.get(candidate)?.is_none() && !system.contains_key(&candidate) {
                    free = Some(candidate);
                    break;
                }
            }
            uid = free.ok_or(Error::UidRangeExhausted)?;
            drop(used);
            transaction.open_table(USED)?.insert(uid, name)?;
            assigned.insert(name, uid)?;
        }
        transaction.commit()?;
        Ok(Uid::new(uid))
    }

    /// Reserva un UID concreto (importación). Se acepta fuera del rango si no
    /// está usado.
    pub fn reserve(&self, name: &str, uid: u32) -> Result<(), Error> {
        let _guard = self
            .lock
            .lock()
            .map_err(|_| Error::LockPoisoned("registry".into()))?;
        let transaction = self.database.begin_write()?;
        {
            let mut assigned = transaction.open_table(ASSIGNED)?;
            if let Some(existing) = assigned.get(name)? {
                return if existing.value() == uid {
                    Ok(())
                } else {
                    Err(Error::UidInUse(existing.value()))
                };
            }
            let mut used = transaction.open_table(USED)?;
            if used.get(uid)?.is_some() {
                return Err(Error::UidInUse(uid));
            }
            if let Some(existing) = system_users()?.get(&uid)
                && existing != name
            {
                return Err(Error::UidInUse(uid));
            }
            used.insert(uid, name)?;
            assigned.insert(name, uid)?;
        }
        transaction.commit()?;
        Ok(())
    }

    /// Libera el UID de un usuario y deja un tombstone.
    pub fn release(&self, name: &str) -> Result<Option<Uid>, Error> {
        let _guard = self
            .lock
            .lock()
            .map_err(|_| Error::LockPoisoned("registry".into()))?;
        let transaction = self.database.begin_write()?;
        let released;
        {
            let mut assigned = transaction.open_table(ASSIGNED)?;
            let Some(uid) = assigned.get(name)?.map(|value| value.value()) else {
                return Ok(None);
            };
            assigned.remove(name)?;
            transaction.open_table(USED)?.insert(uid, "")?;
            released = Uid::new(uid);
        }
        transaction.commit()?;
        Ok(Some(released))
    }

    pub fn uid_for(&self, name: &str) -> Result<Option<Uid>, Error> {
        let transaction = self.database.begin_read()?;
        let table = transaction.open_table(ASSIGNED)?;
        Ok(table.get(name)?.map(|value| Uid::new(value.value())))
    }

    pub fn name_for(&self, uid: Uid) -> Result<Option<String>, Error> {
        let transaction = self.database.begin_read()?;
        let table = transaction.open_table(USED)?;
        Ok(table
            .get(uid.get())?
            .map(|value| value.value().to_owned())
            .filter(|name| !name.is_empty()))
    }
}

fn system_users() -> Result<HashMap<u32, String>, Error> {
    let text = std::fs::read_to_string("/etc/passwd")?;
    let mut users = HashMap::new();
    for line in text.lines() {
        let mut fields = line.split(':');
        let (Some(name), Some(_), Some(uid)) = (fields.next(), fields.next(), fields.next()) else {
            continue;
        };
        if let Ok(uid) = uid.parse() {
            users.insert(uid, name.to_owned());
        }
    }
    Ok(users)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "userd-registry-{label}-{}-{nanos}",
            std::process::id()
        ))
    }

    #[test]
    fn allocates_never_reuses_and_reserves_imports() {
        let dir = temp_dir("alloc");
        std::fs::create_dir_all(&dir).expect("dir");
        let database = crate::storage::open(&dir.join("state.redb")).expect("db");
        let registry = Registry::new(database).expect("registry");
        let range = UidRange {
            from: 40_000,
            to: 40_002,
        };

        let first = registry.allocate("alice", range).expect("alice");
        assert_eq!(first.get(), 40_000);
        assert_eq!(registry.allocate("alice", range).expect("again"), first);
        let second = registry.allocate("bob", range).expect("bob");
        assert_eq!(second.get(), 40_001);

        registry.release("alice").expect("release");
        let third = registry.allocate("carol", range).expect("carol");
        assert_eq!(third.get(), 40_002);
        assert_eq!(registry.uid_for("alice").expect("lookup"), None);

        registry
            .reserve("imported", 700)
            .expect("import outside range");
        assert_eq!(
            registry.uid_for("imported").expect("lookup"),
            Some(Uid::new(700))
        );

        std::fs::remove_dir_all(dir).ok();
    }
}
