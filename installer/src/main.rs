//! Text-mode installer entry point (dry-run by default, `--install` for real).
use color_eyre::eyre::{bail, Result};
use nixdots_installer::app::App;
use nixdots_installer::detect;
use nixdots_installer::theme;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::Terminal;
use std::io::Stdout;
use std::path::{Path, PathBuf};
use std::process::Command;

struct Args {
    dry_run: bool,
    repo: String,
    help: bool,
}

fn parse_args() -> Result<Args> {
    let mut dry_run = true;
    let mut repo = "github:HBlanqueto/dotfiles".to_string();
    let mut help = false;

    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--install" => dry_run = false,
            "--repo" => {
                if let Some(value) = it.next() {
                    repo = value;
                }
            }
            "--help" | "-h" => help = true,
            other => bail!("argumento desconocido: {other}\n\n{}", help_text()),
        }
    }
    Ok(Args {
        dry_run,
        repo,
        help,
    })
}

fn help_text() -> String {
    "nixdots-installer [--install] [--repo <ruta|url>]\n\n  (sin argumentos)  simulación\n  --install         instalación real (requiere root, desde la ISO)\n  --repo            repositorio a instalar (defecto github:HBlanqueto/dotfiles)".to_string()
}

const STATE_DIR: &str = "/tmp/keel-installer";

fn main() -> Result<()> {
    color_eyre::install()?;
    theme::console::install_panic_hook();

    let _ = std::process::Command::new("dmesg")
        .arg("-n")
        .arg("1")
        .status();

    let args = parse_args()?;
    if args.help {
        println!("{}", help_text());
        return Ok(());
    }

    if args.dry_run {
        return run(args);
    }

    mark_started();
    let result = run(args);
    write_exit_code(result.is_ok());
    result
}

fn run(args: Args) -> Result<()> {
    if !args.dry_run {
        if !detect::is_root() {
            bail!("--install requiere root");
        }
        if !detect::is_live_environment() {
            bail!("--install debe ejecutarse desde el entorno live de NixOS");
        }
    }

    let _console = theme::console::Guard::activate();

    let repo_dir = std::env::temp_dir().join("keel-installer-repo");
    let cloned = prepare_repo(&args.repo, &repo_dir).is_ok();
    if !cloned && !args.dry_run {
        prepare_repo(&args.repo, &repo_dir)?;
    }

    let mut app = App::new(args.dry_run, args.repo, repo_dir);
    app.source_ok = cloned;
    app.network = detect::network_online();
    app.refresh_network();

    let mut terminal = setup_terminal()?;
    let result = app.run(&mut terminal);
    restore_terminal(&mut terminal)?;
    result
}

fn mark_started() {
    let _ = std::fs::create_dir_all(STATE_DIR);
    let _ = std::fs::write(format!("{}/started", STATE_DIR), b"");
}

fn write_exit_code(ok: bool) {
    let _ = std::fs::create_dir_all(STATE_DIR);
    let _ = std::fs::write(
        format!("{}/exit-code", STATE_DIR),
        if ok { "0" } else { "1" },
    );
}

fn prepare_repo(spec: &str, dest: &Path) -> Result<()> {
    if dest.exists() {
        std::fs::remove_dir_all(dest).ok();
    }

    let embedded = Path::new("/etc/keel/repo");
    if embedded.exists() {
        let src = std::fs::canonicalize(embedded).unwrap_or_else(|_| embedded.to_path_buf());
        return copy_dir(&src, dest);
    }

    if let Some(slug) = spec.strip_prefix("github:") {
        git_clone(&format!("https://github.com/{slug}.git"), dest)?;
        checkout_iso_rev(dest);
        Ok(())
    } else if spec.contains("://") || spec.starts_with("git@") {
        git_clone(spec, dest)?;
        checkout_iso_rev(dest);
        Ok(())
    } else {
        let src = PathBuf::from(spec);
        if !src.exists() {
            bail!("no existe el repositorio {spec}");
        }
        copy_dir(&src, dest)
    }
}

fn copy_dir(src: &Path, dest: &Path) -> Result<()> {
    let status = Command::new("cp").arg("-a").arg(src).arg(dest).status()?;
    if !status.success() {
        bail!("no se pudo copiar {}", src.display());
    }
    Ok(())
}

fn checkout_iso_rev(repo: &Path) {
    let Ok(rev) = nixcore::rebuild::release_commit() else {
        return;
    };
    let safe = format!("safe.directory={}", repo.display());
    let _ = Command::new("git")
        .args(["-c", safe.as_str(), "-C"])
        .arg(repo)
        .args(["fetch", "--depth", "1", "origin", &rev])
        .status();
    let _ = Command::new("git")
        .args(["-c", safe.as_str(), "-C"])
        .arg(repo)
        .args(["checkout", &rev])
        .status();
}

fn git_clone(url: &str, dest: &Path) -> Result<()> {
    let status = Command::new("git")
        .args(["clone", "--depth", "1"])
        .arg(url)
        .arg(dest)
        .status()?;
    if !status.success() {
        bail!("git clone de {url} falló");
    }
    Ok(())
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
