use crate::app::{Action, App, RedFase};
use crate::widgets::{self, Btn};
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;

const SPINNER: [&str; 8] = ["⢿", "⣻", "⣽", "⣾", "⣷", "⣯", "⣟", "⡿"];

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    match &app.net.phase {
        RedFase::Lista => {
            let focused = app.list_focus && !app.focus_buttons && !app.no_focus();
            let inner = list_frame(f, app, area, focused);
            let lines = list_lines(app, inner.width, inner.height as usize, focused);
            f.render_widget(Paragraph::new(lines), inner);
        }
        RedFase::Contrasena {
            ssid,
            input,
            visible,
        } => {
            let chunks = Layout::vertical([
                Constraint::Min(0),
                Constraint::Length(1),
                Constraint::Length(4),
            ])
            .split(area);
            let focused = app.list_focus && !app.focus_buttons && !app.no_focus();
            let inner = list_frame(f, app, chunks[0], focused);
            let rows = inner.height as usize;
            f.render_widget(
                Paragraph::new(list_lines(app, inner.width, rows, focused)),
                inner,
            );
            f.render_widget(
                Paragraph::new(Span::styled(
                    "─".repeat(area.width as usize),
                    Style::default().fg(app.palette.rule),
                )),
                chunks[1],
            );
            let field = widgets::text_field(
                &app.palette,
                &format!("Contraseña de {ssid}"),
                input.value(),
                input.visual_cursor(),
                !app.focus_buttons && !app.no_focus(),
                app.blink(),
                !*visible,
                "",
                area.width,
                None,
                false,
            );
            f.render_widget(Paragraph::new(field), chunks[2]);
        }
        RedFase::Conectando { ssid } => {
            let spinner = SPINNER[(app.tick as usize / 2) % SPINNER.len()];
            let line = Line::from(vec![
                Span::styled(
                    format!("{spinner} "),
                    Style::default().fg(app.palette.focus),
                ),
                Span::styled(
                    format!("conectando a {ssid}…"),
                    Style::default().fg(app.palette.text),
                ),
            ]);
            f.render_widget(Paragraph::new(line), area);
        }
        RedFase::Error { msg, .. } => {
            let lines = vec![
                Line::from(vec![
                    Span::styled("✗ ", Style::default().fg(app.palette.error)),
                    Span::styled(msg.clone(), Style::default().fg(app.palette.error)),
                ]),
                Line::from(Span::styled(
                    "enter reintentar · esc volver a redes",
                    Style::default().fg(app.palette.muted),
                )),
            ];
            f.render_widget(Paragraph::new(lines), area);
        }
        RedFase::Conectado { ssid } => {
            let head = if ssid == "ethernet" {
                "ethernet".to_string()
            } else {
                format!("conectado a {ssid}")
            };
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("✓ ", Style::default().fg(app.palette.success)),
                    Span::styled(head, Style::default().fg(app.palette.text)),
                ])),
                area,
            );
        }
    }
}

fn list_frame(f: &mut Frame, app: &App, area: Rect, focused: bool) -> Rect {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(if focused {
            app.palette.primary
        } else {
            app.palette.rule
        }));
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block, area);
    inner
}

fn list_lines(app: &App, width: u16, rows: usize, focused: bool) -> Vec<Line<'static>> {
    if app.net.wifi.is_empty() {
        let msg = if app.net.backend.name() == "ninguno" {
            "no hay backend de red (nmcli/iwctl)"
        } else {
            "no se encontraron redes · pulsa r"
        };
        return vec![Line::from(Span::styled(
            msg,
            Style::default().fg(app.palette.warning),
        ))];
    }
    let total = app.net.wifi.len();
    let rows = rows.max(1);
    let start = app
        .net
        .selected
        .saturating_sub(rows / 2)
        .min(total.saturating_sub(rows));
    (start..(start + rows).min(total))
        .map(|i| wifi_row(app, i, width, focused))
        .collect()
}

