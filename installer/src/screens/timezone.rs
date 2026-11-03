use crate::app::{Action, App, PickItem};
use crate::install::shell_quote;
use crate::widgets::Btn;
use ratatui::crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;
use std::process::{Command, Stdio};

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let Some(picker) = &app.timezones else {
        return;
    };
    let list = super::picker::draw_search(
        f,
        &app.palette,
        area,
        picker,
        !app.focus_buttons && !app.no_focus(),
        app.blink(),
    );
    super::picker::draw(
        f,
        &app.palette,
        list,
        picker,
        app.list_focus && !app.focus_buttons && !app.no_focus(),
    );
}

pub fn key(app: &mut App, key: KeyEvent) -> bool {
    super::picker::update_list_focus(app, &key);
    let Some(picker) = app.timezones.as_mut() else {
        return false;
    };
    let consumed = super::picker::key(picker, key);
    if consumed {
        if let Some(item) = picker.current() {
            app.settings.time_zone = item.code.clone();
        }
    }
    consumed
}

pub fn buttons(_app: &App) -> Vec<(Btn, Action)> {
    vec![
        (Btn::new("Atrás"), Action::Back),
        (Btn::new("Continuar"), Action::Next),
    ]
}

pub fn commands(app: &App) -> Vec<String> {
    vec![format!(
        "machine.toml: timeZone = \"{}\";",
        app.settings.time_zone
    )]
}

pub fn load_items() -> Vec<PickItem> {
    let zones = localekit::timezone::zones();
    let times = zone_times(&zones);
    zones
        .into_iter()
        .map(|zone| {
            let time = times
                .iter()
                .find(|(z, _)| z == &zone)
                .map(|(_, t)| t.clone())
                .unwrap_or_default();
            PickItem {
                code: zone,
                name: String::new(),
                extra: time,
            }
        })
        .collect()
}

fn zone_times(zones: &[String]) -> Vec<(String, String)> {
    let script = zones
        .iter()
        .map(|zone| {
            format!(
                "printf '%s\\t%s\\n' {z} \"$(TZ={z} date +%H:%M)\"",
                z = shell_quote(zone)
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    let Ok(output) = Command::new("sh")
        .arg("-c")
        .arg(&script)
        .stdin(Stdio::null())
        .output()
    else {
        return Vec::new();
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let (zone, time) = line.split_once('\t')?;
            Some((zone.to_string(), time.to_string()))
        })
        .collect()
}
