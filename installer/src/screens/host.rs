use crate::app::{Action, App};
use crate::widgets::{self, Btn};
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let w = area.width;
    let blink = app.blink();
    let focus = app.host.focus;
    let has_focus = !app.focus_buttons && !app.no_focus();

    let mut lines = vec![readonly_row(app, "Modelo", app.model.clone(), w)];
    lines.push(Line::from(""));

    let generated = app.final_host_name();
    if !app.host.custom {
        lines.push(readonly_row(
            app,
            "Nombre en la red",
            format!("{generated} · generado del modelo"),
            w,
        ));
    }
    lines.push(Line::from(""));

    let check_y = lines.len() as u16;
    lines.push(widgets::checkbox(
        &app.palette,
        app.host.custom,
        "Usar un nombre de red diferente",
        has_focus && focus == 0,
    ));
    lines.push(Line::from(""));
    let mut field_y = 0;
    if app.host.custom {
        let note = Line::from(Span::styled(
            app.host_name_error().unwrap_or("letras, números y -"),
            Style::default().fg(if app.host_name_error().is_some() {
                app.palette.error
            } else {
                app.palette.muted
            }),
        ));
        field_y = lines.len() as u16;
        lines.extend(widgets::text_field(
            &app.palette,
            "Nombre en la red",
            app.host.name.value(),
            app.host.name.visual_cursor(),
            has_focus && focus == 1,
            blink,
            false,
            &generated,
            w,
            Some(note),
            app.host_name_error().is_some(),
        ));
        lines.push(Line::from(Span::styled(
            "se usa como hostname del sistema",
            Style::default().fg(app.palette.muted),
        )));
    }

    let focus_y = match focus {
        0 => check_y,
        1 if app.host.custom => field_y,
        _ => 0,
    };
    let scroll = widgets::scroll_to_show(focus_y, 4, area.height);
    f.render_widget(Paragraph::new(lines.clone()).scroll((scroll, 0)), area);
    widgets::draw_scrollbar(f, &app.palette, area, lines.len(), scroll, area.height);
}

fn readonly_row(app: &App, label: &str, value: String, width: u16) -> Line<'static> {
    let value = if value.is_empty() {
        "—".to_string()
    } else {
        value
    };
    let pad = (width as usize).saturating_sub(label.chars().count() + value.chars().count() + 1);
    Line::from(vec![
        Span::styled(label.to_string(), Style::default().fg(app.palette.muted)),
        Span::raw(" ".repeat(pad)),
        Span::styled(value, Style::default().fg(app.palette.highlight)),
    ])
}

pub fn key(app: &mut App, key: KeyEvent) -> bool {
    if !app.focus_buttons {
        match key.code {
            KeyCode::Char(' ') if app.host.focus == 0 => {
                app.host.custom = !app.host.custom;
                if app.host.custom {
                    app.host.focus = 1;
                } else {
                    app.host.name = Input::default();
                }
                return true;
            }
            KeyCode::Down | KeyCode::Tab | KeyCode::Enter => {
                focus_next(app);
                return true;
            }
            KeyCode::Up | KeyCode::BackTab => {
                focus_prev(app);
                return true;
            }
            _ => {}
        }
    }
    if app.host.custom && app.host.focus == 1 && super::is_edit_key(&key) {
        edit_host_name(app, key);
        return true;
    }
    false
}

fn focus_next(app: &mut App) {
    if app.host.focus == 0 && app.host.custom {
        app.host.focus = 1;
    } else {
        app.focus_buttons_on_primary();
    }
}

fn focus_prev(app: &mut App) {
    if app.host.focus == 1 {
        app.host.focus = 0;
    }
}

fn edit_host_name(app: &mut App, key: KeyEvent) {
    if let KeyCode::Char(c) = key.code {
        if !(c.is_ascii_alphanumeric() || c == '-') {
            return;
        }
    }
    app.host.name.handle_event(&Event::Key(key));
}

pub fn buttons(_app: &App) -> Vec<(Btn, Action)> {
    vec![
        (Btn::new("Atrás"), Action::Back),
        (Btn::new("Continuar"), Action::Next),
    ]
}

pub fn commands(app: &App) -> Vec<String> {
    vec![
        format!("machine.toml: hostName = \"{}\"", app.final_host_name()),
        format!("machine.toml: prettyHostName = \"{}\";", app.model),
    ]
}