fn wifi_row(app: &App, i: usize, width: u16, focused: bool) -> Line<'static> {
    let wifi = &app.net.wifi[i];
    let selected = i == app.net.selected;
    let width = width as usize;
    let name_w = width.saturating_sub(14);
    let name: String = wifi.ssid.chars().take(name_w).collect();
    let level = match wifi.signal {
        76..=100 => 4,
        51..=75 => 3,
        26..=50 => 2,
        _ => 1,
    };
    let accent = if focused {
        app.palette.primary
    } else {
        app.palette.highlight
    };
    let name_style = if selected {
        Style::default()
            .fg(accent)
            .add_modifier(ratatui::style::Modifier::BOLD)
    } else {
        Style::default().fg(app.palette.text)
    };
    let sec_style = if selected {
        Style::default().fg(accent)
    } else {
        Style::default().fg(app.palette.muted)
    };
    let bright = if selected {
        accent
    } else {
        app.palette.secondary
    };
    let mut spans = vec![
        Span::styled(format!("{name:<name_w$}"), name_style),
        Span::styled(format!("{:<10}", wifi.security()), sec_style),
    ];
    for (k, ch) in ["▂", "▄", "▆", "█"].iter().enumerate() {
        let color = if k < level {
            bright
        } else {
            app.palette.disabled
        };
        spans.push(Span::styled((*ch).to_string(), Style::default().fg(color)));
    }
    let line = Line::from(spans);
    if selected {
        line.style(Style::default().bg(app.palette.well))
    } else {
        line
    }
}

pub fn key(app: &mut App, key: KeyEvent) -> bool {
    match &mut app.net.phase {
        RedFase::Lista => match key.code {
            KeyCode::Up => {
                if app.net.selected > 0 {
                    app.net.selected -= 1;
                }
                true
            }
            KeyCode::Down => {
                if app.net.selected + 1 < app.net.wifi.len() {
                    app.net.selected += 1;
                }
                true
            }
            KeyCode::Home => {
                app.net.selected = 0;
                true
            }
            KeyCode::End => {
                app.net.selected = app.net.wifi.len().saturating_sub(1);
                true
            }
            KeyCode::Char('r') => {
                rescan(app);
                true
            }
            KeyCode::Enter => {
                choose(app);
                true
            }
            _ => false,
        },
        RedFase::Contrasena { visible, .. } => match key.code {
            KeyCode::Enter => {
                connect(app);
                true
            }
            KeyCode::Esc => {
                to_menu(app);
                true
            }
            KeyCode::Tab => {
                *visible = !*visible;
                true
            }
            _ if super::is_edit_key(&key) => {
                if let RedFase::Contrasena { input, .. } = &mut app.net.phase {
                    input.handle_event(&Event::Key(key));
                }
                true
            }
            _ => false,
        },
        RedFase::Error { .. } => match key.code {
            KeyCode::Enter => {
                retry(app);
                true
            }
            KeyCode::Esc => {
                to_menu(app);
                true
            }
            _ => false,
        },
        RedFase::Conectando { .. } | RedFase::Conectado { .. } => false,
    }
}

fn choose(app: &mut App) {
    let Some(wifi) = app.net.wifi.get(app.net.selected).cloned() else {
        return;
    };
    if wifi.secured && !app.net.backend.is_saved(&wifi.ssid) {
        app.net.phase = RedFase::Contrasena {
            ssid: wifi.ssid,
            input: Input::default(),
            visible: false,
        };
        app.list_focus = false;
        app.focus_primary();
    } else {
        app.start_net_connect(wifi.ssid.clone(), String::new());
    }
}

pub fn connect(app: &mut App) {
    if let RedFase::Contrasena { ssid, input, .. } = &app.net.phase {
        let ssid = ssid.clone();
        let pass = input.value().to_string();
        if pass.is_empty() {
            return;
        }
        app.start_net_connect(ssid, pass);
    }
}

pub fn rescan(app: &mut App) {
    app.refresh_network();
}

pub fn to_menu(app: &mut App) {
    app.net.phase = RedFase::Lista;
    app.list_focus = true;
    app.focus_primary();
    if app.net.selected >= app.net.wifi.len() {
        app.net.selected = 0;
    }
}

pub fn retry(app: &mut App) {
    if let RedFase::Error { ssid, .. } = &app.net.phase {
        app.net.phase = RedFase::Contrasena {
            ssid: ssid.clone(),
            input: Input::default(),
            visible: false,
        };
        app.focus_primary();
    }
}

pub fn buttons(app: &App) -> Vec<(Btn, Action)> {
    match &app.net.phase {
        RedFase::Lista => vec![(Btn::new("Atrás"), Action::Back)],
        RedFase::Contrasena { .. } => vec![
            (Btn::new("Redes"), Action::NetMenu),
            (Btn::new("Conectar"), Action::Connect),
        ],
        RedFase::Conectando { .. } => Vec::new(),
        RedFase::Error { .. } => vec![
            (Btn::new("Redes"), Action::NetMenu),
            (Btn::new("Reintentar"), Action::NetRetry),
        ],
        RedFase::Conectado { .. } => vec![
            (Btn::new("Cambiar red"), Action::NetMenu),
            (Btn::new("Continuar"), Action::Next),
        ],
    }
}

