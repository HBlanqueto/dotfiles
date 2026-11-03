use crate::app::Picker;
use crate::theme::Palette;
use crate::widgets;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;
use tui_input::backend::crossterm::EventHandler;

pub const ROWS: usize = 8;

pub fn draw_centered(
    f: &mut Frame,
    palette: &Palette,
    area: Rect,
    picker: &Picker,
    suggested: Option<usize>,
    focused: bool,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(if focused {
            palette.primary
        } else {
            palette.rule
        }));
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block, area);

    let rows = inner.height as usize;
    let total = picker.filtered.len();
    let anchor = picker
        .selected
        .or_else(|| suggested.and_then(|s| picker.filtered.iter().position(|&i| i == s)));
    let start = anchor
        .unwrap_or(0)
        .saturating_sub(rows / 2)
        .min(total.saturating_sub(rows));
    let text_w = inner.width.saturating_sub(1) as usize;
    let accent = if focused {
        palette.primary
    } else {
        palette.highlight
    };

    for row in 0..rows as u16 {
        let row_rect = Rect::new(inner.x, inner.y + row, inner.width, 1);
        let Some(&item_idx) = picker.filtered.get(start + row as usize) else {
            continue;
        };
        let item = &picker.items[item_idx];
        let selected = picker.selected == Some(start + row as usize);
        let is_suggested = suggested == Some(item_idx);
        let style = if selected {
            Style::default().fg(accent).add_modifier(Modifier::BOLD)
        } else if is_suggested {
            Style::default()
                .fg(palette.focus)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(palette.text)
        };
        let name: String = item.name.chars().take(text_w).collect();
        let pad = text_w.saturating_sub(name.chars().count());
        let left = pad / 2;
        let mut line = Line::from(vec![
            Span::styled(" ".repeat(left), style),
            Span::styled(name, style),
            Span::styled(" ".repeat(pad - left), style),
        ]);
        if selected {
            line = line.style(Style::default().bg(palette.well));
        }
        f.render_widget(Paragraph::new(line), row_rect);
    }

    scrollbar(f, palette, inner, start, rows, total);
}

pub fn draw(f: &mut Frame, palette: &Palette, area: Rect, picker: &Picker, focused: bool) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(if focused {
            palette.primary
        } else {
            palette.rule
        }));
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block, area);

    let rows = inner.height as usize;
    let total = picker.filtered.len();
    let start = picker
        .selected
        .unwrap_or(0)
        .saturating_sub(rows / 2)
        .min(total.saturating_sub(rows));
    let text_w = inner.width.saturating_sub(1) as usize;
    let accent = if focused {
        palette.primary
    } else {
        palette.highlight
    };
    let mut lines: Vec<Line> = Vec::new();
    for row in 0..rows {
        let Some(&item_idx) = picker.filtered.get(start + row) else {
            lines.push(Line::from(""));
            continue;
        };
        let item = &picker.items[item_idx];
        let selected = picker.selected == Some(start + row);
        let style = if selected {
            Style::default().fg(accent).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(palette.text)
        };
        let trailing = match (item.extra.is_empty(), item.name.is_empty()) {
            (false, false) => format!("{}  {}", item.extra, item.name),
            (false, true) => item.extra.clone(),
            (true, _) => item.name.clone(),
        };
        let trailing_style = if selected {
            Style::default().fg(accent).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(palette.muted)
        };
        let code = item.code.chars().take(text_w).collect::<String>();
        let used = code.chars().count() + trailing.chars().count();
        let gap = text_w.saturating_sub(used);
        let mut line = Line::from(vec![
            Span::styled(code, style),
            Span::styled(" ".repeat(gap), style),
            Span::styled(trailing, trailing_style),
        ]);
        if selected {
            line = line.style(Style::default().bg(palette.well));
        }
        lines.push(line);
    }
    let content = Rect::new(
        inner.x,
        inner.y,
        inner.width.saturating_sub(1),
        inner.height,
    );
    f.render_widget(Paragraph::new(lines), content);
    scrollbar(f, palette, inner, start, rows, total);
}

fn scrollbar(
    f: &mut Frame,
    palette: &Palette,
    area: Rect,
    start: usize,
    rows: usize,
    total: usize,
) {
    if total <= rows || area.width == 0 {
        return;
    }
    let x = area.right().saturating_sub(1);
    let thumb_h = (rows * rows / total).max(1);
    let travel = rows.saturating_sub(thumb_h);
    let max_start = total.saturating_sub(rows);
    let thumb_top = start
        .saturating_mul(travel)
        .checked_div(max_start)
        .unwrap_or(0);
    for row in 0..rows {
        let filled = row >= thumb_top && row < thumb_top + thumb_h;
        let (ch, style) = if filled {
            ("█", Style::default().fg(palette.primary))
        } else {
            ("│", Style::default().fg(palette.row_bg))
        };
        f.render_widget(
            Paragraph::new(Span::styled(ch, style)),
            Rect::new(x, area.y + row as u16, 1, 1),
        );
    }
}

pub fn search_box(
    f: &mut Frame,
    palette: &Palette,
    area: Rect,
    picker: &Picker,
    focused: bool,
    blink: bool,
) {
    let search = widgets::text_field(
        palette,
        "Buscar",
        picker.query.value(),
        picker.query.visual_cursor(),
        focused,
        focused && blink,
        false,
        "escribe para filtrar",
        area.width,
        None,
        false,
    );
    f.render_widget(Paragraph::new(search), area);
}

pub fn draw_search(
    f: &mut Frame,
    palette: &Palette,
    area: Rect,
    picker: &Picker,
    focused: bool,
    blink: bool,
) -> Rect {
    let chunks = Layout::vertical([
        Constraint::Length(4),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .split(area);
    search_box(f, palette, chunks[0], picker, focused, blink);
    chunks[2]
}

pub fn update_list_focus(app: &mut crate::app::App, key: &KeyEvent) {
    if super::is_edit_key(key) {
        app.list_focus = false;
    } else if matches!(
        key.code,
        KeyCode::Up
            | KeyCode::Down
            | KeyCode::PageUp
            | KeyCode::PageDown
            | KeyCode::Home
            | KeyCode::End
    ) {
        app.list_focus = true;
    }
}

pub fn key(picker: &mut Picker, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Up => {
            picker.up();
            true
        }
        KeyCode::Down => {
            picker.down();
            true
        }
        KeyCode::PageUp => {
            picker.move_by(-(ROWS as isize));
            true
        }
        KeyCode::PageDown => {
            picker.move_by(ROWS as isize);
            true
        }
        KeyCode::Home => {
            picker.selected = (!picker.filtered.is_empty()).then_some(0);
            true
        }
        KeyCode::End => {
            picker.selected = picker.filtered.len().checked_sub(1);
            true
        }
        _ if super::is_edit_key(&key) => {
            picker.query.handle_event(&Event::Key(key));
            picker.refilter();
            true
        }
        _ => false,
    }
}
