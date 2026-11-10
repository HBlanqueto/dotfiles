//! `users-daemon`: system D-Bus service that manages user accounts.
use std::collections::HashMap;
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use color_eyre::eyre::{bail, Context, Result};
use nixcore::paths;
use nixcore::rebuild::{self, Mode, Outcome};
use nixcore::user;
use nixcore::user_data::{self, UserData};
use tokio::sync::Mutex;
use zbus::message::Header;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::OwnedValue;
use zbus::{fdo, interface, proxy, Connection};
use zeroize::Zeroizing;

const BUS_NAME: &str = "io.github.HBlanqueto.System.Users1";
const OBJECT_PATH: &str = "/io/github/HBlanqueto/System/Users1";
const ACTION: &str = "io.github.HBlanqueto.System.users.manage";
const MAX_FULL_NAME: usize = 128;

#[proxy(
    default_service = "org.freedesktop.PolicyKit1",
    default_path = "/org/freedesktop/PolicyKit1/Authority",
    interface = "org.freedesktop.PolicyKit1.Authority"
)]
trait PolicyKit1 {
    fn check_authorization(
        &self,
        subject: (String, HashMap<String, OwnedValue>),
        action_id: &str,
        details: HashMap<String, String>,
        flags: u32,
        cancellation_id: &str,
    ) -> zbus::Result<(bool, bool, HashMap<String, String>)>;
}

#[proxy(
    default_service = "org.freedesktop.DBus",
    default_path = "/org/freedesktop/DBus",
    interface = "org.freedesktop.DBus"
)]
trait DBus {
    fn get_connection_unix_user(&self, name: &str) -> zbus::Result<u32>;
}

struct Inner {
    lock: Mutex<()>,
    next_job: AtomicU32,
}

#[derive(Clone)]
struct Users {
    inner: Arc<Inner>,
}

impl Default for Users {
    fn default() -> Self {
        Self {
            inner: Arc::new(Inner {
                lock: Mutex::new(()),
                next_job: AtomicU32::new(1),
            }),
        }
    }
}

#[interface(name = "io.github.HBlanqueto.System.Users1")]
impl Users {
    async fn list_users(&self) -> Vec<(String, String, bool)> {
        UserData::list()
            .map(|users| {
                users
                    .into_iter()
                    .map(|u| (u.username, u.full_name, u.admin))
                    .collect()
            })
            .unwrap_or_default()
    }

    async fn create_user(
        &self,
        #[zbus(header)] header: Header<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        username: String,
        full_name: String,
        admin: bool,
        password: String,
    ) -> fdo::Result<u32> {
        authorize_header(&header).await?;
        validate_username(&username)?;
        validate_full_name(&full_name)?;
        let password = Zeroizing::new(password);
        let job = self.next_job();
        let inner = self.inner.clone();
        let emitter = emitter.to_owned();
        tokio::spawn(async move {
            let _guard = inner.lock.lock().await;
            let result = create(&username, &full_name, admin, &password, &emitter, job).await;
            finish(&emitter, job, "create", result).await;
        });
        Ok(job)
    }

    async fn delete_user(
        &self,
        #[zbus(header)] header: Header<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        username: String,
        keep_home: bool,
    ) -> fdo::Result<u32> {
        authorize_header(&header).await?;
        validate_username(&username)?;
        let job = self.next_job();
        let inner = self.inner.clone();
        let emitter = emitter.to_owned();
        tokio::spawn(async move {
            let _guard = inner.lock.lock().await;
            let result = delete(&username, keep_home, &emitter, job).await;
            finish(&emitter, job, "delete", result).await;
        });
        Ok(job)
    }

    async fn set_admin(
        &self,
        #[zbus(header)] header: Header<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        username: String,
        admin: bool,
    ) -> fdo::Result<u32> {
        authorize_header(&header).await?;
        validate_username(&username)?;
        let job = self.next_job();
        let inner = self.inner.clone();
        let emitter = emitter.to_owned();
        tokio::spawn(async move {
            let _guard = inner.lock.lock().await;
            let result = set_admin(&username, admin, &emitter, job).await;
            finish(&emitter, job, "set-admin", result).await;
        });
        Ok(job)
    }

    async fn set_full_name(
        &self,
        #[zbus(header)] header: Header<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        username: String,
        full_name: String,
    ) -> fdo::Result<u32> {
        authorize_header(&header).await?;
        validate_username(&username)?;
        validate_full_name(&full_name)?;
        let job = self.next_job();
        let inner = self.inner.clone();
        let emitter = emitter.to_owned();
        tokio::spawn(async move {
            let _guard = inner.lock.lock().await;
            let result = set_full_name(&username, &full_name, &emitter, job).await;
            finish(&emitter, job, "set-full-name", result).await;
        });
        Ok(job)
    }

