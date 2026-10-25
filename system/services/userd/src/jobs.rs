use std::collections::HashMap;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use keel_userd_core::ids::JobId;
use keel_userd_core::job::{JobFailure, JobKind, JobRecord, JobState};
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use tokio::sync::{Semaphore, mpsc};
use zbus::Connection;
use zeroize::Zeroizing;

use crate::Error;
use crate::bus::Job;
use crate::config::Config;

pub mod handlers;

const JOBS: TableDefinition<u64, &[u8]> = TableDefinition::new("jobs");

/// Journal de jobs sobre redb.
pub struct Journal {
    database: Arc<Database>,
}

impl Journal {
    pub fn new(database: Arc<Database>) -> Result<Self, Error> {
        let transaction = database.begin_write()?;
        {
            transaction.open_table(JOBS)?;
        }
        transaction.commit()?;
        Ok(Self { database })
    }

    pub fn open(path: &Path) -> Result<Self, Error> {
        Self::new(crate::storage::open(path)?)
    }

    pub fn put(&self, record: &JobRecord) -> Result<(), Error> {
        let bytes = serde_json::to_vec(record)?;
        let transaction = self.database.begin_write()?;
        {
            let mut table = transaction.open_table(JOBS)?;
            table.insert(record.id.get(), bytes.as_slice())?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn all(&self) -> Result<Vec<JobRecord>, Error> {
        let transaction = self.database.begin_read()?;
        let table = transaction.open_table(JOBS)?;
        let mut records = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            records.push(serde_json::from_slice(value.value())?);
        }
        Ok(records)
    }

    pub fn delete(&self, id: JobId) -> Result<(), Error> {
        let transaction = self.database.begin_write()?;
        {
            let mut table = transaction.open_table(JOBS)?;
            table.remove(id.get())?;
        }
        transaction.commit()?;
        Ok(())
    }
}

/// Ejecuta el trabajo de un job.
pub trait Handler: Send + Sync {
    fn run<'a>(
        &'a self,
        job: &'a JobRecord,
    ) -> Pin<Box<dyn Future<Output = Result<(), JobFailure>> + Send + 'a>>;
}

/// Serializa los jobs por usuario y limita los builds concurrentes.
pub struct Scheduler {
    journal: Journal,
    database: Arc<Database>,
    jobs: RwLock<HashMap<JobId, JobRecord>>,
    creators: RwLock<HashMap<JobId, u32>>,
    results: RwLock<HashMap<JobId, Zeroizing<String>>>,
    actors: tokio::sync::Mutex<HashMap<String, mpsc::Sender<JobId>>>,
    semaphore: Arc<Semaphore>,
    handlers: RwLock<HashMap<JobKind, Arc<dyn Handler>>>,
    transitions: tokio::sync::Mutex<()>,
    next_id: AtomicU64,
    connection: RwLock<Option<Connection>>,
}

impl Scheduler {
    /// Abre el journal, reanuda los jobs no terminales y despacha los `Queued`.
    pub async fn open(config: &Config) -> Result<Arc<Self>, Error> {
        std::fs::create_dir_all(&config.state_dir)?;
        let database = crate::storage::open(&config.journal_path())?;
        let journal = Journal::new(Arc::clone(&database))?;
        let mut records = journal.all()?;

        let mut max_id = 0;
        for record in &mut records {
            max_id = max_id.max(record.id.get());
            if record.resume_after_restart() {
                journal.put(record)?;
                tracing::info!(id = record.id.get(), "requeued job after restart");
            }
        }

        let queued: Vec<JobId> = records
            .iter()
            .filter(|record| matches!(record.state, JobState::Queued))
            .map(|record| record.id)
            .collect();

        let scheduler = Arc::new(Self {
            journal,
            database,
            jobs: RwLock::new(records.into_iter().map(|job| (job.id, job)).collect()),
            creators: RwLock::new(HashMap::new()),
            results: RwLock::new(HashMap::new()),
            actors: tokio::sync::Mutex::new(HashMap::new()),
            semaphore: Arc::new(Semaphore::new(config.max_builds.max(1))),
            handlers: RwLock::new(HashMap::new()),
            transitions: tokio::sync::Mutex::new(()),
            next_id: AtomicU64::new(max_id.saturating_add(1)),
            connection: RwLock::new(None),
        });

        for id in queued {
            scheduler.dispatch(id).await?;
        }

        Ok(scheduler)
    }

