//! Installer screens; each exposes `draw`, `key`, `height`, `commands` and `buttons`.
pub mod disk;
pub mod done;
pub mod encryption;
pub mod host;
pub mod installing;
pub mod keyboard;
pub mod license;
pub mod locale;
pub mod network;
pub mod picker;
pub mod summary;
pub mod timezone;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn is_edit_key(key: &KeyEvent) -> bool {
    !key.modifiers.contains(KeyModifiers::CONTROL)
        && !matches!(
            key.code,
            KeyCode::Up
                | KeyCode::Down
                | KeyCode::Tab
                | KeyCode::BackTab
                | KeyCode::Enter
                | KeyCode::Esc
        )
}
