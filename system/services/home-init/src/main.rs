//! `steward home prepare`: sets up `~/.config/home-manager` on first login.
use std::path::{Path, PathBuf};
use std::process::Command;

use color_eyre::eyre::{bail, eyre, Context, Result};
use nixcore::user_data::Prefs;

const FLAKE_TEMPLATE: &str = include_str!("../../../templates/home/flake.nix");

fn main() {
    color_eyre::install().ok();
    if let Err(err) = run() {
        status(&format!("No se pudo preparar el escritorio: {err}"));
        eprintln!("ERROR: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let username = username()?;
    let home = std::env::var("HOME").wrap_err("falta HOME")?;
    let config = Path::new(&home).join(".config/home-manager");
    let prefs = Prefs::read(&username).unwrap_or(None);

    if config.join("flake.nix").is_file() {
        status("Tu escritorio ya está preparado.");
        delete_seed(&username);
        return Ok(());
    }

    phase(0);
    status("Preparando el flake del home");
    let (flake_ref, rev) = nixcore::rebuild::system_ref()
        .map(|(path, rev)| (path.display().to_string(), rev))
        .unwrap_or_default();
    let (url, used_fallback) = keel_url(&flake_ref, &rev)?;
    if used_fallback {
        eprintln!(
            "steward home prepare: sin revisión válida; usando path:{flake_ref} \
             (nix flake update keel no actualizará este input)"
        );
    }
    let system = system_arch()?;

    std::fs::create_dir_all(&config)
        .wrap_err_with(|| format!("no se pudo crear {}", config.display()))?;

    phase(1);
    status("Escribiendo la configuración");
    write_flake(&config, &url, system)?;
    std::fs::write(
        config.join("home.nix"),
        home_nix(&username, &home, prefs.as_ref()),
    )?;

    if let Some(prefs) = prefs.as_ref() {
        prefs.write_home_file(&config.join("prefs.toml"))?;
    }

    if config.join(".git").exists() {
        let _ = Command::new("git")
            .arg("-C")
            .arg(&config)
            .args(["add", "flake.nix", "home.nix", "prefs.toml"])
            .status();
    }

    phase(2);
    status("Aplicando tu escritorio");
    let mut ok = home_switch(&config);
    if !ok && !used_fallback {
        status("Sin acceso a la revisión; uso la copia instalada");
        let fallback = format!("path:{flake_ref}");
        write_flake(&config, &fallback, system)?;
        ok = home_switch(&config);
    }
    if !ok {
        bail!("home-manager switch falló");
    }

    delete_seed(&username);
    phase(4);
    status("Escritorio listo.");
    Ok(())
}

fn write_flake(config: &Path, url: &str, system: &str) -> Result<()> {
    let flake = FLAKE_TEMPLATE
        .replace("github:HBlanqueto/dotfiles", url)
        .replace("system = \"SYSTEM\";", &format!("system = \"{system}\";"));
    std::fs::write(config.join("flake.nix"), flake)?;
    Ok(())
}

fn home_switch(config: &Path) -> bool {
    Command::new("home-manager")
        .args([
            "switch",
            "-b",
            "backup",
            "--flake",
            &format!("{}#default", config.display()),
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn keel_url(flake_ref: &str, rev: &str) -> Result<(String, bool)> {
    if is_rev(rev) {
        return Ok((format!("github:HBlanqueto/dotfiles/{rev}"), false));
    }
    if flake_ref.is_empty()
        || !Path::new(flake_ref).is_dir()
        || !Path::new(flake_ref).join("flake.nix").is_file()
    {
        bail!("no hay revisión ni /etc/keel/source válidos");
    }
    Ok((format!("path:{flake_ref}"), true))
}

fn is_rev(rev: &str) -> bool {
    rev.len() == 40 && rev.chars().all(|c| c.is_ascii_hexdigit())
}

fn username() -> Result<String> {
    for key in ["USER", "LOGNAME"] {
        if let Ok(value) = std::env::var(key) {
            if !value.is_empty() {
                return Ok(value);
            }
        }
    }
    let out = Command::new("id")
        .arg("-un")
        .output()
        .wrap_err("no se pudo determinar el usuario")?;
    let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if name.is_empty() {
        bail!("no se pudo determinar el usuario");
    }
    Ok(name)
}

fn system_arch() -> Result<&'static str> {
    match std::env::consts::ARCH {
        "x86_64" => Ok("x86_64-linux"),
        "aarch64" => Ok("aarch64-linux"),
        other => Err(eyre!("arquitectura no soportada: {other}")),
    }
}

fn home_nix(username: &str, home: &str, prefs: Option<&Prefs>) -> String {
    let (name, email) = prefs
        .and_then(|p| p.git.as_ref())
        .map(|g| (g.name.as_str(), g.email.as_str()))
        .unwrap_or(("", ""));
    format!(
        "# Mi home; el resto sale de keel.homeModules.\n\
         {{ ... }}:\n\
         {{\n\
         \x20 home.username = {u};\n\
         \x20 home.homeDirectory = {h};\n\
         \x20 home.stateVersion = \"26.05\";\n\n\
         \x20 keel.git.name = {n};\n\
         \x20 keel.git.email = {e};\n\
         }}\n",
        u = nix_str(username),
        h = nix_str(home),
        n = nix_str(name),
        e = nix_str(email),
    )
}

fn nix_str(s: &str) -> String {
    let escaped = s
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace("${", "\\${");
    format!("\"{escaped}\"")
}

fn phase(n: u32) {
    println!("PHASE {n}");
    let _ = write_status(&format!("phase={n}\n"));
}

fn status(message: &str) {
    println!("STATUS {message}");
    let _ = write_status(&format!("message={message}\n"));
}

fn write_status(text: &str) -> Result<()> {
    let dir = match std::env::var("XDG_RUNTIME_DIR") {
        Ok(dir) if !dir.is_empty() => PathBuf::from(dir).join("keel"),
        _ => PathBuf::from("/tmp").join(format!("keel-{}", std::process::id())),
    };
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("home-init.status"), text)?;
    Ok(())
}

fn delete_seed(username: &str) {
    let _ = Command::new("busctl")
        .args([
            "call",
            "io.github.HBlanqueto.Keel.Crew1",
            "/io/github/HBlanqueto/Keel/Crew1",
            "io.github.HBlanqueto.Keel.Crew1",
            "DeletePrefsSeed",
            "s",
            username,
        ])
        .status();
}