    #[must_use]
    pub fn database(&self) -> Arc<Database> {
        Arc::clone(&self.database)
    }

    /// Conecta el scheduler con el bus para registrar los objetos `Job` nuevos.
    pub fn set_connection(&self, connection: Connection) -> Result<(), Error> {
        *write_lock(&self.connection, "connection")? = Some(connection);
        Ok(())
    }

    /// Persiste antes de encolar y registra el objeto D-Bus en el mismo momento.
    pub async fn enqueue(self: &Arc<Self>, record: JobRecord) -> Result<(), Error> {
        self.journal.put(&record)?;
        self.next_id
            .fetch_max(record.id.get().saturating_add(1), Ordering::Relaxed);
        write_lock(&self.jobs, "jobs")?.insert(record.id, record.clone());
        if let Err(error) = self.register_object(record.id).await {
            tracing::warn!(
                id = record.id.get(),
                %error,
                "could not register the job object; dispatching anyway"
            );
        }
        self.dispatch(record.id).await
    }

    /// Registra un handler y reintenta los jobs `Queued` de su mismo tipo.
    pub async fn register(
        self: &Arc<Self>,
        kind: JobKind,
        handler: Arc<dyn Handler>,
    ) -> Result<(), Error> {
        write_lock(&self.handlers, "handlers")?.insert(kind, handler);

        let queued: Vec<JobId> = self
            .records()?
            .into_iter()
            .filter(|record| record.kind == kind && matches!(record.state, JobState::Queued))
            .map(|record| record.id)
            .collect();

        for id in queued {
            self.dispatch(id).await?;
        }
        Ok(())
    }

    #[must_use]
    pub fn next_id(&self) -> JobId {
        JobId::new(self.next_id.fetch_add(1, Ordering::Relaxed))
    }

    pub fn records(&self) -> Result<Vec<JobRecord>, Error> {
        let jobs = read_lock(&self.jobs, "jobs")?;
        let mut records: Vec<_> = jobs.values().cloned().collect();
        records.sort_by_key(|record| record.id);
        Ok(records)
    }

    pub fn record(&self, id: JobId) -> Result<Option<JobRecord>, Error> {
        Ok(read_lock(&self.jobs, "jobs")?.get(&id).cloned())
    }

    /// Guarda el UID que creó el job; solo vive en memoria.
    pub fn set_creator(&self, id: JobId, uid: u32) -> Result<(), Error> {
        write_lock(&self.creators, "creators")?.insert(id, uid);
        Ok(())
    }

    pub fn creator(&self, id: JobId) -> Result<Option<u32>, Error> {
        Ok(read_lock(&self.creators, "creators")?.get(&id).copied())
    }

    /// Guarda un resultado efímero del job. No se persiste en el journal.
    pub fn set_result(&self, id: JobId, result: Zeroizing<String>) -> Result<(), Error> {
        if self.record(id)?.is_none() {
            return Err(Error::JobNotFound(id));
        }
        write_lock(&self.results, "results")?.insert(id, result);
        Ok(())
    }

    /// Extrae el resultado y lo borra de memoria.
    pub fn take_result(&self, id: JobId) -> Result<Option<Zeroizing<String>>, Error> {
        Ok(write_lock(&self.results, "results")?.remove(&id))
    }

