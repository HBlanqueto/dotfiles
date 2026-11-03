use color_eyre::eyre::{bail, eyre, Context, Result};
use nixcore::machine;
use nixcore::runner::{self, Cmd, Stream};
use nixcore::store;
use nixcore::user;
use nixcore::user_data::{self, Git, LegacyUser, Prefs, UserData};
use std::io::Write;
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

const HELP: &str = "nixdots-setup [--repo <ruta>] [--dry-run]

  Asistente de texto del primer arranque. Pide nombre, usuario, contraseña,
  inicio automático e identidad de git; escribe /persist/keel/users/<usuario>.toml,
  el hash y la semilla de prefs, fusiona prettyHostName y autoLoginUser en
  machine.toml y ejecuta nixos-rebuild boot.
  --repo      repositorio a reconstruir (defecto /persist/keel/repo)
  --dry-run   solo imprime lo que haría, sin escribir nada

nixdots-setup migrate [--dry-run]

  migrate     convierte el user.json antiguo en /persist/keel/users/<usuario>.toml y mueve
              sus campos de sistema a machine.toml, dejando user.json.bak.

nixdots-setup migrate-toml

  migrate-toml  convierte machine.json y /persist/keel/users/*.json a .toml conservando los
                originales como respaldo (los seguirá leyendo si falta el .toml).";

struct Answers {
    full_name: String,
    username: String,
    password: Zeroizing<String>,
    auto_login: bool,
    git: Option<Git>,
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
        Some("migrate") => migrate(&args[1..]),
        Some("migrate-toml") => migrate_toml(&args[1..]),
        Some("-h") | Some("--help") => {
            println!("{HELP}");
            Ok(())
        }
        _ => setup(&args),
    }
}

fn setup(args: &[String]) -> Result<()> {
    let mut repo = PathBuf::from("/persist/keel/repo");
    let mut dry_run = false;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--repo" => {
                repo = PathBuf::from(it.next().ok_or_else(|| eyre!("--repo necesita un valor"))?)
            }
            "--dry-run" => dry_run = true,
            other => bail!("opción desconocida: {other}\n\n{HELP}"),
        }
    }
    println!("\nBienvenido a Keel. Vamos a crear tu cuenta.\n");
    let answers = ask()?;
    apply(&answers, &repo, dry_run)
}

fn ask() -> Result<Answers> {
    let full_name = ask_valid("Nombre completo", None, user::full_name_error)?;
    let suggested = user::generate_username(&full_name);
    let username = ask_valid("Nombre de usuario", Some(&suggested), user::username_error)?;
    let password = ask_password()?;
    let auto_login = ask_yes_no("¿Iniciar sesión automáticamente?")?;
    let git_name = prompt("Nombre para git (Intro para omitir)", None)?;
    let git = if git_name.is_empty() {
        None
    } else {
        let email = ask_valid("Correo para git", None, email_error)?;
        Some(Git {
            name: git_name,
            email,
        })
    };
    Ok(Answers {
        full_name,
        username,
        password,
        auto_login,
        git,
    })
}

fn prompt(label: &str, default: Option<&str>) -> Result<String> {
    match default {
        Some(d) if !d.is_empty() => print!("{label} [{d}]: "),
        _ => print!("{label}: "),
    }
    std::io::stdout().flush()?;
    let mut line = String::new();
    if std::io::stdin().read_line(&mut line)? == 0 {
        bail!("se cerró la entrada");
    }
    let line = line.trim().to_string();
    Ok(match default {
        Some(d) if line.is_empty() => d.to_string(),
        _ => line,
    })
}

fn ask_valid(
    label: &str,
    default: Option<&str>,
    error: fn(&str) -> Option<&'static str>,
) -> Result<String> {
    loop {
        let answer = prompt(label, default)?;
        match error(&answer) {
            Some(err) => println!("  {err}"),
            None => return Ok(answer),
        }
    }
}

fn ask_yes_no(label: &str) -> Result<bool> {
    loop {
        match prompt(&format!("{label} [s/N]"), None)?
            .to_lowercase()
            .as_str()
        {
            "" | "n" | "no" => return Ok(false),
            "s" | "si" | "sí" => return Ok(true),
            _ => println!("  responde s o n"),
        }
    }
}

fn ask_password() -> Result<Zeroizing<String>> {
    loop {
        let password = read_hidden("Contraseña")?;
        if password.is_empty() {
            println!("  no puede estar vacía");
            continue;
        }
        if *read_hidden("Repite la contraseña")? != *password {
            println!("  no coinciden");
            continue;
        }
        return Ok(password);
    }
}

fn read_hidden(label: &str) -> Result<Zeroizing<String>> {
    print!("{label}: ");
    std::io::stdout().flush()?;
    let _echo = EchoOff::new();
    let mut line = Zeroizing::new(String::new());
    if std::io::stdin().read_line(&mut line)? == 0 {
        bail!("se cerró la entrada");
    }
    println!();
    Ok(Zeroizing::new(
        line.trim_end_matches(['\n', '\r']).to_string(),
    ))
}

struct EchoOff;

impl EchoOff {
    fn new() -> Self {
        stty("-echo");
        EchoOff
    }
}

impl Drop for EchoOff {
    fn drop(&mut self) {
        stty("echo");
    }
}

fn stty(setting: &str) {
    let _ = std::process::Command::new("stty")
        .arg(setting)
        .stderr(std::process::Stdio::null())
        .status();
}

fn email_error(email: &str) -> Option<&'static str> {
    if email.contains('@') && email.contains('.') {
        None
    } else {
        Some("no parece un correo")
    }
}

fn apply(answers: &Answers, repo: &Path, dry_run: bool) -> Result<()> {
    let username = &answers.username;

    println!("\n== Guardando la cuenta ==");
    let hash = nixcore::crypto::hash_password(&answers.password)?;
    println!(
        "$ escribir {} (0600)",
        user_data::hash_path(username).display()
    );
    let user = UserData {
        version: 1,
        username: username.clone(),
        full_name: answers.full_name.clone(),
        admin: true,
    };
    let prefs = Prefs {
        git: answers.git.clone(),
        ..Prefs::default()
    };
    println!("$ escribir {}", user_data::path_for(username).display());
    println!(
        "$ escribir {} (0600)",
        user_data::prefs_path_for(username).display()
    );
    let mut fields = serde_json::Map::new();
    fields.insert(
        "prettyHostName".into(),
        localized_pretty_host_name(
            &current_pretty_hostname(),
            &answers.full_name,
            &machine_locale(),
        )
        .into(),
    );
    fields.insert(
        "autoLoginUser".into(),
        if answers.auto_login {
            username.clone().into()
        } else {
            serde_json::Value::Null
        },
    );
    println!("$ fusionar {}", machine::path().display());
    if !dry_run {
        user_data::write_hash(username, &hash)?;
        user.write()?;
        prefs.write(username)?;
        machine::merge_fields(fields)?;
    }

    println!("\n== Construyendo el sistema ==");
    let rebuild = Cmd::new("nixos-rebuild")
        .args(["boot", "--impure", "--flake"])
        .arg(format!("{}#{}", repo.display(), hostname()?));
    run_step(&rebuild, dry_run, "nixos-rebuild")?;

    println!(
        "$ ln -sfn {} /home/{username}/dotfiles && chown -R 1000:users {}",
        repo.display(),
        repo.display()
    );
    if !dry_run {
        link_home(username, repo)?;
    }
    println!("\nListo. El equipo se reiniciará.");
    Ok(())
}

fn machine_locale() -> String {
    store::load_document(&machine::path())
        .ok()
        .and_then(|doc| doc.get("defaultLocale")?.as_str().map(str::to_string))
        .unwrap_or_default()
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

fn non_empty(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
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
