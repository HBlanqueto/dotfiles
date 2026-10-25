use std::path::PathBuf;

use keel_userd::config::Config;
#[cfg(feature = "test-hooks")]
use keel_userd::jobs::Journal;
use keel_userd::{Error, run};
#[cfg(feature = "test-hooks")]
use keel_userd_core::ids::JobId;
#[cfg(feature = "test-hooks")]
use keel_userd_core::ids::Username;
#[cfg(feature = "test-hooks")]
use keel_userd_core::job::{JobKind, JobRecord, JobState};
#[cfg(feature = "test-hooks")]
use keel_userd_core::names::ROOT_PATH;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;

const DEFAULT_CONFIG: &str = "/etc/keel/userd.json";

fn main() -> Result<(), Error> {
    init_tracing();

    let arguments: Vec<String> = std::env::args().skip(1).collect();
    #[cfg(feature = "test-hooks")]
    if arguments
        .first()
        .is_some_and(|argument| argument == "seed-job")
    {
        return seed_job(&arguments[1..]);
    }

    let config = load_config(&arguments)?;
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(run(config))
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let result = match tracing_journald::layer() {
        Ok(journald) => tracing_subscriber::registry()
            .with(filter)
            .with(journald)
            .try_init(),
        Err(_) => tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer())
            .try_init(),
    };
    if result.is_err() {
        eprintln!("could not install the tracing subscriber");
    }
}

fn load_config(arguments: &[String]) -> Result<Config, Error> {
    let mut path = PathBuf::from(DEFAULT_CONFIG);
    let mut state_dir = None;

    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--config" => path = value(arguments, &mut index, "--config")?.into(),
            "--state-dir" => state_dir = Some(value(arguments, &mut index, "--state-dir")?.into()),
            other => return Err(Error::Config(format!("unknown argument `{other}`"))),
        }
        index += 1;
    }

    let mut config = Config::load(&path)?;
    if let Some(state_dir) = state_dir {
        config.state_dir = state_dir;
    }
    Ok(config)
}

fn value<'a>(arguments: &'a [String], index: &mut usize, flag: &str) -> Result<&'a str, Error> {
    *index += 1;
    arguments
        .get(*index)
        .map(String::as_str)
        .ok_or_else(|| Error::Config(format!("{flag} requires a value")))
}

/// Engancha un job de prueba en el journal. Solo se compila con la feature
/// `test-hooks`, que usa el paquete del nixosTest; el binario instalado no lo
/// incluye. Desaparece cuando `Manager.CreateUser` cree jobs de verdad.
#[cfg(feature = "test-hooks")]
fn seed_job(arguments: &[String]) -> Result<(), Error> {
    let mut path = PathBuf::from(DEFAULT_CONFIG);
    let mut state_dir = None;
    let mut user = None;
    let mut running = false;

    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--config" => path = value(arguments, &mut index, "--config")?.into(),
            "--state-dir" => state_dir = Some(value(arguments, &mut index, "--state-dir")?.into()),
            "--user" => user = Some(value(arguments, &mut index, "--user")?.to_owned()),
            "--running" => running = true,
            other => return Err(Error::Config(format!("unknown argument `{other}`"))),
        }
        index += 1;
    }

    let mut config = Config::load(&path)?;
    if let Some(state_dir) = state_dir {
        config.state_dir = state_dir;
    }

    let user = user.ok_or_else(|| Error::Config("--user is required".into()))?;
    let user = Username::new(user).map_err(keel_userd_core::Error::from)?;
    std::fs::create_dir_all(&config.state_dir)?;

    let journal = Journal::open(&config.journal_path())?;
    let id = journal
        .all()?
        .iter()
        .map(|record| record.id.get())
        .max()
        .unwrap_or(0)
        .saturating_add(1);

    let mut record = JobRecord::new(
        JobId::new(id),
        JobKind::ModifyUser,
        format!("{ROOT_PATH}/User/{user}"),
        Some(user),
    );
    if running {
        record.set_state(JobState::Running { progress: None });
    }
    journal.put(&record)?;

    println!("{id}");
    Ok(())
}