    /// Borra los jobs en estado terminal más antiguos que `retention`.
    pub async fn purge_old_jobs(self: &Arc<Self>, retention: Duration) -> Result<(), Error> {
        let scheduler = Arc::clone(self);
        tokio::task::spawn_blocking(move || {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let cutoff = now.saturating_sub(retention.as_secs());
            let ids: Vec<JobId> = scheduler
                .records()?
                .into_iter()
                .filter(|record| record.is_terminal() && record.updated_at < cutoff)
                .map(|record| record.id)
                .collect();
            for id in ids {
                scheduler.journal.delete(id)?;
                write_lock(&scheduler.jobs, "jobs")?.remove(&id);
                write_lock(&scheduler.creators, "creators")?.remove(&id);
                write_lock(&scheduler.results, "results")?.remove(&id);
                tracing::debug!(id = id.get(), "purged old terminal job");
            }
            Ok::<(), Error>(())
        })
        .await
        .map_err(|error| Error::Identity(error.to_string()))?
    }

    /// Si el payload del job contiene `password_hash`, lo sustituye por `null`
    /// en memoria y en el journal.
    pub async fn redact_secrets(self: &Arc<Self>, id: JobId) -> Result<(), Error> {
        let scheduler = Arc::clone(self);
        tokio::task::spawn_blocking(move || {
            let mut record = scheduler
                .record(id)?
                .ok_or_else(|| Error::JobNotFound(id))?;
            if let Some(payload) = record.payload.as_mut()
                && let Ok(mut value) = serde_json::from_str::<serde_json::Value>(payload)
                && value.get("password_hash").is_some()
            {
                value["password_hash"] = serde_json::Value::Null;
                *payload = serde_json::to_string(&value)?;
                scheduler.journal.put(&record)?;
                write_lock(&scheduler.jobs, "jobs")?.insert(id, record);
            }
            Ok::<(), Error>(())
        })
        .await
        .map_err(|error| Error::Identity(error.to_string()))?
    }

    /// Cambia el estado solo si el actual cumple `expected`.
    ///
    /// El mutex de transiciones serializa las escrituras; las lecturas usan el
    /// `RwLock` y no esperan a que termine el `journal.put`.
    pub async fn transition(
        &self,
        id: JobId,
        expected: impl Fn(&JobState) -> bool + Send,
        next: JobState,
    ) -> Result<JobRecord, Error> {
        let _guard = self.transitions.lock().await;

        let mut record = {
            let jobs = read_lock(&self.jobs, "jobs")?;
            jobs.get(&id).cloned().ok_or(Error::JobNotFound(id))?
        };

        if !expected(&record.state) {
            return Err(Error::UnexpectedState(id));
        }

        record.set_state(next);
        self.journal.put(&record)?;
        write_lock(&self.jobs, "jobs")?.insert(id, record.clone());
        Ok(record)
    }

    /// Solo se cancelan jobs que aún no se están ejecutando.
    pub async fn cancel(&self, id: JobId) -> Result<JobRecord, Error> {
        self.transition(
            id,
            |state| matches!(state, JobState::Queued | JobState::Blocked(_)),
            JobState::Cancelled,
        )
        .await
        .map_err(|error| match error {
            Error::UnexpectedState(_) => Error::JobNotCancelable(id),
            other => other,
        })
    }

    fn handler(&self, kind: JobKind) -> Result<Option<Arc<dyn Handler>>, Error> {
        Ok(read_lock(&self.handlers, "handlers")?.get(&kind).cloned())
    }

    async fn register_object(self: &Arc<Self>, id: JobId) -> Result<(), Error> {
        let connection = read_lock(&self.connection, "connection")?.clone();
        let Some(connection) = connection else {
            return Ok(());
        };
        connection
            .object_server()
            .at(
                keel_userd_core::names::job_path(id),
                Job::new(Arc::clone(self), id),
            )
            .await?;
        Ok(())
    }

    async fn dispatch(self: &Arc<Self>, id: JobId) -> Result<(), Error> {
        let Some(record) = self.record(id)? else {
            return Err(Error::JobNotFound(id));
        };
        if !matches!(record.state, JobState::Queued) {
            return Ok(());
        }

        let key = record
            .user
            .as_ref()
            .map_or_else(|| "_global".to_owned(), ToString::to_string);

        let mut actors = self.actors.lock().await;
        let sender = if let Some(sender) = actors.get(&key) {
            sender.clone()
        } else {
            let (sender, receiver) = mpsc::channel(64);
            actors.insert(key, sender.clone());
            tokio::spawn(actor(Arc::clone(self), receiver));
            sender
        };
        drop(actors);

        sender
            .send(id)
            .await
            .map_err(|_| Error::Config("job actor stopped".into()))
    }
}

