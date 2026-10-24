use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::Arc;

use redb::Database;

use crate::Error;

/// Abre la base redb compartida del daemon (jobs y registry).
pub fn open(path: &Path) -> Result<Arc<Database>, Error> {
    let database = Database::create(path)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(Arc::new(database))
}
