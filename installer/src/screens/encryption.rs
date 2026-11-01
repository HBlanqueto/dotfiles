//! Encryption screen: LUKS passphrase, recovery key and TPM unlock.
use crate::app::{Action, App, ENC_CONFIRM, ENC_NOTED, ENC_PASS, ENC_TPM};
use crate::runner::{self, Cmd};
use crate::widgets::{self, Btn};
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;
use tui_input::backend::crossterm::EventHandler;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let w = area.width;
    let blink = app.blink();
    let has_focus = !app.focus_buttons && !app.no_focus();
    let mut lines: Vec<Line> = Vec::new();
    let mut slots: Vec<(usize, u16)> = Vec::new();

    slots.push((ENC_PASS, lines.len() as u16));
    lines.extend(widgets::text_field(
        &app.palette,
        "Frase de desbloqueo",
        app.enc.pass.value(),
        app.enc.pass.visual_cursor(),
        has_focus && app.enc.focus == ENC_PASS,
        blink,
        true,
        "",
        w,
        if app.enc.pass.value().is_empty() {
            None
        } else {
            Some(strength_note(app))
        },
        false,
    ));
    lines.push(Line::from(""));

    slots.push((ENC_CONFIRM, lines.len() as u16));
    let note = if app.enc.confirm.value().is_empty() {
        None
    } else if app.enc.passphrase_matches() {
        Some(Line::from(Span::styled(
            "✓ coinciden",
            Style::default().fg(app.palette.success),
        )))
    } else {
        Some(Line::from(Span::styled(
            "no coinciden",
            Style::default().fg(app.palette.error),
        )))
    };
    lines.extend(widgets::text_field(
        &app.palette,
        "Confirmar",
        app.enc.confirm.value(),
        app.enc.confirm.visual_cursor(),
        has_focus && app.enc.focus == ENC_CONFIRM,
        blink,
        true,
        "",
        w,
        note,
        !app.enc.confirm.value().is_empty() && !app.enc.passphrase_matches(),
    ));
    lines.push(Line::from(""));

    let id = crate::keyboard::display(
        &app.settings.keyboard_layout,
        &app.settings.keyboard_variant,
    );
    lines.push(Line::from(Span::styled(
        format!("Se escribe al encender, con teclado {id}."),
        Style::default().fg(app.palette.warning),
    )));
    if pass_has_non_ascii(app) {
        lines.push(Line::from(Span::styled(
            format!(
                "! contiene caracteres especiales: asegúrate de que el teclado al arrancar sea {id}"
            ),
            Style::default().fg(app.palette.warning),
        )));
    }
    lines.push(Line::from(""));

    if app.enc.tpm_present {
        slots.push((ENC_TPM, lines.len() as u16));
        lines.push(widgets::checkbox(
            &app.palette,
            app.enc.tpm,
            "Desbloquear con TPM",
            has_focus && app.enc.focus == ENC_TPM,
        ));
        lines.push(Line::from(Span::styled(
            "TPM 2.0 detectado",
            Style::default().fg(app.palette.muted),
        )));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled(
        "clave de recuperación · anótala, no se volverá a mostrar",
        Style::default().fg(app.palette.muted),
    )));
    lines.extend(key_box_lines(app, w));
    let usb = if let Some(dev) = &app.enc.usb_saved {
        format!("u  guardar en USB  ·  guardada en {dev}")
    } else {
        "u  guardar en USB".to_string()
    };
    lines.push(Line::from(Span::styled(
        usb,
        Style::default().fg(app.palette.highlight),
    )));

    slots.push((ENC_NOTED, lines.len() as u16));
    lines.push(widgets::checkbox(
        &app.palette,
        app.enc.noted,
        "La anoté en un lugar seguro",
        has_focus && app.enc.focus == ENC_NOTED,
    ));

    let (_, focus_y) = slots
        .iter()
        .find(|(tag, _)| *tag == app.enc.focus)
        .copied()
        .unwrap_or((0, 0));
    let focus_h = if app.enc.focus == ENC_PASS || app.enc.focus == ENC_CONFIRM {
        4
    } else {
        1
    };
    let scroll = widgets::scroll_to_show(focus_y, focus_h, area.height);
    f.render_widget(Paragraph::new(lines.clone()).scroll((scroll, 0)), area);
    widgets::draw_scrollbar(f, &app.palette, area, lines.len(), scroll, area.height);
}

fn strength_note(app: &App) -> Line<'static> {
    let (filled, label, color) = strength(app.enc.pass.value(), &app.palette);
    Line::from(vec![
        Span::styled("█".repeat(filled), Style::default().fg(color)),
        Span::styled(
            "░".repeat(12 - filled),
            Style::default().fg(app.palette.rule),
        ),
        Span::styled(
            format!(" {label}"),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
    ])
}