async fn actor(scheduler: Arc<Scheduler>, mut receiver: mpsc::Receiver<JobId>) {
    while let Some(id) = receiver.recv().await {
        let record = match scheduler.record(id) {
            Ok(Some(record)) => record,
            Ok(None) => continue,
            Err(error) => {
                tracing::warn!(%error, id = id.get(), "could not read job");
                continue;
            }
        };
        if !matches!(record.state, JobState::Queued) {
            continue;
        }

        let handler = match scheduler.handler(record.kind) {
            Ok(Some(handler)) => handler,
            Ok(None) => {
                tracing::debug!(
                    id = id.get(),
                    kind = record.kind.as_str(),
                    "no handler registered; job stays queued"
                );
                continue;
            }
            Err(error) => {
                tracing::warn!(%error, id = id.get(), "could not read handlers");
                continue;
            }
        };

        let Ok(permit) = scheduler.semaphore.clone().acquire_owned().await else {
            break;
        };

        let running = match scheduler
            .transition(
                id,
                |state| matches!(state, JobState::Queued),
                JobState::Running { progress: None },
            )
            .await
        {
            Ok(record) => record,
            Err(Error::UnexpectedState(_)) => {
                tracing::debug!(id = id.get(), "job is no longer queued; skipping");
                continue;
            }
            Err(error) => {
                tracing::warn!(%error, id = id.get(), "could not start job");
                continue;
            }
        };

        let state = match handler.run(&running).await {
            Ok(()) => JobState::Succeeded,
            Err(failure) => JobState::Failed(failure),
        };
        let terminal = state.is_terminal();

        if let Err(error) = scheduler
            .transition(
                id,
                |current| matches!(current, JobState::Running { .. }),
                state,
            )
            .await
        {
            tracing::warn!(%error, id = id.get(), "could not finish job");
        }
        if terminal {
            scheduler.redact_secrets(id).await.unwrap_or_else(|error| {
                tracing::warn!(%error, id = id.get(), "could not redact job payload");
            });
        }
        drop(permit);
    }
}

fn read_lock<'a, T>(lock: &'a RwLock<T>, name: &str) -> Result<RwLockReadGuard<'a, T>, Error> {
    lock.read()
        .map_err(|_| Error::LockPoisoned(name.to_owned()))
}

