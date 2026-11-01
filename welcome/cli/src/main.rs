//! `nixdots-setup`: CLI that applies the first-boot wizard data.
use color_eyre::eyre::{bail, eyre, Context, Result};
use nixcore::machine;
use nixcore::runner::{self, Cmd, Stream};
use nixcore::user_data::{self, Git, LegacyUser, Prefs, UserData};
use serde::Deserialize;
use std::io::Read;
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

const HELP: &str = "nixdots-setup apply <datos.json> [--repo <ruta>] [--dry-run]

  datos.json  fullName, username, locale, region, keyboardLayout,
              keyboardVariant, timeZone, timeZoneAuto, a11y, colorScheme,
              accentColor, avatarColor, autoLogin y, opcionalmente, gitName,
              gitEmail, avatarPath y admin.
  --repo      repositorio a reconstruir (defecto /persist/keel/repo)
  --dry-run   solo imprime lo que haría, sin escribir nada

Los datos personales van a /persist/keel/users/<usuario>.json; los de sistema (locale, zona,
teclado, prettyHostName, autoLoginUser) se fusionan en machine.toml. La
contraseña se lee de stdin.

nixdots-setup migrate [--dry-run]

  migrate     convierte el user.json antiguo en /persist/keel/users/<usuario>.toml y mueve
              sus campos de sistema a machine.toml, dejando user.json.bak.

