use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::Error;
use crate::backend::BackendKind;
use crate::ids::{JobId, Username};

/// Porcentaje de progreso entre 0 y 100.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct Percent(u8);

impl Percent {
    #[must_use]
    pub const fn new(value: u8) -> Option<Self> {
        if value <= 100 {
            Some(Self(value))
        } else {
            None
        }
    }

    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for Percent {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::new(value).ok_or_else(|| Error::InvalidConfig(format!("invalid percent {value}")))
    }
}

impl From<Percent> for u8 {
    fn from(value: Percent) -> Self {
        value.0
    }
}

/// Estado de un job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Queued,
    Running { progress: Option<Percent> },
    Blocked(BlockReason),
    Succeeded,
    Failed(JobFailure),
    Cancelled,
}

impl JobState {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running { .. } => "running",
            Self::Blocked(_) => "blocked",
            Self::Succeeded => "succeeded",
            Self::Failed(_) => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed(_) | Self::Cancelled)
    }

    #[must_use]
    pub const fn block_reason(&self) -> Option<&BlockReason> {
        match self {
            Self::Blocked(reason) => Some(reason),
            _ => None,
        }
    }

    #[must_use]
    pub const fn progress(&self) -> Option<Percent> {
        match self {
            Self::Running { progress } => *progress,
            _ => None,
        }
    }

    #[must_use]
    pub fn error_message(&self) -> Option<&str> {
        match self {
            Self::Failed(failure) => Some(&failure.message),
            _ => None,
        }
    }
}

/// Motivo por el que un job no puede avanzar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BlockReason {
    HomeLocked,
    UserSessionUnavailable,
    BackendUnavailable(BackendKind),
    NetworkUnavailable,
    AwaitingAuth,
}

impl BlockReason {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::HomeLocked => "home-locked",
            Self::UserSessionUnavailable => "user-session-unavailable",
            Self::BackendUnavailable(_) => "backend-unavailable",
            Self::NetworkUnavailable => "network-unavailable",
            Self::AwaitingAuth => "awaiting-auth",
        }
    }
}

/// Fallo terminal de un job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobFailure {
    pub message: String,
}

impl JobFailure {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Operación que ejecuta un job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum JobKind {
    CreateUser,
    ModifyUser,
    ArchiveUser,
    ApplyHome,
    RollbackHome,
    InstallPackage,
    RemovePackage,
    EnableCapability,
    DisableCapability,
}

impl JobKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CreateUser => "create-user",
            Self::ModifyUser => "modify-user",
            Self::ArchiveUser => "archive-user",
            Self::ApplyHome => "apply-home",
            Self::RollbackHome => "rollback-home",
            Self::InstallPackage => "install-package",
            Self::RemovePackage => "remove-package",
            Self::EnableCapability => "enable-capability",
            Self::DisableCapability => "disable-capability",
        }
    }
}

/// Job tal y como se persiste en el journal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobRecord {
    pub id: JobId,
    pub kind: JobKind,
    /// Ruta D-Bus del objeto afectado.
    pub target: String,
    /// Usuario cuya cola serializa el job; `None` para trabajos globales.
    pub user: Option<Username>,
    pub state: JobState,
    /// Datos privados de la operación, en JSON. Nunca contiene contraseñas en
    /// claro.
    #[serde(default)]
    pub payload: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

impl JobRecord {
    #[must_use]
    pub fn new(
        id: JobId,
        kind: JobKind,
        target: impl Into<String>,
        user: Option<Username>,
    ) -> Self {
        let now = now_unix();
        Self {
            id,
            kind,
            target: target.into(),
            user,
            state: JobState::Queued,
            payload: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[must_use]
    pub fn with_payload(mut self, payload: impl Into<String>) -> Self {
        self.payload = Some(payload.into());
        self
    }

    /// Tras un reinicio, todo job no terminal vuelve a la cola. Los `Blocked`
    /// se reevalúan al ejecutarse (en F1 cada handler vuelve a bloquear si la
    /// condición persiste).
    pub fn resume_after_restart(&mut self) -> bool {
        match self.state {
            JobState::Running { .. } | JobState::Blocked(_) => {
                self.set_state(JobState::Queued);
                true
            }
            JobState::Queued | JobState::Succeeded | JobState::Failed(_) | JobState::Cancelled => {
                false
            }
        }
    }

    pub fn set_state(&mut self, state: JobState) {
        self.state = state;
        self.updated_at = now_unix();
    }

    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        self.state.is_terminal()
    }
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(state: JobState) -> JobRecord {
        let mut record = JobRecord::new(
            JobId::new(1),
            JobKind::ApplyHome,
            "/io/github/HBlanqueto/Keel/Userd1/User/60001",
            Some(Username::new("s4rch").expect("valid name")),
        );
        record.set_state(state);
        record
    }

    #[test]
    fn running_and_blocked_requeue_on_restart() {
        let mut running = record(JobState::Running {
            progress: Percent::new(40),
        });
        assert!(running.resume_after_restart());
        assert_eq!(running.state, JobState::Queued);

        let mut blocked = record(JobState::Blocked(BlockReason::HomeLocked));
        assert!(blocked.resume_after_restart());
        assert_eq!(blocked.state, JobState::Queued);

        let mut done = record(JobState::Succeeded);
        assert!(!done.resume_after_restart());
        assert_eq!(done.state, JobState::Succeeded);
    }

    #[test]
    fn percent_is_bounded() {
        assert_eq!(Percent::new(100).map(Percent::get), Some(100));
        assert_eq!(Percent::new(101), None);
        assert!(serde_json::from_str::<Percent>("101").is_err());
        assert_eq!(
            serde_json::from_str::<Percent>("42").expect("valid percent"),
            Percent::new(42).expect("valid percent")
        );
    }

    #[test]
    fn record_round_trips_through_json() {
        let original = record(JobState::Blocked(BlockReason::BackendUnavailable(
            BackendKind::Identity(crate::backend::IdentityBackendKind::Userdb),
        )));
        let json = serde_json::to_string(&original).expect("serializable");
        let decoded: JobRecord = serde_json::from_str(&json).expect("deserializable");
        assert_eq!(decoded, original);
    }

    #[test]
    fn dbus_strings_are_stable() {
        let state = JobState::Blocked(BlockReason::BackendUnavailable(BackendKind::Package(
            crate::backend::PackageBackendKind::Flatpak,
        )));
        assert_eq!(state.as_str(), "blocked");
        assert_eq!(
            state.block_reason().map(BlockReason::as_str),
            Some("backend-unavailable")
        );
        assert_eq!(JobKind::CreateUser.as_str(), "create-user");
    }
}