fn write_lock<'a, T>(lock: &'a RwLock<T>, name: &str) -> Result<RwLockWriteGuard<'a, T>, Error> {
    lock.write()
        .map_err(|_| Error::LockPoisoned(name.to_owned()))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use std::sync::atomic::AtomicBool;

    use super::*;
    use keel_userd_core::ids::Username;

    fn temp_config(label: &str) -> Config {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        Config {
            state_dir: std::env::temp_dir()
                .join(format!("userd-jobs-{label}-{}-{nanos}", std::process::id())),
            ..Config::default()
        }
    }

    fn record(id: u64) -> JobRecord {
        record_for(id, "s4rch")
    }

    fn record_for(id: u64, name: &str) -> JobRecord {
        JobRecord::new(
            JobId::new(id),
            JobKind::ModifyUser,
            "/io/github/HBlanqueto/Keel/Userd1/User/60001",
            Some(Username::new(name).expect("valid name")),
        )
    }

    #[tokio::test]
    async fn running_jobs_requeue_after_reopen() {
        let config = temp_config("resume");
        std::fs::create_dir_all(&config.state_dir).expect("state dir");
        let journal = Journal::open(&config.journal_path()).expect("journal");
        let mut job = record(1);
        job.set_state(JobState::Running { progress: None });
        journal.put(&job).expect("persist");
        drop(journal);

        let scheduler = Scheduler::open(&config).await.expect("scheduler");
        let resumed = scheduler.record(JobId::new(1)).expect("lock").expect("job");
        assert_eq!(resumed.state, JobState::Queued);

        std::fs::remove_dir_all(&config.state_dir).ok();
    }

    #[tokio::test]
    async fn actor_runs_registered_handler() {
        struct Done;
        impl Handler for Done {
            fn run<'a>(
                &'a self,
                _job: &'a JobRecord,
            ) -> Pin<Box<dyn Future<Output = Result<(), JobFailure>> + Send + 'a>> {
                Box::pin(async { Ok(()) })
            }
        }

        let config = temp_config("actor");
        let scheduler = Scheduler::open(&config).await.expect("scheduler");
        scheduler
            .register(JobKind::ModifyUser, Arc::new(Done))
            .await
            .expect("register");
        scheduler.enqueue(record(2)).await.expect("enqueue");

        for _ in 0..100 {
            let finished = scheduler
                .record(JobId::new(2))
                .expect("lock")
                .is_some_and(|job| matches!(job.state, JobState::Succeeded));
            if finished {
                std::fs::remove_dir_all(&config.state_dir).ok();
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("job did not finish");
    }

    #[tokio::test]
    async fn cancel_only_queued_or_blocked() {
        let config = temp_config("cancel");
        let scheduler = Scheduler::open(&config).await.expect("scheduler");
        scheduler.enqueue(record(3)).await.expect("enqueue");

        let cancelled = scheduler.cancel(JobId::new(3)).await.expect("cancel");
        assert_eq!(cancelled.state, JobState::Cancelled);
        assert!(matches!(
            scheduler.cancel(JobId::new(3)).await,
            Err(Error::JobNotCancelable(_))
        ));

        std::fs::remove_dir_all(&config.state_dir).ok();
    }

    #[tokio::test]
    async fn cancel_while_waiting_for_semaphore_never_runs_handler() {
        struct Gate {
            started: Arc<tokio::sync::Notify>,
            release: Arc<tokio::sync::Notify>,
            ran_second: Arc<AtomicBool>,
        }

        impl Handler for Gate {
            fn run<'a>(
                &'a self,
                job: &'a JobRecord,
            ) -> Pin<Box<dyn Future<Output = Result<(), JobFailure>> + Send + 'a>> {
                let started = Arc::clone(&self.started);
                let release = Arc::clone(&self.release);
                let ran_second = Arc::clone(&self.ran_second);
                let first = job.id == JobId::new(1);
                Box::pin(async move {
                    if first {
                        started.notify_one();
                        release.notified().await;
                    } else {
                        ran_second.store(true, Ordering::SeqCst);
                    }
                    Ok(())
                })
            }
        }

        let config = Config {
            max_builds: 1,
            ..temp_config("cancel-waiting")
        };
        let scheduler = Scheduler::open(&config).await.expect("scheduler");
        let gate = Arc::new(Gate {
            started: Arc::new(tokio::sync::Notify::new()),
            release: Arc::new(tokio::sync::Notify::new()),
            ran_second: Arc::new(AtomicBool::new(false)),
        });
        scheduler
            .register(JobKind::ModifyUser, gate.clone())
            .await
            .expect("register");

        let started = gate.started.notified();
        scheduler
            .enqueue(record_for(1, "u1"))
            .await
            .expect("enqueue first");
        tokio::time::timeout(Duration::from_secs(5), started)
            .await
            .expect("first job should start");

        scheduler
            .enqueue(record_for(2, "u2"))
            .await
            .expect("enqueue second");
        loop {
            let state = scheduler
                .record(JobId::new(2))
                .expect("lock")
                .expect("job")
                .state;
            if matches!(state, JobState::Queued) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        let cancelled = scheduler.cancel(JobId::new(2)).await.expect("cancel");
        assert_eq!(cancelled.state, JobState::Cancelled);

        gate.release.notify_one();
        for _ in 0..100 {
            let first_done = scheduler
                .record(JobId::new(1))
                .expect("lock")
                .is_some_and(|job| matches!(job.state, JobState::Succeeded));
            if first_done {
                assert!(!gate.ran_second.load(Ordering::SeqCst));
                assert_eq!(
                    scheduler
                        .record(JobId::new(2))
                        .expect("lock")
                        .expect("job")
                        .state,
                    JobState::Cancelled
                );
                std::fs::remove_dir_all(&config.state_dir).ok();
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("first job did not finish");
    }

    #[tokio::test]
    async fn results_are_memory_only_and_purged() {
        struct Done;
        impl Handler for Done {
            fn run<'a>(
                &'a self,
                _job: &'a JobRecord,
            ) -> Pin<Box<dyn Future<Output = Result<(), JobFailure>> + Send + 'a>> {
                Box::pin(async { Ok(()) })
            }
        }

        let config = temp_config("results");
        let scheduler = Scheduler::open(&config).await.expect("scheduler");
        scheduler
            .register(JobKind::ModifyUser, Arc::new(Done))
            .await
            .expect("register");
        scheduler.enqueue(record(4)).await.expect("enqueue");

        for _ in 0..100 {
            let finished = scheduler
                .record(JobId::new(4))
                .expect("lock")
                .is_some_and(|job| matches!(job.state, JobState::Succeeded));
            if finished {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        scheduler
            .set_result(JobId::new(4), Zeroizing::new("secret-key".into()))
            .expect("result");
        assert_eq!(
            scheduler
                .take_result(JobId::new(4))
                .expect("result")
                .as_ref()
                .map(|key| key.as_str()),
            Some("secret-key")
        );
        assert!(
            scheduler
                .take_result(JobId::new(4))
                .expect("result")
                .is_none()
        );
        let journal =
            serde_json::to_string(&scheduler.journal.all().expect("journal")).expect("json");
        assert!(!journal.contains("secret-key"));

        tokio::time::sleep(Duration::from_millis(1100)).await;
        scheduler
            .purge_old_jobs(Duration::ZERO)
            .await
            .expect("purge");
        assert!(scheduler.record(JobId::new(4)).expect("record").is_none());
        assert!(
            scheduler
                .take_result(JobId::new(4))
                .expect("result")
                .is_none()
        );

        std::fs::remove_dir_all(&config.state_dir).ok();
    }

    #[tokio::test]
    async fn global_semaphore_limits_concurrency() {
        struct Track {
            current: std::sync::atomic::AtomicUsize,
            peak: std::sync::atomic::AtomicUsize,
        }
        impl Handler for Track {
            fn run<'a>(
                &'a self,
                _job: &'a JobRecord,
            ) -> Pin<Box<dyn Future<Output = Result<(), JobFailure>> + Send + 'a>> {
                Box::pin(async move {
                    let current = self.current.fetch_add(1, Ordering::SeqCst) + 1;
                    self.peak.fetch_max(current, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(20)).await;
                    self.current.fetch_sub(1, Ordering::SeqCst);
                    Ok(())
                })
            }
        }

        let config = Config {
            max_builds: 1,
            ..temp_config("semaphore")
        };
        let scheduler = Scheduler::open(&config).await.expect("scheduler");
        let track = Arc::new(Track {
            current: std::sync::atomic::AtomicUsize::new(0),
            peak: std::sync::atomic::AtomicUsize::new(0),
        });
        scheduler
            .register(JobKind::ModifyUser, track.clone())
            .await
            .expect("register");

        for (id, name) in [(10, "u10"), (11, "u11"), (12, "u12")] {
            scheduler
                .enqueue(record_for(id, name))
                .await
                .expect("enqueue");
        }
        for _ in 0..100 {
            let done = scheduler
                .records()
                .expect("lock")
                .iter()
                .all(|job| matches!(job.state, JobState::Succeeded));
            if done {
                assert_eq!(track.peak.load(Ordering::SeqCst), 1);
                std::fs::remove_dir_all(&config.state_dir).ok();
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("jobs did not finish");
    }
}
