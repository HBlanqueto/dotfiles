//! Small shared widgets: buttons, fields, list rows and dialog dimmer.
use crate::theme::Palette;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Widget};
use ratatui::Frame;

#[derive(Debug, Clone)]
pub struct Btn {
    pub label: String,
    pub enabled: bool,
    pub danger: bool,
}

impl Btn {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            enabled: true,
            danger: false,
        }
    }

    pub fn danger(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            enabled: true,
            danger: true,
        }
    }
}

pub struct StepLayout {
    pub progress: Rect,
    pub title: Rect,
    pub body: Rect,
    pub content: Rect,
    pub note: Rect,
    pub buttons: Rect,
    pub faltante: Rect,
    pub shortcuts: Rect,
    pub commands: Rect,
}

impl StepLayout {
    pub fn compute(area: Rect, note_h: u16, commands_h: u16) -> Self {
        let width = (area.width * 60 / 100).clamp(64, 90).min(area.width);
        let height = (area.height * 50 / 100).clamp(14, 24).min(area.height);
        let x = area.x + area.width.saturating_sub(width) / 2;
        let gap: u16 = 1;
        let shortcuts_h = u16::from(area.height >= 30);
        let mut total = 1 + gap + 1 + gap + height + gap + 1 + gap + 1;
        if note_h > 0 {
            total += note_h + gap;
        }
        if shortcuts_h > 0 {
            total += shortcuts_h + gap;
        }
        if commands_h > 0 {
            total += commands_h + gap;
        }
        let mut y = area.y + area.height.saturating_sub(total) / 2;

        let progress = Rect::new(x, y, width, 1);
        y += 1 + gap;
        let title = Rect::new(x, y, width, 1);
        y += 1 + gap;
        let body = Rect::new(x, y, width, height);
        let inside = Rect::new(
            body.x + 1,
            body.y + 1,
            body.width.saturating_sub(2),
            body.height.saturating_sub(2),
        );
        let content = Rect::new(
            inside.x + 3,
            inside.y + 1,
            inside.width.saturating_sub(6),
            inside.height.saturating_sub(2),
        );
        y += height + gap;
        let note = Rect::new(x, y, width, note_h);
        if note_h > 0 {
            y += note_h + gap;
        }
        let buttons = Rect::new(x, y, width, 1);
        y += 1 + gap;
        let faltante = Rect::new(x, y, width, 1);
        y += 1 + gap;
        let shortcuts = Rect::new(x, y, width, shortcuts_h);
        if shortcuts_h > 0 {
            y += shortcuts_h + gap;
        }
        let commands = Rect::new(x, y, width, commands_h);

        Self {
            progress,
            title,
            body,
            content,
            note,
            buttons,
            faltante,
            shortcuts,
            commands,
        }
    }
}

pub fn scroll_to_show(focus_y: u16, focus_h: u16, visible: u16) -> u16 {
    (focus_y + focus_h).saturating_sub(visible)
}

pub fn draw_scrollbar(
    f: &mut Frame,
    palette: &Palette,
    area: Rect,
    total: usize,
    offset: u16,
    visible: u16,
) {
    let visible = visible as usize;
    if total <= visible || area.height == 0 {
        return;
    }
    let thumb_h = (visible * visible / total).max(1);
    let travel = visible.saturating_sub(thumb_h);
    let max_start = total.saturating_sub(visible);
    let thumb_top = (offset as usize)
        .saturating_mul(travel)
        .checked_div(max_start)
        .unwrap_or(0);
    let x = area.right();
    for row in 0..area.height {
        let filled = (row as usize) >= thumb_top && (row as usize) < thumb_top + thumb_h;
        let (ch, style) = if filled {
            ("█", Style::default().fg(palette.primary))
        } else {
            ("│", Style::default().fg(palette.row_bg))
        };
        f.render_widget(
            Paragraph::new(Span::styled(ch, style)),
            Rect::new(x, area.y + row, 1, 1),
        );
    }
}

pub fn dim_buffer(f: &mut Frame, palette: &Palette) {
    let area = f.area();
    let base = palette.reset;
    let buf = f.buffer_mut();
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            if let Some(cell) = buf.cell_mut((x, y)) {
                let half_block = matches!(cell.symbol(), "▀" | "▄" | "▌" | "▐");
                cell.fg = palette.disabled;
                cell.modifier.remove(Modifier::BOLD | Modifier::UNDERLINED);
                if cell.bg != base {
                    cell.bg = if half_block {
                        palette.disabled
                    } else {
                        palette.dim_bg
                    };
                }
            }
        }
    }
}

pub fn draw_dialog(
    f: &mut Frame,
    over: Rect,
    palette: &Palette,
    message: Line<'static>,
    buttons: impl FnOnce(&mut Frame, Rect),
) {
    dim_buffer(f, palette);

    let screen = f.area();
    let max_w = (screen.width * 60 / 100).max(1);
    let width = (message.width() as u16 + 8).max(44).min(max_w);
    let height: u16 = 7;
    let x = over.x + over.width.saturating_sub(width) / 2;
    let y = over.y + over.height.saturating_sub(height) / 2;
    let rect = Rect::new(x, y, width.min(screen.width), height);

    f.render_widget(Clear, rect);
    f.buffer_mut()
        .set_style(rect, Style::default().bg(palette.well));
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(palette.primary))
        .style(Style::default().bg(palette.well));
    let inner = block.inner(rect);
    f.render_widget(block, rect);

    f.render_widget(
        Paragraph::new(message).alignment(Alignment::Center),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );
    buttons(f, Rect::new(inner.x, inner.y + 3, inner.width, 1));
}

