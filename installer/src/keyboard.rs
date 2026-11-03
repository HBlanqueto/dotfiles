use std::process::{Command, Stdio};

pub use localekit::keyboard::{all, by_id, display, suggest, Layout};

pub fn detect_active() -> String {
    if let Ok(output) = Command::new("localectl")
        .arg("status")
        .stdin(Stdio::null())
        .output()
    {
        let text = String::from_utf8_lossy(&output.stdout);
        let mut xkb = String::new();
        let mut variant = String::new();
        for line in text.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("X11 Layout:") {
                xkb = rest.trim().to_string();
            } else if let Some(rest) = line.strip_prefix("X11 Variant:") {
                variant = rest.trim().to_string();
            }
        }
        if !xkb.is_empty() {
            let full = if variant.is_empty() {
                xkb.clone()
            } else {
                format!("{xkb}:{variant}")
            };
            if all().iter().any(|l| l.code == full) {
                return full;
            }
            if all().iter().any(|l| l.code == xkb) {
                return xkb;
            }
        }
    }
    "us".to_string()
}

pub fn apply_console(layout: &Layout) -> ConsoleOutcome {
    match console_keymap(layout) {
        Some(keymap) => {
            if run_loadkeys(&keymap) {
                ConsoleOutcome::Applied
            } else {
                ConsoleOutcome::Failed
            }
        }
        None => {
            let _ = run_loadkeys("us");
            ConsoleOutcome::Fallback
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleOutcome {
    Applied,
    Fallback,
    Failed,
}

fn run_loadkeys(keymap: &str) -> bool {
    let cmd = crate::runner::Cmd::new("loadkeys").arg(keymap);
    let mut sink = |_out: crate::runner::Stream| {};
    crate::runner::run(&cmd, &mut sink) == 0
}

fn console_keymap(layout: &Layout) -> Option<String> {
    Some(
        match (layout.xkb.as_str(), layout.variant.as_str()) {
            ("us", "") => "us",
            ("us", "intl") => "us-acentos",
            ("latam", "") => "la-latin1",
            ("es", "") => "es",
            ("gb", "") => "uk",
            ("br", "") => "br-abnt2",
            ("fr", "") => "fr",
            ("de", "") => "de",
            ("it", "") => "it",
            ("pt", "") => "pt-latin1",
            ("ru", "") => "ru",
            ("pl", "") => "pl",
            ("nl", "") => "nl",
            ("jp", "") => "jp106",
            _ => return None,
        }
        .to_string(),
    )
}
