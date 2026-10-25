use std::sync::Arc;

use keel_userd_core::ids::JobId;
use keel_userd_core::job::JobRecord;
use keel_userd_core::names::PolkitAction;
use zbus::interface;
use zbus::message::Header;
use zbus::object_server::SignalEmitter;

use crate::authz;
use crate::bus::DbusError;
use crate::jobs::Scheduler;

/// `io.github.HBlanqueto.Keel.Userd1.Job` en `/…/Job/<id>`.
pub struct Job {
    scheduler: Arc<Scheduler>,
    id: JobId,
}

impl Job {
    #[must_use]
    pub fn new(scheduler: Arc<Scheduler>, id: JobId) -> Self {
        Self { scheduler, id }
    }

    fn record(&self) -> zbus::fdo::Result<Option<JobRecord>> {
        self.scheduler
            .record(self.id)
            .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))
    }
}

#[interface(name = "io.github.HBlanqueto.Keel.Userd1.Job")]
impl Job {
    #[zbus(property)]
    fn kind(&self) -> zbus::fdo::Result<String> {
        Ok(self
            .record()?
            .map_or_else(String::new, |record| record.kind.as_str().to_owned()))
    }

    #[zbus(property)]
    fn target(&self) -> zbus::fdo::Result<String> {
        Ok(self
            .record()?
            .map_or_else(String::new, |record| record.target))
    }

    #[zbus(property)]
    fn state(&self) -> zbus::fdo::Result<String> {
        Ok(self
            .record()?
            .map_or_else(String::new, |record| record.state.as_str().to_owned()))
    }

    #[zbus(property)]
    fn block_reason(&self) -> zbus::fdo::Result<String> {
        Ok(self
            .record()?
            .and_then(|record| {
                record
                    .state
                    .block_reason()
                    .map(|reason| reason.as_str().to_owned())
            })
            .unwrap_or_default())
    }

    #[zbus(property)]
    fn progress(&self) -> zbus::fdo::Result<u32> {
        Ok(self
            .record()?
            .and_then(|record| record.state.progress())
            .map_or(u32::MAX, |percent| u32::from(percent.get())))
    }

    #[zbus(property)]
    fn error(&self) -> zbus::fdo::Result<String> {
        Ok(self
            .record()?
            .and_then(|record| record.state.error_message().map(ToOwned::to_owned))
            .unwrap_or_default())
    }

    async fn take_recovery_key(
        &self,
        #[zbus(header)] header: Header<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> Result<String, DbusError> {
        let connection = emitter.connection();
        let creator = self
            .scheduler
            .creator(self.id)
            .map_err(|error| DbusError::Failed(error.to_string()))?
            .ok_or_else(|| DbusError::NotFound("recovery key is not available".into()))?;
        let caller = authz::caller_uid(connection, &header)
            .await
            .map_err(|error| DbusError::Failed(error.to_string()))?;
        if caller != creator {
            return Err(DbusError::PermissionDenied(
                "only the job creator can take the recovery key".into(),
            ));
        }
        match authz::check(connection, &header, PolkitAction::UsersManage).await {
            Ok(true) => {}
            Ok(false) => {
                return Err(DbusError::PermissionDenied(
                    "not authorized to take the recovery key".into(),
                ));
            }
            Err(error) => return Err(DbusError::Failed(error.to_string())),
        }
        self.scheduler
            .take_result(self.id)
            .map_err(|error| DbusError::Failed(error.to_string()))?
            .map(|key| key.to_string())
            .ok_or_else(|| DbusError::NotFound("recovery key was already taken".into()))
    }

    async fn cancel(
        &self,
        #[zbus(header)] header: Header<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> Result<(), DbusError> {
        match authz::check(emitter.connection(), &header, PolkitAction::UsersManage).await {
            Ok(true) => {}
            Ok(false) => {
                return Err(DbusError::PermissionDenied(
                    "not authorized to manage users".into(),
                ));
            }
            Err(error) => return Err(DbusError::Failed(error.to_string())),
        }

        let record = self
            .scheduler
            .cancel(self.id)
            .await
            .map_err(|error| match error {
                crate::Error::JobNotFound(_) => DbusError::NotFound(self.id.to_string()),
                crate::Error::JobNotCancelable(_) => {
                    DbusError::Conflict("job is already running or finished".into())
                }
                other => DbusError::Failed(other.to_string()),
            })?;

        Self::finished(&emitter, record.state.as_str())
            .await
            .map_err(DbusError::Zbus)?;
        Ok(())
    }

    #[zbus(signal)]
    pub async fn finished(emitter: &SignalEmitter<'_>, state: &str) -> zbus::Result<()>;
}
