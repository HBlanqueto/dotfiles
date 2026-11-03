use color_eyre::eyre::{Context, Result};
use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Output, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::Duration;

const IDLE_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Clone)]
pub struct Cmd {
    pub program: String,
    pub args: Vec<String>,
    pub display: String,
    pub idle_timeout: Option<Duration>,
}

impl Cmd {
    pub fn new(program: impl Into<String>) -> Self {
        let program = program.into();
        Self {
            display: program.clone(),
            program,
            args: Vec::new(),
            idle_timeout: Some(IDLE_TIMEOUT),
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        let arg = arg.into();
        self.display.push(' ');
        self.display.push_str(&arg);
        self.args.push(arg);
        self
    }

    pub fn arg_path(self, path: impl AsRef<std::path::Path>) -> Self {
        self.arg(path.as_ref().to_string_lossy().to_string())
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for arg in args {
            self = self.arg(arg);
        }
        self
    }

    pub fn without_idle_timeout(mut self) -> Self {
        self.idle_timeout = None;
        self
    }

    pub fn with_display(mut self, display: impl Into<String>) -> Self {
        self.display = display.into();
        self
    }
}

#[derive(Debug, Clone)]
pub enum Stream {
    Stdout(String),
    Stderr(String),
}

pub fn nix_config() -> String {
    let ours = "experimental-features = nix-command flakes";
    match std::env::var("NIX_CONFIG") {
        Ok(existing) if !existing.trim().is_empty() => format!("{existing}\n{ours}"),
        _ => ours.to_string(),
    }
}

pub fn run(cmd: &Cmd, sink: &mut dyn FnMut(Stream)) -> i32 {
    let mut command = Command::new(&cmd.program);
    command
        .args(&cmd.args)
        .env("NIX_CONFIG", nix_config())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(err) => {
            sink(Stream::Stderr(format!(
                "no se pudo ejecutar {}: {err}",
                cmd.program
            )));
            return 127;
        }
    };

    let (tx, rx) = mpsc::channel::<(bool, String)>();
    let mut readers = Vec::new();
    if let Some(stdout) = child.stdout.take() {
        let tx = tx.clone();
        readers.push(thread::spawn(move || read_lines(stdout, false, &tx)));
    }
    if let Some(stderr) = child.stderr.take() {
        let tx = tx.clone();
        readers.push(thread::spawn(move || read_lines(stderr, true, &tx)));
    }
    drop(tx);

    let mut timed_out = false;
    loop {
        let received = match cmd.idle_timeout {
            Some(timeout) => rx.recv_timeout(timeout),
            None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        match received {
            Ok((is_stderr, line)) => {
                if is_stderr {
                    sink(Stream::Stderr(line));
                } else {
                    sink(Stream::Stdout(line));
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                timed_out = true;
                break;
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }

    if timed_out {
        let _ = child.kill();
        sink(Stream::Stderr(format!(
            "{} no produjo salida en {} s; se cancela el comando",
            cmd.program,
            IDLE_TIMEOUT.as_secs()
        )));
    }

    for reader in readers {
        let _ = reader.join();
    }

    if timed_out {
        let _ = child.wait();
        return -2;
    }

    match child.wait() {
        Ok(status) => status.code().unwrap_or(-1),
        Err(err) => {
            sink(Stream::Stderr(format!(
                "error esperando a {}: {err}",
                cmd.program
            )));
            -1
        }
    }
}

fn read_lines<R: Read>(reader: R, is_stderr: bool, tx: &mpsc::Sender<(bool, String)>) {
    for line in BufReader::new(reader).lines().map_while(Result::ok) {
        if tx.send((is_stderr, line)).is_err() {
            break;
        }
    }
}

pub fn capture(cmd: &Cmd) -> Result<Output> {
    Command::new(&cmd.program)
        .args(&cmd.args)
        .env("NIX_CONFIG", nix_config())
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("no se pudo ejecutar {}", cmd.program))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_is_shell_like() {
        let cmd = Cmd::new("disko")
            .arg("--mode")
            .arg("destroy,format,mount")
            .arg("disko.nix");
        assert_eq!(cmd.display, "disko --mode destroy,format,mount disko.nix");
        assert_eq!(cmd.args.len(), 3);
    }

    #[test]
    fn capture_reads_stdout() {
        let out = capture(&Cmd::new("sh").arg("-c").arg("echo hola")).unwrap();
        assert!(out.status.success());
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "hola");
    }

    #[test]
    fn run_streams_and_returns_code() {
        let mut lines = Vec::new();
        let code = run(
            &Cmd::new("sh")
                .arg("-c")
                .arg("echo uno; echo dos >&2; exit 3"),
            &mut |out| match out {
                Stream::Stdout(l) => lines.push(format!("out:{l}")),
                Stream::Stderr(l) => lines.push(format!("err:{l}")),
            },
        );
        assert_eq!(code, 3);
        assert!(lines.iter().any(|l| l == "out:uno"));
        assert!(lines.iter().any(|l| l == "err:dos"));
    }
}
