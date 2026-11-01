//! Language selection screen.
use crate::app::{Action, App, PickItem};
use crate::widgets::Btn;
use ratatui::crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;

pub use localekit::locale::display;

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let Some(picker) = &app.loc else {
        return;
    };
    let list = super::picker::draw_search(
        f,
        &app.palette,
        area,
        picker,
        !app.focus_buttons && !app.no_focus(),
        app.blink(),
    );
    super::picker::draw_centered(
        f,
        &app.palette,
        list,
        picker,
        None,
        app.list_focus && !app.focus_buttons && !app.no_focus(),
    );
}

pub fn key(app: &mut App, key: KeyEvent) -> bool {
    super::picker::update_list_focus(app, &key);
    let Some(picker) = app.loc.as_mut() else {
        return false;
    };
    let consumed = super::picker::key(picker, key);
    if consumed {
        if let Some(item) = picker.current() {
            app.settings.default_locale = item.code.clone();
        }
    }
    consumed
}

pub fn buttons(_app: &App) -> Vec<(Btn, Action)> {
    vec![
        (Btn::new("Atrás"), Action::Back),
        (Btn::new("Continuar"), Action::Next),
    ]
}

pub fn commands(app: &App) -> Vec<String> {
    vec![format!(
        "machine.toml: defaultLocale = \"{}\";",
        app.settings.default_locale
    )]
}

pub fn load_items() -> Vec<PickItem> {
    localekit::locale::all()
        .into_iter()
        .map(|l| PickItem {
            name: l.name,
            code: l.code,
            extra: String::new(),
        })
        .collect()
}
