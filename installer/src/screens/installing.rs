use crate::app::App;
use crate::install::Step;
use crate::widgets::Btn;
use ratatui::crossterm::event::{KeyCode, KeyEvent};

fn step_weight(step: Step) -> u64 {
    match step {
        Step::Disk => 5,
        Step::Settings => 5,
        Step::System => 80,
        Step::Boot => 10,
    }
}

pub fn overall_percent(app: &App) -> u64 {
    let iv = &app.install;
    let mut p = 0u64;
    for s in Step::ALL {
        if iv.done.contains(&s) {
            p += step_weight(s);
        } else if iv.current == Some(s) && s == Step::System && iv.total > 0 {
            p += step_weight(s) * iv.built.min(iv.total) / iv.total;
        }
    }
    p.min(100)
}

pub fn key(app: &mut App, key: KeyEvent) -> bool {
    if app.install_cancel {
        return match key.code {
            KeyCode::Esc => {
                app.install_cancel = false;
                app.focus_buttons = false;
                true
            }
            KeyCode::Char('v') => {
                app.install_log_view = !app.install_log_view;
                app.install_follow = true;
                true
            }
            _ => false,
        };
    }
    let page = 5usize;
    match key.code {
        KeyCode::Char('v') => {
            app.install_log_view = !app.install_log_view;
            app.install_follow = true;
            true
        }
        KeyCode::Char('c') => {
            app.install_commands = !app.install_commands;
            app.install_follow = true;
            true
        }
        KeyCode::Up => {
            app.install_follow = false;
            app.install_log_scroll = app.install_log_scroll.saturating_sub(1);
            true
        }
        KeyCode::PageUp => {
            app.install_follow = false;
            app.install_log_scroll = app.install_log_scroll.saturating_sub(page);
            true
        }
        KeyCode::Down => {
            app.install_log_scroll = (app.install_log_scroll + 1).min(app.install_max_scroll);
            app.install_follow = app.install_log_scroll >= app.install_max_scroll;
            true
        }
        KeyCode::PageDown => {
            app.install_log_scroll = (app.install_log_scroll + page).min(app.install_max_scroll);
            app.install_follow = app.install_log_scroll >= app.install_max_scroll;
            true
        }
        KeyCode::Home => {
            app.install_follow = false;
            app.install_log_scroll = 0;
            true
        }
        KeyCode::End => {
            app.install_follow = true;
            true
        }
        KeyCode::Esc => {
            if app.install.error.is_none() {
                app.install_cancel = true;
                app.focus_buttons = true;
                app.button_focus = 0;
            }
            true
        }
        _ => false,
    }
}

pub fn buttons(_app: &App) -> Vec<(Btn, crate::app::Action)> {
    Vec::new()
}

pub fn commands(_app: &App) -> Vec<String> {
    Vec::new()
}
