//! Daemon de keel-userd (F0: fundaciones).
//!
//! Expone `io.github.HBlanqueto.Keel.Userd1.Manager` y los objetos `Job`,
//! persiste los jobs en redb y mantiene el último KDL válido de cada usuario.

#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]

pub mod authz;
pub mod backends;
pub mod bus;
pub mod config;
pub mod error;
pub mod identity;
pub mod jobs;
pub mod password;
pub mod registry;
pub mod services;
pub mod storage;
pub mod store;

use std::sync::Arc;
use std::time::Duration;

use keel_userd_core::names::{BUS_NAME, ROOT_PATH};
use tokio::signal::unix::{SignalKind, signal};

pub use config::Config;
pub use error::Error;

use crate::bus::Manager;
use crate::jobs::Scheduler;
use crate::jobs::handlers::{ArchiveUserHandler, CreateUserHandler};
use crate::registry::Registry;
use crate::services::Services;
use crate::store::{StoreEvent, UserStore};
use keel_userd_core::job::JobKind;

/// Arranca el daemon hasta que llegue SIGTERM o SIGINT.
pub async fn run(config: Config) -> Result<(), Error> {
    let config = Arc::new(config);

    let (store, mut events) = UserStore::open(config.users_dir())?;
    let store = Arc::new(store);
    store.start_watcher()?;

    let connection = zbus::connection::Builder::system()?.build().await?;

    let scheduler = Scheduler::open(&config).await?;
    scheduler.set_connection(connection.clone())?;

    let registry = Arc::new(Registry::new(scheduler.database())?);
    let services = Arc::new(Services::new(
        Arc::clone(&config),
        Arc::clone(&store),
        registry,
    ));
    services.set_connection(connection.clone())?;

    scheduler
        .register(
            JobKind::CreateUser,
            Arc::new(CreateUserHandler::new(
                Arc::clone(&services),
                Arc::clone(&scheduler),
            )),
        )
        .await?;
    scheduler
        .register(
            JobKind::ArchiveUser,
            Arc::new(ArchiveUserHandler::new(Arc::clone(&services))),
        )
        .await?;

    // Purga periódica de jobs terminales antiguos.
    let scheduler_for_purge = Arc::clone(&scheduler);
    let retention = Duration::from_secs(config.jobs_retention);
    tokio::spawn(async move {
        scheduler_for_purge.purge_old_jobs(retention).await.ok();
        let mut interval = tokio::time::interval(Duration::from_secs(3600));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            scheduler_for_purge.purge_old_jobs(retention).await.ok();
        }
    });

    connection
        .object_server()
        .at(
            ROOT_PATH,
            Manager::new(Arc::clone(&scheduler), Arc::clone(&services)),
        )
        .await?;
    bus::manager::register_jobs(&connection, &scheduler).await?;
    for view in services.views() {
        services.register_user(&view).await?;
    }
    // Pedir el nombre después de registrar los objetos evita que una llamada
    // llegue antes de que existan las interfaces.
    connection.request_name(BUS_NAME).await?;

    let signal_connection = connection.clone();
    tokio::spawn(async move {
        while let Some(event) = events.recv().await {
            let StoreEvent::ConfigRejected { name, reason } = event;
            tracing::warn!(name, reason, "configuration rejected");
            if let Err(error) = bus::manager::reject(&signal_connection, &name, &reason).await {
                tracing::warn!(%error, "could not emit ConfigRejected");
            }
        }
    });

    tracing::info!(bus = BUS_NAME, "keel-userd started");

    let mut terminate = signal(SignalKind::terminate())?;
    tokio::select! {
        _ = terminate.recv() => {}
        result = tokio::signal::ctrl_c() => { result?; }
    }

    tracing::info!("keel-userd stopping");
    Ok(())
}