fn key_box_lines(app: &App, width: u16) -> Vec<Line<'static>> {
    let key = app.enc.recovery.as_str();
    let w = (key.chars().count() as u16 + 4).min(width);
    let pad = " ".repeat(width.saturating_sub(w) as usize / 2);
    let warning = Style::default().fg(app.palette.warning);
    let border = "─".repeat(w.saturating_sub(2) as usize);
    vec![
        Line::from(vec![
            Span::raw(pad.clone()),
            Span::styled(format!("╭{border}╮"), warning),
        ]),
        Line::from(vec![
            Span::raw(pad.clone()),
            Span::styled("│ ", warning),
            Span::styled(
                key.to_string(),
                Style::default()
                    .fg(app.palette.white)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" │", warning),
        ]),
        Line::from(vec![
            Span::raw(pad),
            Span::styled(format!("╰{border}╯"), warning),
        ]),
    ]
}

fn pass_has_non_ascii(app: &App) -> bool {
    !app.enc.pass.value().is_ascii()
}

fn strength(pass: &str, palette: &crate::theme::Palette) -> (usize, &'static str, Color) {
    let n = pass.chars().count();
    let mut classes = 0;
    if pass.chars().any(|c| c.is_ascii_lowercase()) {
        classes += 1;
    }
    if pass.chars().any(|c| c.is_ascii_uppercase()) {
        classes += 1;
    }
    if pass.chars().any(|c| c.is_ascii_digit()) {
        classes += 1;
    }
    if pass.chars().any(|c| !c.is_ascii_alphanumeric()) {
        classes += 1;
    }
    let score = ((n.min(20) as f32) / 20.0) * 0.6 + (classes as f32 / 4.0) * 0.4;
    let filled = ((score * 12.0).round() as usize).clamp(1, 12);
    if n < 8 || score < 0.45 {
        (filled, "débil", palette.error)
    } else if score < 0.72 {
        (filled, "aceptable", palette.warning)
    } else {
        (filled, "buena", palette.success)
    }
}

pub fn key(app: &mut App, key: KeyEvent) -> bool {
    if !app.focus_buttons {
        match key.code {
            KeyCode::Char(' ') => match app.enc.focus {
                ENC_TPM => {
                    app.enc.tpm = !app.enc.tpm;
                    return true;
                }
                ENC_NOTED => {
                    app.enc.noted = !app.enc.noted;
                    return true;
                }
                _ => {}
            },
            KeyCode::Down | KeyCode::Tab => {
                let targets = app.enc.targets();
                let pos = targets
                    .iter()
                    .position(|&t| t == app.enc.focus)
                    .unwrap_or(0);
                if pos + 1 < targets.len() {
                    app.enc.focus = targets[pos + 1];
                } else {
                    app.focus_buttons_on_primary();
                }
                return true;
            }
            KeyCode::Up | KeyCode::BackTab => {
                let targets = app.enc.targets();
                let pos = targets
                    .iter()
                    .position(|&t| t == app.enc.focus)
                    .unwrap_or(0);
                if pos > 0 {
                    app.enc.focus = targets[pos - 1];
                }
                return true;
            }
            KeyCode::Char('u') | KeyCode::Char('U')
                if app.enc.focus != ENC_PASS && app.enc.focus != ENC_CONFIRM =>
            {
                app.open_usb_modal();
                return true;
            }
            KeyCode::Enter if app.enc.focus == ENC_PASS => {
                app.enc.focus = ENC_CONFIRM;
                return true;
            }
            _ => {}
        }
    }
    if (app.enc.focus == ENC_PASS || app.enc.focus == ENC_CONFIRM) && super::is_edit_key(&key) {
        let target = if app.enc.focus == ENC_PASS {
            &mut app.enc.pass
        } else {
            &mut app.enc.confirm
        };
        target.handle_event(&Event::Key(key));
        return true;
    }
    false
}

pub fn buttons(_app: &App) -> Vec<(Btn, Action)> {
    vec![
        (Btn::new("Atrás"), Action::Back),
        (Btn::new("Continuar"), Action::Next),
    ]
}

pub fn shortcuts(_app: &App) -> Vec<String> {
    vec!["espacio casilla".into(), "↑↓ mover".into()]
}

pub fn commands(app: &App) -> Vec<String> {
    let device = app
        .selected_disk()
        .map(|d| d.path())
        .unwrap_or_else(|| "/dev/xxx".into());
    vec![
        "disko --argstr encrypt true  (luksFormat con la frase por stdin)".into(),
        format!(
            "cryptsetup luksAddKey --key-file /run/... <partición crypto_LUKS de {device}> /run/..."
        ),
        format!("systemd-cryptenroll --tpm2-device=auto --tpm2-pcrs= <partición LUKS de {device}>"),
    ]
}

