//! License screen with the GPL text and accept/decline buttons.
use crate::app::{Action, App};
use crate::widgets::{self, Btn, ButtonRow};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};
use ratatui::Frame;

const LICENSE: &str = include_str!("../../../LICENSE");

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let console = crate::theme::console::is_linux_console();
    let logo = crate::logo::nixos_logo(console);
    let logo_h = logo.len() as u16;
    let logo_w = logo
        .iter()
        .map(|l| {
            l.spans
                .iter()
                .map(|s| s.content.chars().count())
                .sum::<usize>()
        })
        .max()
        .unwrap_or(0) as u16;

    let box_w = 84.min(area.width.saturating_sub(4)).max(24);
    let overhead = logo_h + 3 + 1 + 2 + 1 + 1 + 1 + 1;
    let box_h = area.height.saturating_sub(overhead).clamp(3, 20);
    let total = overhead + box_h;
    let mut y = area.y + area.height.saturating_sub(total) / 2;

    if logo_h > 0 {
        let x = area.x + area.width.saturating_sub(logo_w) / 2;
        f.render_widget(Paragraph::new(logo), Rect::new(x, y, logo_w, logo_h));
        y += logo_h;
    }
    y += 3;

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "Para continuar con la instalación, debes aceptar los términos de la licencia.",
            Style::default().fg(app.palette.muted),
        )))
        .alignment(Alignment::Center),
        Rect::new(area.x, y, area.width, 1),
    );
    y += 1;

    let box_x = area.x + area.width.saturating_sub(box_w) / 2;
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                "Keel es la configuración personal de NixOS de ",
                Style::default().fg(app.palette.muted),
            ),
            Span::styled("HBlanqueto", Style::default().fg(app.palette.highlight)),
            Span::styled(
                ". No soy un experto en Nix, así que úsala bajo tu propio riesgo.",
                Style::default().fg(app.palette.muted),
            ),
        ]))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true }),
        Rect::new(box_x, y, box_w, 2),
    );
    y += 3;

    let box_rect = Rect::new(box_x, y, box_w, box_h);
    draw_box(f, app, box_rect);
    y += box_h;

    let gray = format!(
        "Se guardará una copia en /etc/keel/LICENSE · línea {} de {}",
        app.license_line.min(app.license_total).max(1),
        app.license_total
    );
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            gray,
            Style::default().fg(app.palette.muted),
        )))
        .alignment(Alignment::Center),
        Rect::new(area.x, y, area.width, 1),
    );
    y += 2;

    app.draw_buttons(f, Rect::new(area.x, y, area.width, 1));

    if app.license_modal {
        draw_modal(f, app, box_rect);
    }
}

fn draw_box(f: &mut Frame, app: &mut App, rect: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.palette.primary));
    let inner = block.inner(rect);
    f.render_widget(block, rect);

    let text_area = Rect::new(
        inner.x + 3,
        inner.y + 1,
        inner.width.saturating_sub(6),
        inner.height.saturating_sub(2),
    );
    app.license_text_area = text_area;

    let lines = reflow(&app.palette, text_area.width.max(8) as usize);
    let total = lines.len().max(1);
    let visible = text_area.height as usize;
    let max = total.saturating_sub(visible);
    app.license_max_scroll = max;
    if app.license_scroll > max {
        app.license_scroll = max;
    }
    let start = app.license_scroll;
    app.license_total = total;
    app.license_line = (start + 1).min(total);

    let rendered: Vec<Line> = lines.into_iter().skip(start).take(visible).collect();
    f.render_widget(Paragraph::new(rendered), text_area);

    if total > visible && visible > 0 {
        let bar_x = inner.right().saturating_sub(2);
        let thumb_h = (visible * visible / total).max(1);
        let travel = visible.saturating_sub(thumb_h);
        let max_start = total.saturating_sub(visible);
        let thumb_top = start
            .saturating_mul(travel)
            .checked_div(max_start)
            .unwrap_or(0);
        for row in 0..visible as u16 {
            let filled = (row as usize) >= thumb_top && (row as usize) < thumb_top + thumb_h;
            let (ch, style) = if filled {
                ("█", Style::default().fg(app.palette.primary))
            } else {
                ("│", Style::default().fg(app.palette.row_bg))
            };
            f.render_widget(
                Paragraph::new(Span::styled(ch, style)),
                Rect::new(bar_x, text_area.y + row, 1, 1),
            );
        }
    }
}

fn draw_modal(f: &mut Frame, app: &mut App, over: Rect) {
    let palette = app.palette;
    let buttons = vec![Btn::new("No acepto"), Btn::new("Acepto")];
    let focus = app.license_modal_focus;
    let message = Line::from(vec![
        Span::styled("❄  ", Style::default().fg(palette.primary)),
        Span::styled(
            "Leí y acepto los términos de la licencia.",
            Style::default().fg(palette.text),
        ),
    ]);
    widgets::draw_dialog(f, over, &palette, message, |f, row| {
        let widget = ButtonRow {
            buttons: &buttons,
            focus,
            focused: true,
            palette: &palette,
        };
        f.render_widget(widget, row);
    });
}