pub fn commands(app: &App) -> Vec<String> {
    let backend = app.net.backend.name();
    match &app.net.phase {
        RedFase::Conectado { ssid } if ssid == "ethernet" => {
            vec!["conectar a cache.nixos.org:443".into()]
        }
        RedFase::Lista => vec![format!("{backend} device wifi list")],
        RedFase::Contrasena { ssid, .. } | RedFase::Conectando { ssid } => vec![format!(
            "{backend} device wifi connect {ssid} password ••••••"
        )],
        RedFase::Error { ssid, .. } => vec![format!("{backend} device wifi connect {ssid}")],
        RedFase::Conectado { ssid } => vec![
            format!("{backend}: {ssid}"),
            "conectar a cache.nixos.org:443".into(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Screen;
    use crate::net::Wifi;
    use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    fn app() -> App {
        let mut app = App::new(
            true,
            "test".into(),
            PathBuf::from("/tmp/keel-does-not-exist"),
        );
        app.screen = Screen::Network;
        app.net.phase = RedFase::Lista;
        app.net.wifi = vec![
            Wifi {
                ssid: "WPA-Net".into(),
                signal: 80,
                secured: true,
            },
            Wifi {
                ssid: "Abierta".into(),
                signal: 40,
                secured: false,
            },
        ];
        app.net.selected = 0;
        app
    }

    fn press(app: &mut App, code: KeyCode) {
        app.handle_event(Event::Key(KeyEvent::new(code, KeyModifiers::NONE)));
    }

    fn wait_for_reply(app: &mut App) {
        let start = Instant::now();
        while matches!(app.net.phase, RedFase::Conectando { .. })
            && start.elapsed() < Duration::from_secs(5)
        {
            app.poll_network();
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn enter_on_wpa_opens_password() {
        let mut app = app();
        press(&mut app, KeyCode::Enter);
        assert!(
            matches!(app.net.phase, RedFase::Contrasena { .. }),
            "Enter en una red WPA debe abrir Contrasena"
        );
    }

    #[test]
    fn typing_q_in_password_does_not_quit() {
        let mut app = app();
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char('q'));
        press(&mut app, KeyCode::Char('v'));
        assert!(!app.should_quit, "q no debe cerrar mientras se escribe");
        match &app.net.phase {
            RedFase::Contrasena { input, .. } => assert_eq!(input.value(), "qv"),
            _ => panic!("debería seguir en Contrasena"),
        }
    }

    #[test]
    fn esc_returns_to_list() {
        let mut app = app();
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Esc);
        assert!(matches!(app.net.phase, RedFase::Lista));
    }

    #[test]
    fn tab_toggles_password_visibility() {
        let mut app = app();
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Tab);
        match &app.net.phase {
            RedFase::Contrasena { visible, .. } => assert!(*visible),
            _ => panic!("debería seguir en Contrasena"),
        }
    }

    #[test]
    fn short_password_in_simulation_errors() {
        let mut app = app();
        press(&mut app, KeyCode::Enter);
        for ch in "corta".chars() {
            press(&mut app, KeyCode::Char(ch));
        }
        press(&mut app, KeyCode::Enter);
        assert!(matches!(app.net.phase, RedFase::Conectando { .. }));
        wait_for_reply(&mut app);
        assert!(
            matches!(app.net.phase, RedFase::Error { .. }),
            "una contraseña corta debe llevar a Error"
        );
    }

    #[test]
    fn long_password_in_simulation_connects() {
        let mut app = app();
        press(&mut app, KeyCode::Enter);
        for ch in "contrasena-larga".chars() {
            press(&mut app, KeyCode::Char(ch));
        }
        press(&mut app, KeyCode::Enter);
        wait_for_reply(&mut app);
        assert!(matches!(app.net.phase, RedFase::Conectado { .. }));
    }

    #[test]
    fn only_connected_allows_advance() {
        let mut app = app();
        assert!(!app.primary_enabled(), "Lista no debe permitir avanzar");
        app.net.phase = RedFase::Conectado {
            ssid: "WPA-Net".into(),
        };
        assert!(app.primary_enabled(), "Conectado debe permitir avanzar");
    }
}