pub fn usb_key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => app.enc.usb_open = false,
        KeyCode::Up => app.enc.usb_index = app.enc.usb_index.saturating_sub(1),
        KeyCode::Down => {
            if app.enc.usb_index + 1 < app.enc.usb.len() {
                app.enc.usb_index += 1;
            }
        }
        KeyCode::Enter => {
            save_recovery(app);
            app.enc.usb_open = false;
        }
        _ => {}
    }
    true
}

pub fn draw_usb(f: &mut Frame, app: &App, area: Rect) {
    widgets::dim_buffer(f, &app.palette);
    let w = 58.min(area.width);
    let rows = app.enc.usb.len() as u16;
    let h = (rows + 4).clamp(5, area.height.saturating_sub(2));
    let x = area.x + area.width.saturating_sub(w) / 2;
    let y = area.y + area.height.saturating_sub(h) / 2;
    let rect = Rect::new(x, y, w, h);
    f.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.palette.primary))
        .title(Span::styled(
            " Guardar en USB ",
            Style::default().fg(app.palette.primary),
        ));
    let inner = block.inner(rect);
    f.render_widget(block, rect);

    if app.enc.usb.is_empty() {
        f.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled(
                    "no hay medios extraíbles montables",
                    Style::default().fg(app.palette.muted),
                ))
                .alignment(Alignment::Center),
                Line::from(Span::styled(
                    "conecta un USB y vuelve a abrir esta lista",
                    Style::default().fg(app.palette.muted),
                ))
                .alignment(Alignment::Center),
            ]),
            Rect::new(
                inner.x,
                inner.y + 1,
                inner.width,
                inner.height.saturating_sub(2),
            ),
        );
    } else {
        let mut lines = Vec::new();
        for (i, medium) in app.enc.usb.iter().enumerate() {
            let row = i as u16;
            if row >= inner.height.saturating_sub(1) {
                break;
            }
            lines.push(widgets::list_row(
                &app.palette,
                i == app.enc.usb_index,
                format!("{:<12}", medium.path),
                Some(medium.label.clone()),
            ));
        }
        f.render_widget(
            Paragraph::new(lines),
            Rect::new(
                inner.x,
                inner.y,
                inner.width,
                inner.height.saturating_sub(1),
            ),
        );
    }

    f.render_widget(
        Paragraph::new(Span::styled(
            "↑↓ elegir · enter guardar · esc cerrar",
            Style::default().fg(app.palette.muted),
        ))
        .alignment(Alignment::Center),
        Rect::new(inner.x, inner.bottom().saturating_sub(1), inner.width, 1),
    );
}

fn save_recovery(app: &mut App) {
    let Some(medium) = app.enc.usb.get(app.enc.usb_index).cloned() else {
        return;
    };
    let name = format!("keel-recovery-{}.txt", app.settings.host_name);
    let content = format!(
        "Keel · clave de recuperación LUKS2\n\
         hostname: {}\n\
         disco: {}\n\n\
         {}\n\n\
         Anótala en un lugar seguro. Se pedirá al arrancar si el TPM no desbloquea.\n",
        app.settings.host_name,
        app.selected_disk().map(|d| d.path()).unwrap_or_default(),
        app.enc.recovery.as_str(),
    );

    if app.dry_run {
        app.enc.usb_saved = Some(medium.path.clone());
        app.status = Some(format!("simulación: {name} → {}", medium.path));
        return;
    }

    let dir = "/tmp/keel-usb";
    let _ = std::fs::create_dir_all(dir);
    let mount = Cmd::new("mount")
        .arg("-o")
        .arg("rw")
        .arg(&medium.path)
        .arg(dir);
    match runner::capture(&mount) {
        Ok(out) if out.status.success() => {}
        Ok(out) => {
            app.status = Some(format!(
                "no se pudo montar {}: {}",
                medium.path,
                String::from_utf8_lossy(&out.stderr).trim()
            ));
            return;
        }
        Err(err) => {
            app.status = Some(format!("mount: {err}"));
            return;
        }
    }

    let path = format!("{dir}/{name}");
    let write = std::fs::write(&path, content.as_bytes());
    let _ = runner::capture(&Cmd::new("sync"));
    let _ = runner::capture(&Cmd::new("umount").arg(dir));
    match write {
        Ok(()) => {
            app.enc.usb_saved = Some(medium.path.clone());
            app.status = Some(format!("clave guardada en {}", medium.path));
        }
        Err(err) => app.status = Some(format!("no se pudo escribir la clave: {err}")),
    }
}
