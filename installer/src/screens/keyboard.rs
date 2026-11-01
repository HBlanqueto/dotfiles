//! Keyboard layout screen.
use crate::app::{Action, App};
use crate::widgets::{self, Btn};
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use tui_input::backend::crossterm::EventHandler;

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let Some(picker) = &app.keyboards else {
        return;
    };
    let chunks = Layout::vertical([
        Constraint::Length(4),
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
        Constraint::Length(4),
    ])
    .split(area);
    super::picker::search_box(
        f,
        &app.palette,
        chunks[0],
        picker,
        !app.focus_buttons && !app.key_test_focus && !app.no_focus(),
        app.blink(),
    );
    super::picker::draw_centered(
        f,
        &app.palette,
        chunks[2],
        picker,
        app.keyboard_suggested,
        app.list_focus && !app.focus_buttons && !app.no_focus(),
    );
    let test = widgets::text_field(
        &app.palette,
        "Prueba el teclado",
        app.key_test.value(),
        app.key_test.visual_cursor(),
        app.key_test_focus && !app.focus_buttons && !app.no_focus(),
        app.blink(),
        false,
        "ñ ¿ ¡ @ # á é",
        area.width,
        None,
        false,
    );
    f.render_widget(Paragraph::new(test), chunks[4]);
}

pub fn key(app: &mut App, key: KeyEvent) -> bool {
    if app.key_test_focus {
        return match key.code {
            KeyCode::Esc => {
                app.key_test_focus = false;
                true
            }
            KeyCode::Tab => {
                app.key_test_focus = false;
                app.focus_buttons_on_primary();
                true
            }
            _ if crate::screens::is_edit_key(&key) => {
                app.key_test.handle_event(&Event::Key(key));
                true
            }
            _ => false,
        };
    }
    super::picker::update_list_focus(app, &key);
    match key.code {
        KeyCode::Tab => {
            app.key_test_focus = true;
            true
        }
        KeyCode::Enter => {
            if app.focus_buttons {
                return false;
            }
            if let Some(index) = app.keyboards.as_ref().and_then(|p| p.selected) {
                app.choose_keyboard(index);
                app.focus_buttons_on_primary();
                true
            } else {
                false
            }
        }
        _ => app
            .keyboards
            .as_mut()
            .map(|picker| super::picker::key(picker, key))
            .unwrap_or(false),
    }
}

pub fn buttons(_app: &App) -> Vec<(Btn, Action)> {
    vec![
        (Btn::new("Atrás"), Action::Back),
        (Btn::new("Continuar"), Action::Next),
    ]
}

pub fn commands(app: &App) -> Vec<String> {
    vec![
        format!("loadkeys {}", app.keyboard_layout),
        format!(
            "machine.toml: keyboardLayout = \"{}\";",
            app.keyboard_layout
        ),
    ]
}