pub fn modal_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.license_modal = false,
        KeyCode::Left | KeyCode::Right | KeyCode::Tab | KeyCode::BackTab => {
            app.license_modal_focus = 1 - app.license_modal_focus;
        }
        KeyCode::Enter => {
            if app.license_modal_focus == 1 {
                app.license_modal = false;
                app.advance();
            } else {
                app.license_modal = false;
            }
        }
        _ => {}
    }
}

fn reflow(palette: &crate::theme::Palette, width: usize) -> Vec<Line<'static>> {
    let src: Vec<&str> = LICENSE.lines().collect();
    let mut out: Vec<Line<'static>> = Vec::new();
    let mut i = 0;
    let mut block = 0;
    while i < src.len() {
        while i < src.len() && src[i].trim().is_empty() {
            i += 1;
        }
        if i >= src.len() {
            break;
        }
        let start = i;
        while i < src.len() && !src[i].trim().is_empty() {
            i += 1;
        }
        if !out.is_empty() {
            out.push(Line::from(""));
        }
        emit_block(palette, &src[start..i], width, &mut out, block == 0);
        block += 1;
    }
    out
}

fn emit_block(
    palette: &crate::theme::Palette,
    block: &[&str],
    width: usize,
    out: &mut Vec<Line<'static>>,
    header: bool,
) {
    let lines: Vec<&str> = block
        .iter()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();
    if lines.is_empty() {
        return;
    }
    if header || lines.iter().any(|l| l.contains("Copyright (C)")) {
        for line in &lines {
            let style = if is_uppercase(line) {
                Style::default()
                    .fg(palette.white)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette.text)
            };
            out.push(centered(line, style));
        }
        return;
    }
    if block
        .iter()
        .filter(|l| !l.trim().is_empty())
        .all(|l| leading_spaces(l) >= 15)
    {
        for line in &lines {
            out.push(centered(
                line,
                Style::default()
                    .fg(palette.white)
                    .add_modifier(Modifier::BOLD),
            ));
        }
        return;
    }
    let joined = lines.join(" ");
    let indent = if is_numbered(&joined) { 3 } else { 0 };
    let wrapped = wrap_words(&joined, width.saturating_sub(indent).max(1));
    for (i, line) in wrapped.iter().enumerate() {
        let pad = if i > 0 { indent } else { 0 };
        out.push(Line::from(Span::styled(
            format!("{}{line}", " ".repeat(pad)),
            Style::default().fg(palette.text),
        )));
    }
}

fn centered(text: &str, style: Style) -> Line<'static> {
    Line::from(Span::styled(text.to_string(), style)).alignment(Alignment::Center)
}

fn leading_spaces(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn is_uppercase(text: &str) -> bool {
    text.chars().any(|c| c.is_alphabetic())
        && text
            .chars()
            .filter(|c| c.is_alphabetic())
            .all(|c| c.is_uppercase())
}

fn is_numbered(text: &str) -> bool {
    let mut chars = text.chars();
    match chars.next() {
        Some(c) if c.is_ascii_digit() => text.chars().take(3).any(|c| c == '.'),
        Some(c) if c.is_ascii_alphabetic() => chars.next() == Some(')'),
        _ => false,
    }
}

fn wrap_words(text: &str, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current = word.to_string();
        } else if current.chars().count() + 1 + word.chars().count() <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(std::mem::take(&mut current));
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

pub fn key(app: &mut App, key: KeyEvent) -> bool {
    let page = app.license_text_area.height.max(1) as usize;
    match key.code {
        KeyCode::Up => {
            app.license_scroll = app.license_scroll.saturating_sub(1);
            true
        }
        KeyCode::Down => {
            app.license_scroll = (app.license_scroll + 1).min(app.license_max_scroll);
            true
        }
        KeyCode::PageUp => {
            app.license_scroll = app.license_scroll.saturating_sub(page);
            true
        }
        KeyCode::PageDown => {
            app.license_scroll = (app.license_scroll + page).min(app.license_max_scroll);
            true
        }
        KeyCode::Home => {
            app.license_scroll = 0;
            true
        }
        KeyCode::End => {
            app.license_scroll = app.license_max_scroll;
            true
        }
        KeyCode::Char(' ') => {
            app.license_scroll = (app.license_scroll + page).min(app.license_max_scroll);
            true
        }
        _ => false,
    }
}

pub fn buttons(_app: &App) -> Vec<(Btn, Action)> {
    vec![
        (Btn::new("No acepto"), Action::Back),
        (Btn::new("Acepto"), Action::LicenseAccept),
    ]
}

pub fn commands(_app: &App) -> Vec<String> {
    vec!["cp LICENSE /mnt/etc/keel/LICENSE".into()]
}
