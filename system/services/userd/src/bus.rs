//! Interfaces D-Bus del daemon y conformidad con los XML versionados.

pub mod job;
pub mod manager;
pub mod user;

pub use job::Job;
pub use manager::Manager;
pub use user::User;

/// Errores propios, bajo `io.github.HBlanqueto.Keel.Userd1.Error`.
///
/// El prefijo del derive debe ser literal (limitación de zbus); el test de
/// conformidad comprueba que coincide con `keel_userd_core::names::ERROR_NAMESPACE`.
#[derive(Debug, zbus::DBusError)]
#[zbus(prefix = "io.github.HBlanqueto.Keel.Userd1.Error")]
pub enum DbusError {
    #[zbus(error)]
    Zbus(zbus::Error),
    NotFound(String),
    PermissionDenied(String),
    InvalidConfig(String),
    BackendUnavailable(String),
    ReadOnly(String),
    Conflict(String),
    Failed(String),
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use keel_userd_core::backend::IdentityBackendKind;
    use keel_userd_core::ids::{JobId, Uid, Username};
    use keel_userd_core::names::{ErrorKind, JOB_INTERFACE, MANAGER_INTERFACE, USER_INTERFACE};
    use zbus::object_server::Interface;

    use super::{DbusError, Job, Manager, User};
    use crate::config::Config;
    use crate::jobs::Scheduler;
    use crate::registry::Registry;
    use crate::services::{Services, UserView};
    use crate::store::UserStore;

    struct Fixture {
        manager: Manager,
        scheduler: Arc<Scheduler>,
        services: Arc<Services>,
        dir: PathBuf,
    }

    async fn fixture() -> Fixture {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("userd-conformance-{}-{nanos}", std::process::id()));
        let config = Arc::new(Config {
            state_dir: dir.join("state"),
            ..Config::default()
        });
        std::fs::create_dir_all(config.users_dir()).expect("users dir");
        let (store, _events) = UserStore::open(config.users_dir()).expect("store");
        let store = Arc::new(store);
        let scheduler = Scheduler::open(&config).await.expect("scheduler");
        let registry = Arc::new(Registry::new(scheduler.database()).expect("registry"));
        let services = Arc::new(Services::new(Arc::clone(&config), store, registry));
        let manager = Manager::new(Arc::clone(&scheduler), Arc::clone(&services));
        Fixture {
            manager,
            scheduler,
            services,
            dir,
        }
    }

    fn rendered(interface: &impl Interface) -> String {
        let mut output = String::new();
        interface.introspect_to_writer(&mut output, 0);
        output
    }

    fn normalized(xml: &str) -> String {
        xml.chars()
            .filter(|character| !character.is_whitespace())
            .collect()
    }

    fn stored(name: &str) -> String {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("interfaces")
            .join(name);
        std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
    }

    #[tokio::test]
    async fn manager_matches_the_xml() {
        let fixture = fixture().await;
        let generated = rendered(&fixture.manager);
        assert!(
            generated.contains(MANAGER_INTERFACE),
            "missing interface name in {generated}"
        );
        assert_eq!(
            normalized(&generated),
            normalized(&stored("io.github.HBlanqueto.Keel.Userd1.Manager.xml"))
        );
        std::fs::remove_dir_all(&fixture.dir).ok();
    }

    #[tokio::test]
    async fn job_matches_the_xml() {
        let fixture = fixture().await;
        let job = Job::new(Arc::clone(&fixture.scheduler), JobId::new(1));
        let generated = rendered(&job);
        assert!(
            generated.contains(JOB_INTERFACE),
            "missing interface name in {generated}"
        );
        assert_eq!(
            normalized(&generated),
            normalized(&stored("io.github.HBlanqueto.Keel.Userd1.Job.xml"))
        );
        std::fs::remove_dir_all(&fixture.dir).ok();
    }

    #[tokio::test]
    async fn user_matches_the_xml() {
        let fixture = fixture().await;
        let view = UserView {
            name: Username::new("s4rch").expect("valid name"),
            uid: Uid::new(30_000),
            backend: IdentityBackendKind::Declared,
            managed: false,
            admin: false,
            real_name: None,
            shell: None,
            home: PathBuf::from("/home/s4rch"),
            state: "active",
        };
        let user = User::new(Arc::clone(&fixture.services), view);
        let generated = rendered(&user);
        assert!(
            generated.contains(USER_INTERFACE),
            "missing interface name in {generated}"
        );
        assert_eq!(
            normalized(&generated),
            normalized(&stored("io.github.HBlanqueto.Keel.Userd1.User.xml"))
        );
        std::fs::remove_dir_all(&fixture.dir).ok();
    }

    #[test]
    fn error_names_use_the_core_namespace() {
        let error = DbusError::InvalidConfig("test".into());
        assert_eq!(
            <DbusError as zbus::DBusError>::name(&error).as_str(),
            ErrorKind::InvalidConfig.name()
        );
    }

    #[tokio::test]
    #[ignore = "escribe la introspección para regenerar interfaces/"]
    async fn dump_interfaces() {
        let fixture = fixture().await;
        let job = Job::new(Arc::clone(&fixture.scheduler), JobId::new(1));
        let view = UserView {
            name: Username::new("s4rch").expect("valid name"),
            uid: Uid::new(30_000),
            backend: IdentityBackendKind::Declared,
            managed: false,
            admin: false,
            real_name: None,
            shell: None,
            home: PathBuf::from("/home/s4rch"),
            state: "active",
        };
        let user = User::new(Arc::clone(&fixture.services), view);
        println!(
            "===MANAGER===\n{}===JOB===\n{}===USER===\n{}",
            rendered(&fixture.manager),
            rendered(&job),
            rendered(&user)
        );
        std::fs::remove_dir_all(&fixture.dir).ok();
    }
}
