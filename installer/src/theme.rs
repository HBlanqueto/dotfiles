use ratatui::style::Color;

pub mod hex {
    pub const LBG: &str = "212b38";

    pub const FG: &str = "adbac7";
    pub const BG: &str = "13171d";

    pub const C0: &str = "171d27";
    pub const C1: &str = "d47d7a";
    pub const C2: &str = "8ecb94";
    pub const C3: &str = "cfb984";
    pub const C4: &str = "809ad2";
    pub const C5: &str = "aa96c2";
    pub const C6: &str = "a5c0d2";
    pub const C7: &str = "cdd9e5";

    pub const C8: &str = "55657e";
    pub const C9: &str = "e59592";
    pub const C10: &str = "a3dcb1";
    pub const C11: &str = "dfca9b";
    pub const C12: &str = "97b7ff";
    pub const C13: &str = "bda9d6";
    pub const C14: &str = "cee8ff";
    pub const C15: &str = "eef4fa";

    pub const DISABLED: &str = "2f3a49";
    pub const RULE: &str = "2a3442";
    pub const DIM: &str = "1c2430";
    pub const LOGO_LIGHT: &str = "7fb7ff";
}

const fn nibble(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => 0,
    }
}

pub const fn rgb(hex: &str) -> Color {
    let b = hex.as_bytes();
    Color::Rgb(
        (nibble(b[0]) << 4) | nibble(b[1]),
        (nibble(b[2]) << 4) | nibble(b[3]),
        (nibble(b[4]) << 4) | nibble(b[5]),
    )
}

pub const CONSOLE_SLOTS: [&str; 16] = [
    hex::C0,
    hex::C1,
    hex::C2,
    hex::C3,
    hex::C4,
    hex::C5,
    hex::C6,
    hex::C7,
    hex::C8,
    hex::C9,
    hex::C10,
    hex::C11,
    hex::C12,
    hex::C13,
    hex::C14,
    hex::C15,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub text: Color,
    pub highlight: Color,
    pub muted: Color,
    pub border: Color,
    pub primary: Color,
    pub focus: Color,
    pub secondary: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub white: Color,
    pub well: Color,
    pub snapshot: Color,
    pub on_primary: Color,
    pub row_bg: Color,
    pub disabled: Color,
    pub rule: Color,
    pub dim_bg: Color,
    pub logo_light: Color,
    pub reset: Color,
}

impl Palette {
    pub const fn truecolor() -> Self {
        Self {
            text: rgb(hex::FG),
            highlight: rgb(hex::C7),
            muted: rgb(hex::C8),
            border: rgb(hex::LBG),
            primary: rgb(hex::C4),
            focus: rgb(hex::C12),
            secondary: rgb(hex::C6),
            success: rgb(hex::C2),
            warning: rgb(hex::C3),
            error: rgb(hex::C1),
            white: rgb(hex::C15),
            well: rgb(hex::C0),
            snapshot: rgb(hex::C5),
            disabled: rgb(hex::DISABLED),
            rule: rgb(hex::RULE),
            dim_bg: rgb(hex::DIM),
            logo_light: rgb(hex::LOGO_LIGHT),
            on_primary: rgb(hex::BG),
            row_bg: rgb(hex::LBG),
            reset: Color::Reset,
        }
    }

    pub const fn console() -> Self {
        Self {
            text: Color::Indexed(15),
            highlight: Color::Indexed(7),
            muted: Color::Indexed(8),
            border: Color::Indexed(0),
            primary: Color::Indexed(4),
            focus: Color::Indexed(12),
            secondary: Color::Indexed(6),
            success: Color::Indexed(2),
            warning: Color::Indexed(3),
            error: Color::Indexed(1),
            white: Color::Indexed(15),
            well: Color::Indexed(0),
            snapshot: Color::Indexed(5),
            disabled: Color::Indexed(0),
            rule: Color::Indexed(0),
            dim_bg: Color::Indexed(0),
            logo_light: Color::Indexed(12),
            on_primary: Color::Indexed(0),
            row_bg: Color::Indexed(0),
            reset: Color::Reset,
        }
    }
}

pub mod console {
    use super::CONSOLE_SLOTS;
    use std::io::Write;

    pub fn is_linux_console() -> bool {
        std::env::var("TERM").as_deref() == Ok("linux")
    }

    fn write_slots<W: Write>(out: &mut W) -> std::io::Result<()> {
        for (i, hex) in CONSOLE_SLOTS.iter().enumerate() {
            write!(out, "\x1b]P{i:x}{hex}")?;
        }
        out.flush()
    }

    fn write_restore<W: Write>(out: &mut W) -> std::io::Result<()> {
        out.write_all(b"\x1b]R")?;
        out.flush()
    }

    #[must_use = "the guard restores the console palette when dropped"]
    pub struct Guard {
        active: bool,
    }

    impl Guard {
        pub fn activate() -> Self {
            if !is_linux_console() {
                return Self { active: false };
            }
            let mut stdout = std::io::stdout();
            let _ = write_slots(&mut stdout);
            Self { active: true }
        }
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            if self.active {
                let mut stdout = std::io::stdout();
                let _ = write_restore(&mut stdout);
            }
        }
    }

    pub fn install_panic_hook() {
        let default = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = ratatui::crossterm::terminal::disable_raw_mode();
            let mut stdout = std::io::stdout();
            let _ = ratatui::crossterm::execute!(
                stdout,
                ratatui::crossterm::terminal::LeaveAlternateScreen,
                ratatui::crossterm::cursor::Show
            );
            if is_linux_console() {
                let _ = write_restore(&mut stdout);
            }
            default(info);
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn neptunia() -> HashMap<String, String> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("kits/neptunia/neptunia.nix");
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("no se pudo leer {}: {e}", path.display()));

        let mut map = HashMap::new();
        for line in text.lines() {
            let line = line.split('#').next().unwrap_or("");
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim();
            let value = value.trim().trim_end_matches(';').trim().trim_matches('"');
            if !key.is_empty() && !value.is_empty() {
                map.insert(key.to_string(), value.to_string());
            }
        }
        map
    }

    #[test]
    fn palette_constants_match_neptunia() {
        let n = neptunia();

        let checks: &[(&str, &str)] = &[
            ("lbg", hex::LBG),
            ("fg", hex::FG),
            ("bg", hex::BG),
            ("c0", hex::C0),
            ("c1", hex::C1),
            ("c2", hex::C2),
            ("c3", hex::C3),
            ("c4", hex::C4),
            ("c5", hex::C5),
            ("c6", hex::C6),
            ("c7", hex::C7),
            ("c8", hex::C8),
            ("c9", hex::C9),
            ("c10", hex::C10),
            ("c11", hex::C11),
            ("c12", hex::C12),
            ("c13", hex::C13),
            ("c14", hex::C14),
            ("c15", hex::C15),
            ("uiDisabled", hex::DISABLED),
            ("uiRule", hex::RULE),
            ("uiDim", hex::DIM),
            ("logoLight", hex::LOGO_LIGHT),
        ];

        for (key, ours) in checks {
            let theirs = n
                .get(*key)
                .unwrap_or_else(|| panic!("neptunia.nix no define {key}"));
            assert_eq!(theirs, ours, "neptunia.nix {key} cambió");
        }

        assert_eq!(n.get("primary").map(String::as_str), Some("c4"));
        assert_eq!(n.get("primaryBright").map(String::as_str), Some("c12"));
        assert_eq!(n.get("secondary").map(String::as_str), Some("c6"));
        assert_eq!(n.get("textColor").map(String::as_str), Some("bg"));
    }
}
