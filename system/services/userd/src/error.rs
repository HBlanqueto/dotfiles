use keel_userd_core::ids::{JobId, Username};
use thiserror::Error;

/// Errores internos del daemon.
#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Core(#[from] keel_userd_core::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("journal database error: {0}")]
    RedbDatabase(#[from] redb::DatabaseError),
    #[error("journal transaction error: {0}")]
    RedbTransaction(#[from] redb::TransactionError),
    #[error("journal table error: {0}")]
    RedbTable(#[from] redb::TableError),
    #[error("journal storage error: {0}")]
    RedbStorage(#[from] redb::StorageError),
    #[error("journal commit error: {0}")]
    RedbCommit(#[from] redb::CommitError),
    #[error("journal serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("D-Bus error: {0}")]
    Dbus(#[from] zbus::Error),
    #[error("job {0} not found")]
    JobNotFound(JobId),
    #[error("job {0} is not in the expected state")]
    UnexpectedState(JobId),
    #[error("no free UID in the configured range")]
    UidRangeExhausted,
    #[error("UID {0} is already in use")]
    UidInUse(u32),
    #[error("identity backend error: {0}")]
    Identity(String),
    #[error("password hashing failed: {0}")]
    Password(String),
    #[error("no hay clave de recuperación custodiada para {0}; usa ResetPassword con la clave")]
    RecoveryMissing(Username),
    #[error("recovery key custody error: {0}")]
    Recovery(String),
    #[error("operation not allowed: {0}")]
    Forbidden(String),
    #[error("user is busy: {0}")]
    Busy(String),
    #[error("job {0} cannot be cancelled in its current state")]
    JobNotCancelable(JobId),
    #[error("lock poisoned: {0}")]
    LockPoisoned(String),
    #[error("config error: {0}")]
    Config(String),
}
