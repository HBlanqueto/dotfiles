use crate::app::{Action, App};
use crate::widgets::Btn;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use std::path::Path;
use std::process::{Command, Stdio};

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let y = area.y + area.height / 2;
    if y > area.y {
        f.render_widget(
            Paragraph::new(Span::styled(
                "Al reiniciar se configurará tu cuenta.",
                Style::default().fg(app.palette.muted),
            ))
            .alignment(Alignment::Center),
            Rect::new(area.x, y.saturating_sub(2), area.width, 1),
        );
    }
    app.draw_buttons(f, Rect::new(area.x, y, area.width, 1));
}

pub fn key(app: &mut App, key: KeyEvent) -> bool {
    if key.code == KeyCode::Char('q') {
        salir(app);
        return true;
    }
    false
}

pub fn buttons(_app: &App) -> Vec<(Btn, Action)> {
    vec![
        (Btn::new("Salir"), Action::Finish),
        (Btn::new("Reiniciar"), Action::Reboot),
    ]
}

fn copy_log(app: &App) {
    if app.dry_run {
        return;
    }
    if let Some(path) = &app.install.log_path {
        let dir = Path::new("/mnt/persist");
        if dir.is_dir() {
            let _ = std::fs::copy(path, dir.join("system-install.log"));
        }
    }
}

fn umount() {
    let _ = Command::new("umount")
        .arg("-R")
        .arg("/mnt")
        .stdin(Stdio::null())
        .status();
}

pub fn restart(app: &mut App) {
    copy_log(app);
    if app.dry_run {
        app.should_quit = true;
        return;
    }
    umount();
    let _ = Command::new("reboot").stdin(Stdio::null()).status();
    app.should_quit = true;
}

pub fn salir(app: &mut App) {
    copy_log(app);
    if !app.dry_run {
        umount();
    }
    app.should_quit = true;
}

pub fn commands(_app: &App) -> Vec<String> {
    Vec::new()
}