pub struct ButtonRow<'a> {
    pub buttons: &'a [Btn],
    pub focus: usize,
    pub focused: bool,
    pub palette: &'a Palette,
}

impl Widget for ButtonRow<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut flat: Vec<Span<'static>> = Vec::new();
        for (i, button) in self.buttons.iter().enumerate() {
            if i > 0 {
                flat.push(Span::raw("   "));
            }
            let is_focus = self.focused && i == self.focus && button.enabled;
            if !button.enabled {
                push_button(
                    &mut flat,
                    &button.label,
                    Style::default().fg(self.palette.disabled),
                );
            } else if is_focus {
                let text = Style::default()
                    .fg(self.palette.white)
                    .bg(self.palette.primary)
                    .add_modifier(Modifier::BOLD);
                push_button(&mut flat, &button.label, text);
            } else {
                let color = if button.danger {
                    self.palette.error
                } else {
                    self.palette.muted
                };
                push_button(&mut flat, &button.label, Style::default().fg(color));
            }
        }
        Paragraph::new(Line::from(flat))
            .alignment(Alignment::Center)
            .render(area, buf);
    }
}

fn push_button(flat: &mut Vec<Span<'static>>, label: &str, text: Style) {
    flat.push(Span::styled(format!(" {label} "), text));
}

#[allow(clippy::too_many_arguments)]
pub fn text_field(
    palette: &Palette,
    label: &str,
    value: &str,
    cursor: usize,
    focused: bool,
    cursor_on: bool,
    masked: bool,
    placeholder: &str,
    width: u16,
    note: Option<Line<'static>>,
    error: bool,
) -> Vec<Line<'static>> {
    let w = width.max(4) as usize;
    let label_style = if focused {
        Style::default().fg(palette.white)
    } else {
        Style::default().fg(palette.muted)
    };
    let note_w = note.as_ref().map(|n| n.width()).unwrap_or(0);
    let pad = w.saturating_sub(label.chars().count() + note_w + 1);
    let mut label_spans = vec![
        Span::styled(label.to_string(), label_style),
        Span::raw(" ".repeat(pad)),
    ];
    if let Some(note) = note {
        label_spans.extend(note.spans);
    }
    let label_line = Line::from(label_spans);

    let border_color = if error {
        palette.error
    } else if focused {
        palette.primary
    } else {
        palette.rule
    };
    let border = Style::default().fg(border_color);
    let top = Line::from(Span::styled(format!("╭{}╮", "─".repeat(w - 2)), border));
    let bottom = Line::from(Span::styled(format!("╰{}╯", "─".repeat(w - 2)), border));

    let inner_w = w.saturating_sub(4);
    let empty = value.is_empty();
    let mut display: Vec<char> = if empty {
        placeholder.chars().collect()
    } else if masked {
        vec!['•'; value.chars().count()]
    } else {
        value.chars().collect()
    };
    let cursor = cursor.min(if empty { 0 } else { display.len() });
    let start = if display.len() > inner_w {
        cursor.saturating_sub(inner_w.saturating_sub(1))
    } else {
        0
    };
    let end = (start + inner_w).min(display.len());
    let mut window: Vec<char> = display.drain(start..end).collect();
    while window.len() < inner_w {
        window.push(' ');
    }
    let cw = cursor.saturating_sub(start);
    let value_color = if empty {
        palette.disabled
    } else {
        palette.white
    };
    let before: String = window[..cw.min(window.len())].iter().collect();
    let at: char = window.get(cw).copied().unwrap_or(' ');
    let after: String = window.get(cw + 1..).unwrap_or(&[]).iter().collect();

    let mut value_spans = vec![
        Span::styled("│ ", border),
        Span::styled(before, Style::default().fg(value_color)),
    ];
    if focused && cursor_on {
        value_spans.push(Span::styled(
            at.to_string(),
            Style::default().fg(palette.on_primary).bg(palette.primary),
        ));
    } else {
        value_spans.push(Span::styled(
            at.to_string(),
            Style::default().fg(value_color),
        ));
    }
    value_spans.push(Span::styled(after, Style::default().fg(value_color)));
    value_spans.push(Span::styled(" │", border));
    let value_line = Line::from(value_spans);

    vec![label_line, top, value_line, bottom]
}

pub fn checkbox(palette: &Palette, checked: bool, label: &str, focused: bool) -> Line<'static> {
    let mark = if checked { "[x]" } else { "[ ]" };
    let mark_style = Style::default().fg(if checked {
        palette.primary
    } else {
        palette.muted
    });
    let label_style = if focused {
        Style::default().fg(palette.white)
    } else {
        Style::default().fg(palette.text)
    };
    Line::from(vec![
        Span::styled(format!("{mark} "), mark_style),
        Span::styled(label.to_string(), label_style),
    ])
}

pub fn list_row(
    palette: &Palette,
    selected: bool,
    text: String,
    trailing: Option<String>,
) -> Line<'static> {
    let style = if selected {
        Style::default()
            .fg(palette.white)
            .bg(palette.primary)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette.text)
    };
    let mut spans = vec![Span::styled(text, style)];
    if let Some(trailing) = trailing {
        spans.push(Span::styled(
            trailing,
            Style::default().fg(if selected {
                palette.white
            } else {
                palette.muted
            }),
        ));
    }
    Line::from(spans)
}
