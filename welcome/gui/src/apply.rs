//! Runs the `nixdots-setup apply` CLI and streams its output.
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;

fn write_json(json: &str) -> std::io::Result<PathBuf> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("keel-setup-gui-{}-{nanos}", std::process::id()));
    std::fs::create_dir(&dir)?;
    set_mode(&dir, 0o700)?;
    let path = dir.join("datos.json");
    std::fs::write(&path, json)?;
    set_mode(&path, 0o600)?;
    Ok(path)
}

fn set_mode(path: &std::path::Path, mode: u32) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
}

fn cli() -> std::ffi::OsString {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let sibling = dir.join("nixdots-setup");
            if sibling.exists() {
                return sibling.into_os_string();
            }
        }
    }
    "nixdots-setup".into()
}

fn pump<R: Read + Send + 'static>(
    stream: R,
    on_line: Arc<dyn Fn(String) + Send + Sync>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let reader = BufReader::new(stream);
        for line in reader.lines().map_while(Result::ok) {
            on_line(line);
        }
    })
}

pub fn start<F, D>(
    json: &str,
    repo: &str,
    dry_run: bool,
    password: &str,
    on_line: F,
    on_done: D,
) -> Result<(), String>
where
    F: Fn(String) + Send + Sync + 'static,
    D: Fn(bool) + Send + 'static,
{
    let path = write_json(json).map_err(|e| format!("no se pudo escribir los datos: {e}"))?;

    let mut cmd = Command::new(cli());
    cmd.env("NO_COLOR", "1");
    cmd.arg("apply").arg(&path);
    if !repo.is_empty() {
        cmd.arg("--repo").arg(repo);
    }
    if dry_run {
        cmd.arg("--dry-run");
    }
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("no se pudo ejecutar el CLI: {e}"))?;

    {
        let mut stdin = child.stdin.take().ok_or("sin stdin del CLI")?;
        stdin
            .write_all(password.as_bytes())
            .map_err(|e| e.to_string())?;
    }

    let on_line: Arc<dyn Fn(String) + Send + Sync> = Arc::new(on_line);
    let out = child.stdout.take().map(|o| pump(o, on_line.clone()));
    let err = child.stderr.take().map(|e| pump(e, on_line.clone()));

    std::thread::spawn(move || {
        let ok = child.wait().map(|s| s.success()).unwrap_or(false);
        if let Some(t) = out {
            let _ = t.join();
        }
        if let Some(t) = err {
            let _ = t.join();
        }
        on_done(ok);
    });
    Ok(())
}