nixdots-setup migrate-toml

  migrate-toml  convierte machine.json y /persist/keel/users/*.json a .toml conservando los
                originales como respaldo (los seguirá leyendo si falta el .toml).";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Data {
    full_name: String,
    username: String,
    locale: String,
    #[serde(default)]
    region: Option<String>,
    #[serde(default)]
    keyboard_layout: String,
    #[serde(default)]
    keyboard_variant: String,
    time_zone: String,
    #[serde(default)]
    time_zone_auto: Option<bool>,
    #[serde(default)]
    color_scheme: Option<String>,
    #[serde(default)]
    accent_color: Option<String>,
    #[serde(default)]
    avatar_color: Option<String>,
    #[serde(default)]
    avatar_path: Option<String>,
    #[serde(default)]
    auto_login: Option<bool>,
    #[serde(default)]
    a11y: Option<nixcore::user_data::A11y>,
    #[serde(default)]
    git_name: Option<String>,
    #[serde(default)]
    git_email: Option<String>,
    #[serde(default)]
    admin: Option<bool>,
}

fn main() {
    color_eyre::install().ok();
    if let Err(err) = run() {
        eprintln!("ERROR: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("apply") => apply(&args[1..]),
        Some("migrate") => migrate(&args[1..]),
        Some("migrate-toml") => migrate_toml(&args[1..]),
        Some("-h") | Some("--help") | None => {
            println!("{HELP}");
            Ok(())
        }
        Some(other) => bail!("orden desconocida: {other}\n\n{HELP}"),
    }
}

fn apply(args: &[String]) -> Result<()> {
    let mut data_path = None;
    let mut repo = PathBuf::from("/persist/keel/repo");
    let mut dry_run = false;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--repo" => {
                repo = PathBuf::from(it.next().ok_or_else(|| eyre!("--repo necesita un valor"))?)
            }
            "--dry-run" => dry_run = true,
            "-h" | "--help" => {
                println!("{HELP}");
                return Ok(());
            }
            other if other.starts_with("--") => bail!("opción desconocida: {other}"),
            other => data_path = Some(PathBuf::from(other)),
        }
    }
    let data_path = data_path.ok_or_else(|| eyre!("falta el archivo de datos"))?;
    let data = load_data(&data_path)?;
    validate(&data)?;
    let password = read_password()?;
    let sim_fail = std::env::var_os("KEEL_SIM_FAIL").is_some();

    phase(0, dry_run);
    let hash = nixcore::crypto::hash_password(&password)?;
    let secrets = Path::new("/persist/secrets");
    let hash_path = secrets.join(format!("{}.hash", data.username));
    println!("$ escribir {} (0600)", hash_path.display());
    if !dry_run {
        std::fs::create_dir_all(secrets)?;
        set_mode(secrets, 0o711)?;
        {
            use std::io::Write;
            use std::os::unix::fs::OpenOptionsExt;
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&hash_path)?;
            file.write_all(hash.as_bytes())?;
            file.sync_all()?;
        }
        set_mode(&hash_path, 0o600)?;
    }

    phase(1, dry_run);
    if sim_fail {
        bail!("Fallo simulado al aplicar la configuración");
    }
    let user = UserData {
        version: 1,
        username: data.username.clone(),
        full_name: data.full_name.clone(),
        admin: data.admin.unwrap_or(true),
    };
    let prefs = Prefs {
        version: 1,
        color_scheme: data.color_scheme.clone().and_then(non_empty),
        accent_color: data.accent_color.clone().and_then(non_empty),
        avatar_color: data.avatar_color.clone().and_then(non_empty),
        avatar_path: data.avatar_path.clone().and_then(non_empty),
        a11y: data.a11y.clone(),
        git: match (data.git_name.clone(), data.git_email.clone()) {
            (Some(name), Some(email)) if !name.trim().is_empty() && !email.trim().is_empty() => {
                Some(Git { name, email })
            }
            _ => None,
        },
    };
    println!(
        "$ escribir {} (0644)",
        user_data::path_for(&data.username).display()
    );
    if !dry_run {
        user.write()?;
        println!(
            "$ escribir {} (0644)",
            user_data::prefs_path_for(&data.username).display()
        );
        prefs.write(&data.username)?;
    }

    let mut fields = serde_json::Map::new();
    fields.insert(
        "prettyHostName".into(),
        localized_pretty_host_name(&current_pretty_hostname(), &data.full_name, &data.locale)
            .into(),
    );
    fields.insert("defaultLocale".into(), data.locale.clone().into());
    if let Some(region) = data.region.clone().and_then(non_empty) {
        fields.insert("region".into(), region.into());
    }
    fields.insert("timeZone".into(), data.time_zone.clone().into());
    fields.insert(
        "timeZoneAuto".into(),
        data.time_zone_auto.unwrap_or(false).into(),
    );
    if let Some(layout) = non_empty(data.keyboard_layout.clone()) {
        fields.insert("keyboardLayout".into(), layout.into());
    }
    if let Some(variant) = non_empty(data.keyboard_variant.clone()) {
        fields.insert("keyboardVariant".into(), variant.into());
    }
    fields.insert(
        "autoLoginUser".into(),
        if data.auto_login.unwrap_or(false) {
            data.username.clone().into()
        } else {
            serde_json::Value::Null
        },
    );
    println!("$ fusionar {} (0644)", machine::path().display());
    if !dry_run {
        machine::merge_fields(fields)?;
    }

    phase(2, dry_run);
    let host = hostname()?;
    let rebuild = Cmd::new("nixos-rebuild")
        .args(["boot", "--impure", "--flake"])
        .arg(format!("{}#{host}", repo.display()));
    run_step(&rebuild, dry_run, "nixos-rebuild")?;

    phase(3, dry_run);
    let home_link = format!("/home/{}/dotfiles", data.username);
    println!(
        "$ ln -sfn {} {home_link} && chown -R 1000:users {}",
        repo.display(),
        repo.display()
    );
    if !dry_run {
        link_home(&data.username, &repo)?;
    }
    phase(4, dry_run);
    Ok(())
}

fn migrate(args: &[String]) -> Result<()> {
    let mut dry_run = false;
    for arg in args {
        match arg.as_str() {
            "--dry-run" => dry_run = true,
            "-h" | "--help" => {
                println!("{HELP}");
                return Ok(());
            }
            other => bail!("opción desconocida: {other}"),
        }
    }

    let Some(old) = LegacyUser::read()? else {
        println!("No hay user.json; nada que migrar.");
        return Ok(());
    };
    let username = old
        .username
        .clone()
        .filter(|u| !u.trim().is_empty())
        .ok_or_else(|| eyre!("user.json no tiene username"))?;

    let user = UserData {
        version: 1,
        username: username.clone(),
        full_name: old
            .userdescription
            .clone()
            .and_then(non_empty)
            .unwrap_or_else(|| username.clone()),
        admin: true,
    };
    let prefs = Prefs {
        version: 1,
        color_scheme: old.color_scheme.clone().and_then(non_empty),
        accent_color: old.accent_color.clone().and_then(non_empty),
        avatar_color: old.avatar_color.clone().and_then(non_empty),
        avatar_path: old.avatar_path.clone().and_then(non_empty),
        a11y: old.a11y.clone(),
        git: match (old.git_name.clone(), old.git_email.clone()) {
            (Some(name), Some(email)) if !name.trim().is_empty() && !email.trim().is_empty() => {
                Some(Git { name, email })
            }
            _ => None,
        },
    };
    println!("$ escribir {}", user_data::path_for(&username).display());
    if !dry_run {
        user.write()?;
        if prefs != Prefs::default() {
            println!(
                "$ escribir {}",
                user_data::prefs_path_for(&username).display()
            );
            prefs.write(&username)?;
        }
    }

    let mut fields = serde_json::Map::new();
    if let Some(v) = old.pretty_host_name.clone().and_then(non_empty) {
        fields.insert("prettyHostName".into(), v.into());
    }
    if let Some(v) = old.default_locale.clone().and_then(non_empty) {
        fields.insert("defaultLocale".into(), v.into());
    }
    if let Some(v) = old.region.clone().and_then(non_empty) {
        fields.insert("region".into(), v.into());
    }
    if let Some(v) = old.time_zone.clone().and_then(non_empty) {
        fields.insert("timeZone".into(), v.into());
    }
    if let Some(v) = old.time_zone_auto {
        fields.insert("timeZoneAuto".into(), v.into());
    }
    if let Some(v) = old.keyboard_layout.clone().and_then(non_empty) {
        fields.insert("keyboardLayout".into(), v.into());
    }
    if let Some(v) = old.keyboard_variant.clone().and_then(non_empty) {
        fields.insert("keyboardVariant".into(), v.into());
    }
    fields.insert(
        "autoLoginUser".into(),
        if old.auto_login == Some(true) {
            username.clone().into()
        } else {
            serde_json::Value::Null
        },
    );
    println!("$ fusionar {}", machine::path().display());
    if !dry_run {
        machine::merge_fields(fields)?;
    }

    let path = user_data::legacy_path();
    let bak = path.with_extension("json.bak");
    println!("$ mv {} {}", path.display(), bak.display());
    if !dry_run {
        std::fs::rename(&path, &bak)?;
    }

    println!("Migración completada.");
    Ok(())
}

fn migrate_toml(args: &[String]) -> Result<()> {
    if let Some(arg) = args.first() {
        match arg.as_str() {
            "-h" | "--help" => {
                println!("{HELP}");
                return Ok(());
            }
            other => bail!("opción desconocida: {other}"),
        }
    }
    let written = user_data::migrate_to_toml()?;
    println!("Convertidos {written} archivos a TOML (los .json quedan como respaldo).");
    Ok(())
}

fn phase(n: u8, dry_run: bool) {
    if dry_run && n > 0 {
        std::thread::sleep(std::time::Duration::from_millis(1600));
    }
    println!("PHASE {n}");
}

fn load_data(path: &Path) -> Result<Data> {
    let text = std::fs::read_to_string(path)
        .wrap_err_with(|| format!("no se pudo leer {}", path.display()))?;
    serde_json::from_str(&text).wrap_err("los datos no tienen el formato esperado")
}

fn validate(data: &Data) -> Result<()> {
    if data.full_name.trim().is_empty() {
        bail!("fullName está vacío");
    }
    if let Some(err) = nixcore::user::username_error(&data.username) {
        bail!("username inválido: {err}");
    }
    if data.locale.trim().is_empty() {
        bail!("locale está vacío");
    }
    if !locale_known(&data.locale) {
        bail!("locale desconocido: {}", data.locale);
    }
    if let Some(region) = data.region.as_deref() {
        if !region.trim().is_empty() && !locale_known(region) {
            bail!("region desconocida: {region}");
        }
    }
    if data.time_zone.trim().is_empty() {
        bail!("timeZone está vacío");
    }
    if !timezone_known(&data.time_zone) {
        bail!("zona horaria desconocida: {}", data.time_zone);
    }
    if let Some(email) = &data.git_email {
        if !email.trim().is_empty() && !(email.contains('@') && email.contains('.')) {
            bail!("gitEmail no parece un correo");
        }
    }
    Ok(())
}

fn locale_known(code: &str) -> bool {
    localekit::locale::all().iter().any(|l| l.code == code)
}

fn timezone_known(zone: &str) -> bool {
    localekit::timezone::zones().iter().any(|z| z == zone)
}

fn non_empty(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn read_password() -> Result<Zeroizing<String>> {
    let mut buf = Zeroizing::new(String::new());
    std::io::stdin()
        .read_to_string(&mut buf)
        .wrap_err("no se pudo leer la contraseña de stdin")?;
    let password = buf.trim_end_matches(['\n', '\r']).to_string();
    if password.is_empty() {
        bail!("la contraseña está vacía");
    }
    Ok(Zeroizing::new(password))
}

fn hostname() -> Result<String> {
    let name = std::fs::read_to_string("/proc/sys/kernel/hostname")
        .wrap_err("no se pudo leer el hostname del sistema")?;
    let name = name.trim().to_string();
    if name.is_empty() {
        bail!("el hostname está vacío");
    }
    Ok(name)
}

fn current_pretty_hostname() -> String {
    if let Ok(text) = std::fs::read_to_string("/etc/machine-info") {
        for line in text.lines() {
            if let Some(rest) = line.trim().strip_prefix("PRETTY_HOSTNAME=") {
                let value = rest.trim().trim_matches('"');
                if !value.is_empty() {
                    return value.to_string();
                }
            }
        }
    }
    if let Ok(out) = std::process::Command::new("hostnamectl")
        .arg("--pretty")
        .output()
    {
        if out.status.success() {
            let value = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !value.is_empty() {
                return value;
            }
        }
    }
    String::new()
}

fn localized_pretty_host_name(model: &str, full_name: &str, locale: &str) -> String {
    let first = full_name.split_whitespace().next().unwrap_or("");
    let model = if model.trim().is_empty() {
        hostname().unwrap_or_default()
    } else {
        model.trim().to_string()
    };
    if model.is_empty() {
        return first.to_string();
    }
    if first.is_empty() {
        return model;
    }
    match language_code(locale) {
        "en" => format!("{first}'s {model}"),
        "de" => format!("{model} von {first}"),
        "it" => format!("{model} di {first}"),
        _ => format!("{model} de {first}"),
    }
}

fn language_code(locale: &str) -> &str {
    locale.split(['_', '.']).next().unwrap_or("")
}

fn run_step(cmd: &Cmd, dry_run: bool, what: &str) -> Result<()> {
    println!("$ {}", cmd.display);
    if dry_run {
        return Ok(());
    }
    let mut sink = |stream: Stream| match stream {
        Stream::Stdout(l) => println!("{l}"),
        Stream::Stderr(l) => eprintln!("{l}"),
    };
    let code = runner::run(cmd, &mut sink);
    if code != 0 {
        bail!("{what} falló (código {code})");
    }
    Ok(())
}

fn set_mode(path: &Path, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
    Ok(())
}

fn link_home(username: &str, repo: &Path) -> Result<()> {
    let home = format!("/home/{username}");
    std::fs::create_dir_all(&home).wrap_err_with(|| format!("no se pudo crear {home}"))?;
    let link = format!("{home}/dotfiles");
    let _ = std::fs::remove_file(&link);
    std::os::unix::fs::symlink(repo, &link).wrap_err("no se pudo crear el enlace ~/dotfiles")?;

    chown(&["-R", "1000:users"], repo)?;
    chown(&["1000:users"], Path::new(&home))?;
    Ok(())
}

fn chown(args: &[&str], path: &Path) -> Result<()> {
    let status = std::process::Command::new("chown")
        .args(args)
        .arg(path)
        .status()?;
    if !status.success() {
        bail!("chown falló en {}", path.display());
    }
    Ok(())
}