    async fn set_password(
        &self,
        #[zbus(header)] header: Header<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        username: String,
        password: String,
    ) -> fdo::Result<u32> {
        authorize_header(&header).await?;
        validate_username(&username)?;
        if password.is_empty() {
            return Err(fdo::Error::Failed("la contraseña está vacía".into()));
        }
        let password = Zeroizing::new(password);
        let job = self.next_job();
        let inner = self.inner.clone();
        let emitter = emitter.to_owned();
        tokio::spawn(async move {
            let _guard = inner.lock.lock().await;
            let result = set_password(&username, &password, &emitter, job).await;
            finish(&emitter, job, "set-password", result).await;
        });
        Ok(job)
    }

    async fn delete_prefs_seed(
        &self,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] connection: &Connection,
        username: String,
    ) -> fdo::Result<()> {
        validate_username(&username)?;
        let sender = sender(&header)?;
        let dbus = DBusProxy::new(connection)
            .await
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;
        let uid = dbus
            .get_connection_unix_user(&sender)
            .await
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;
        if uid_of(&username) != Some(uid) {
            authorize(connection, &sender).await?;
        }
        let _ = std::fs::remove_file(user_data::prefs_path_for(&username));
        Ok(())
    }

    #[zbus(signal)]
    async fn progress(
        emitter: &SignalEmitter<'_>,
        job: u32,
        action: &str,
        phase: u32,
        percent: u32,
        message: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn finished(emitter: &SignalEmitter<'_>, job: u32, action: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn failed(
        emitter: &SignalEmitter<'_>,
        job: u32,
        action: &str,
        message: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn warning(
        emitter: &SignalEmitter<'_>,
        job: u32,
        action: &str,
        message: &str,
    ) -> zbus::Result<()>;
}

impl Users {
    fn next_job(&self) -> u32 {
        self.inner.next_job.fetch_add(1, Ordering::Relaxed)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install().ok();
    let connection = zbus::connection::Builder::system()
        .wrap_err("no se pudo abrir el bus del sistema")?
        .name(BUS_NAME)
        .wrap_err("no se pudo tomar el nombre D-Bus")?
        .serve_at(OBJECT_PATH, Users::default())
        .wrap_err("no se pudo publicar la interfaz")?
        .build()
        .await
        .wrap_err("no se pudo conectar al bus del sistema")?;
    let _ = connection;
    std::future::pending::<()>().await;
    Ok(())
}

async fn create(
    username: &str,
    full_name: &str,
    admin: bool,
    password: &Zeroizing<String>,
    emitter: &SignalEmitter<'static>,
    job: u32,
) -> Result<()> {
    if existing(username) {
        bail!("el usuario '{username}' ya existe");
    }
    progress(emitter, job, "create", 1, 10, "Validando").await;
    let hash = nixcore::crypto::hash_password(password)?;
    let user = UserData {
        version: 1,
        username: username.to_string(),
        full_name: full_name.to_string(),
        admin,
    };
    progress(emitter, job, "create", 2, 30, "Escribiendo la cuenta").await;
    user.write()?;
    if let Err(err) = write_hash(username, &hash) {
        let _ = std::fs::remove_file(user_data::path_for(username));
        return Err(err);
    }
    progress(emitter, job, "create", 3, 50, "Reconstruyendo el sistema").await;
    if let Err(err) = rebuild_system(emitter, job, "create", Vec::new()).await {
        let _ = std::fs::remove_file(user_data::path_for(username));
        let _ = std::fs::remove_file(hash_path(username));
        return Err(err);
    }
    progress(emitter, job, "create", 4, 100, "Cuenta creada").await;
    Ok(())
}

async fn delete(
    username: &str,
    keep_home: bool,
    emitter: &SignalEmitter<'static>,
    job: u32,
) -> Result<()> {
    let Some(user) = UserData::read(username)? else {
        bail!("el usuario '{username}' no existe");
    };
    if user.admin && admin_count()? <= 1 {
        bail!("no se puede borrar el último administrador");
    }
    if !keep_home && user_has_processes(username)? {
        bail!("'{username}' tiene procesos abiertos; cierra su sesión antes de borrar su home");
    }
    progress(emitter, job, "delete", 1, 10, "Eliminando la cuenta").await;

    let user_bak = std::fs::read_to_string(user_data::path_for(username)).ok();
    let hash_bak = std::fs::read(hash_path(username)).ok();
    let prefs_bak = std::fs::read_to_string(user_data::prefs_path_for(username)).ok();
    let _ = std::fs::remove_file(user_data::path_for(username));
    let _ = std::fs::remove_file(hash_path(username));
    let _ = std::fs::remove_file(user_data::prefs_path_for(username));

    progress(emitter, job, "delete", 2, 50, "Reconstruyendo el sistema").await;
    if let Err(err) = rebuild_system(emitter, job, "delete", vec![username.to_string()]).await {
        restore(
            username,
            user_bak.as_deref(),
            hash_bak.as_deref(),
            prefs_bak.as_deref(),
        );
        return Err(err);
    }
    if !keep_home {
        let home = Path::new("/home").join(username);
        if home.exists() {
            std::fs::remove_dir_all(&home)?;
        }
    }
    progress(emitter, job, "delete", 3, 100, "Cuenta eliminada").await;
    Ok(())
}

async fn set_admin(
    username: &str,
    admin: bool,
    emitter: &SignalEmitter<'static>,
    job: u32,
) -> Result<()> {
    let mut user = existing_user(username)?;
    if user.admin && !admin && admin_count()? <= 1 {
        bail!("no se puede quitar el último administrador");
    }
    user.admin = admin;
    apply(user, emitter, job, "set-admin").await
}

async fn set_full_name(
    username: &str,
    full_name: &str,
    emitter: &SignalEmitter<'static>,
    job: u32,
) -> Result<()> {
    let mut user = existing_user(username)?;
    user.full_name = full_name.to_string();
    apply(user, emitter, job, "set-full-name").await
}

async fn set_password(
    username: &str,
    password: &Zeroizing<String>,
    emitter: &SignalEmitter<'static>,
    job: u32,
) -> Result<()> {
    let _ = existing_user(username)?;
    progress(
        emitter,
        job,
        "set-password",
        1,
        30,
        "Cambiando la contraseña",
    )
    .await;
    let hash = nixcore::crypto::hash_password(password)?;
    let old = std::fs::read(hash_path(username)).ok();
    write_hash(username, &hash)?;
    if let Err(err) = rebuild_system(emitter, job, "set-password", Vec::new()).await {
        if let Some(old) = old {
            let _ = write_hash(username, &String::from_utf8_lossy(&old));
        }
        return Err(err);
    }
    Ok(())
}

async fn apply(
    user: UserData,
    emitter: &SignalEmitter<'static>,
    job: u32,
    action: &str,
) -> Result<()> {
    progress(emitter, job, action, 1, 30, "Escribiendo la cuenta").await;
    let old = std::fs::read_to_string(user_data::path_for(&user.username)).ok();
    user.write()?;
    progress(emitter, job, action, 2, 50, "Reconstruyendo el sistema").await;
    if let Err(err) = rebuild_system(emitter, job, action, Vec::new()).await {
        restore(&user.username, old.as_deref(), None, None);
        return Err(err);
    }
    progress(emitter, job, action, 3, 100, "Aplicado").await;
    Ok(())
}

async fn finish(emitter: &SignalEmitter<'static>, job: u32, action: &str, result: Result<()>) {
    match result {
        Ok(()) => {
            let _ = Users::finished(emitter, job, action).await;
        }
        Err(err) => {
            let _ = Users::failed(emitter, job, action, &err.to_string()).await;
        }
    }
}

async fn progress(
    emitter: &SignalEmitter<'static>,
    job: u32,
    action: &str,
    phase: u32,
    percent: u32,
    message: &str,
) {
    let _ = Users::progress(emitter, job, action, phase, percent, message).await;
}

async fn rebuild_system(
    emitter: &SignalEmitter<'static>,
    job: u32,
    action: &str,
    allow_missing: Vec<String>,
) -> Result<()> {
    let (flake_ref, _rev) = rebuild::system_ref()?;
    let outcome =
        tokio::task::spawn_blocking(move || rebuild::run(&flake_ref, Mode::Switch, &allow_missing))
            .await
            .wrap_err("la reconstrucción se interrumpió")??;
    warn_failed(emitter, job, action, &outcome).await;
    Ok(())
}

async fn warn_failed(emitter: &SignalEmitter<'static>, job: u32, action: &str, outcome: &Outcome) {
    if outcome.failed_units.is_empty() {
        return;
    }
    let message = format!(
        "el sistema se activó, pero quedaron unidades en fallo: {}",
        outcome.failed_units.join(", ")
    );
    let _ = Users::warning(emitter, job, action, &message).await;
}

fn validate_username(username: &str) -> fdo::Result<()> {
    if let Some(err) = user::username_error(username) {
        return Err(fdo::Error::Failed(format!("username inválido: {err}")));
    }
    Ok(())
}

fn validate_full_name(full_name: &str) -> fdo::Result<()> {
    if full_name.trim().is_empty() {
        return Err(fdo::Error::Failed("el nombre no puede estar vacío".into()));
    }
    if full_name.chars().count() > MAX_FULL_NAME {
        return Err(fdo::Error::Failed(format!(
            "el nombre supera {MAX_FULL_NAME} caracteres"
        )));
    }
    if full_name.chars().any(|c| c.is_control() || c == ':') {
        return Err(fdo::Error::Failed(
            "el nombre no puede llevar caracteres de control ni ':'".into(),
        ));
    }
    Ok(())
}

fn existing(username: &str) -> bool {
    user_data::path_for(username).exists() || passwd_line(username).is_some()
}

fn existing_user(username: &str) -> Result<UserData> {
    UserData::read(username)?
        .ok_or_else(|| color_eyre::eyre::eyre!("el usuario '{username}' no existe"))
}

fn passwd_line(username: &str) -> Option<String> {
    let text = std::fs::read_to_string("/etc/passwd").ok()?;
    text.lines()
        .find(|line| line.split(':').next() == Some(username))
        .map(str::to_string)
}

fn uid_of(username: &str) -> Option<u32> {
    passwd_line(username)?.split(':').nth(2)?.parse().ok()
}

fn admin_count() -> Result<usize> {
    Ok(UserData::list()?.into_iter().filter(|u| u.admin).count())
}

fn user_has_processes(username: &str) -> Result<bool> {
    let Some(uid) = uid_of(username) else {
        return Ok(false);
    };
    let uid_line = format!("Uid:\t{uid}\t");
    for entry in std::fs::read_dir("/proc")? {
        let Ok(entry) = entry else { continue };
        if !entry
            .file_name()
            .to_string_lossy()
            .chars()
            .all(|c| c.is_ascii_digit())
        {
            continue;
        }
        if let Ok(status) = std::fs::read_to_string(entry.path().join("status")) {
            if status.lines().any(|l| l.starts_with(&uid_line)) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn hash_path(username: &str) -> PathBuf {
    paths::secrets().join(format!("{username}.hash"))
}

fn write_hash(username: &str, hash: &str) -> Result<()> {
    let dir = paths::secrets();
    std::fs::create_dir_all(&dir)?;
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o711))?;
    let path = hash_path(username);
    let tmp = dir.join(format!(".{username}.hash.tmp"));
    {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&tmp)?;
        file.write_all(hash.as_bytes())?;
        file.sync_all()?;
    }
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

fn restore(username: &str, user: Option<&str>, hash: Option<&[u8]>, prefs: Option<&str>) {
    if let Some(text) = user {
        let _ = std::fs::write(user_data::path_for(username), text);
    }
    if let Some(bytes) = hash {
        let _ = write_hash(username, &String::from_utf8_lossy(bytes));
    }
    if let Some(text) = prefs {
        let _ = std::fs::write(user_data::prefs_path_for(username), text);
    }
}

fn sender(header: &Header<'_>) -> fdo::Result<String> {
    header
        .sender()
        .map(|s| s.as_str().to_string())
        .ok_or_else(|| fdo::Error::Failed("llamada sin remitente".into()))
}

async fn authorize_header(header: &Header<'_>) -> fdo::Result<()> {
    let connection = zbus::Connection::system()
        .await
        .map_err(|e| fdo::Error::Failed(e.to_string()))?;
    authorize(&connection, &sender(header)?).await
}

async fn authorize(connection: &Connection, sender: &str) -> fdo::Result<()> {
    let polkit = PolicyKit1Proxy::new(connection)
        .await
        .map_err(|e| fdo::Error::Failed(format!("polkit no disponible: {e}")))?;
    let mut subject = HashMap::new();
    subject.insert(
        "name".to_string(),
        OwnedValue::try_from(zbus::zvariant::Value::from(sender))
            .map_err(|e| fdo::Error::Failed(e.to_string()))?,
    );
    let (authorized, _challenge, _details) = polkit
        .check_authorization(
            ("system-bus-name".to_string(), subject),
            ACTION,
            HashMap::new(),
            1,
            "",
        )
        .await
        .map_err(|e| fdo::Error::Failed(format!("polkit falló: {e}")))?;
    if authorized {
        Ok(())
    } else {
        Err(fdo::Error::AccessDenied(
            "se requiere autenticación de administrador".into(),
        ))
    }
}
