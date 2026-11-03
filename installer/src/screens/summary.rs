use crate::app::{Action, App, CheckState};
use crate::widgets::Btn;
use ratatui::crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

fn gb(bytes: u64) -> String {
    format!("{:.0} GB", bytes as f64 / 1_000_000_000.0)
}

fn row(app: &App, label: &str, value: String) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{label:<15}"),
            Style::default().fg(app.palette.muted),
        ),
        Span::styled(value, Style::default().fg(app.palette.text)),
    ])
}

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let disk = app
        .selected_disk()
        .map(|d| {
            let action = match app.disks.effective() {
                crate::disk::DiskDecision::WholeDisk => "todo el disco".to_string(),
                crate::disk::DiskDecision::FreeSpace { .. } => "en el espacio libre".to_string(),
                crate::disk::DiskDecision::Shrink { .. } => {
                    format!("junto a Windows ({})", gb(app.disks.shrink_bytes))
                }
                _ => String::new(),
            };
            format!("{} · {}", d.short_name(), action)
        })
        .unwrap_or_default();
    let cifrado = if app.enc.tpm && app.enc.tpm_present {
        "Sí · TPM".to_string()
    } else {
        "Sí".to_string()
    };

    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                format!("{:<15}", "Equipo"),
                Style::default().fg(app.palette.muted),
            ),
            Span::styled(
                app.settings.pretty_host_name.clone(),
                Style::default().fg(app.palette.text),
            ),
            Span::styled(" · ", Style::default().fg(app.palette.muted)),
            Span::styled(
                app.settings.host_name.clone(),
                Style::default().fg(app.palette.muted),
            ),
        ]),
        row(
            app,
            "Lenguaje",
            crate::screens::locale::display(&app.settings.default_locale),
        ),
        row(
            app,
            "Teclado",
            crate::keyboard::display(
                &app.settings.keyboard_layout,
                &app.settings.keyboard_variant,
            ),
        ),
        row(app, "Zona horaria", app.settings.time_zone.clone()),
        row(app, "Disco", disk),
        row(app, "Cifrado", cifrado),
        Line::from(Span::styled(
            "─".repeat(area.width as usize),
            Style::default().fg(app.palette.rule),
        )),
    ];

    let (mark, text, color) = match &app.source_check {
        CheckState::Ok => (
            "✓ ",
            "configuración evalúa sin errores".to_string(),
            app.palette.success,
        ),
        CheckState::Failed(err) => ("✗ ", err.clone(), app.palette.error),
        CheckState::Running => (
            "… ",
            "evaluando configuración".to_string(),
            app.palette.muted,
        ),
        CheckState::Idle => ("", String::new(), app.palette.muted),
    };
    lines.push(Line::from(vec![
        Span::styled(mark.to_string(), Style::default().fg(color)),
        Span::styled(text, Style::default().fg(color)),
    ]));

    f.render_widget(Paragraph::new(lines), area);
}

pub fn key(_app: &mut App, _key: KeyEvent) -> bool {
    false
}

pub fn buttons(_app: &App) -> Vec<(Btn, Action)> {
    vec![
        (Btn::new("Atrás"), Action::Back),
        (Btn::new("Instalar"), Action::Next),
    ]
}

pub fn commands(app: &App) -> Vec<String> {
    let device = app
        .selected_disk()
        .map(|d| d.path())
        .unwrap_or_else(|| "/dev/xxx".into());
    let mut commands = vec![
        format!(
            "disko --mode destroy,format,mount --yes-wipe-all-disks system/disko.nix --argstr device {device}"
        ),
        "nixos-install --flake /mnt/persist/keel/repo#keel --root /mnt --no-root-passwd --impure"
            .into(),
    ];
    if app.secure_boot {
        commands.push("sbctl create-keys --directory /mnt/persist/var/lib/sbctl".into());
    }
    commands
}
