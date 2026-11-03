use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

const DARK_RGB: (u8, u8, u8) = (82, 119, 195);
const LIGHT_RGB: (u8, u8, u8) = (127, 183, 255);

const LOGO: &str = include_str!("../assets/nixos-logo.ans");

fn blue(rgb: (u8, u8, u8), console: bool) -> Color {
    if !console {
        return Color::Rgb(rgb.0, rgb.1, rgb.2);
    }
    if rgb == DARK_RGB {
        Color::Indexed(4)
    } else {
        Color::Indexed(12)
    }
}

pub fn nixos_logo(console: bool) -> Vec<Line<'static>> {
    let dark = blue(DARK_RGB, console);
    let light = blue(LIGHT_RGB, console);

    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut run = String::new();
    let mut run_style = Style::default();
    let mut fg = Color::Reset;
    let mut bg = Color::Reset;

    let mut chars = LOGO.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            let mut seq = String::new();
            for n in chars.by_ref() {
                if n == 'm' {
                    break;
                }
                seq.push(n);
            }
            apply_sgr(&seq, &mut fg, &mut bg, dark, light);
            continue;
        }
        if c == '\n' {
            flush(&mut spans, &mut run, run_style);
            lines.push(Line::from(std::mem::take(&mut spans)));
            continue;
        }
        let style = Style::default().fg(fg).bg(bg);
        if style != run_style {
            flush(&mut spans, &mut run, run_style);
            run_style = style;
        }
        run.push(c);
    }
    flush(&mut spans, &mut run, run_style);
    if !spans.is_empty() {
        lines.push(Line::from(spans));
    }
    lines
}

fn flush(spans: &mut Vec<Span<'static>>, run: &mut String, style: Style) {
    if !run.is_empty() {
        spans.push(Span::styled(std::mem::take(run), style));
    }
}

fn apply_sgr(seq: &str, fg: &mut Color, bg: &mut Color, dark: Color, light: Color) {
    let seq = seq.strip_prefix('[').unwrap_or(seq);
    let params: Vec<&str> = seq.split(';').collect();
    match params.as_slice() {
        [""] | ["0"] => {
            *fg = Color::Reset;
            *bg = Color::Reset;
        }
        ["38", "2", r, g, b] => {
            *fg = map_rgb(r, g, b, dark, light);
        }
        ["48", "2", r, g, b] => {
            *bg = map_rgb(r, g, b, dark, light);
        }
        _ => {}
    }
}

fn map_rgb(r: &str, g: &str, b: &str, dark: Color, light: Color) -> Color {
    match (r.parse().ok(), g.parse().ok(), b.parse().ok()) {
        (Some(r), Some(g), Some(b)) if (r, g, b) == DARK_RGB => dark,
        (Some(r), Some(g), Some(b)) if (r, g, b) == LIGHT_RGB => light,
        (Some(r), Some(g), Some(b)) => Color::Rgb(r, g, b),
        _ => Color::Reset,
    }
}
