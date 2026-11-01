//! Glibc UTF-8 locales with names in their own language.
use std::collections::HashSet;

const SUPPORTED: &str = include_str!(env!("KEEL_LOCALES"));

include!(concat!(env!("OUT_DIR"), "/locale_names.rs"));

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Locale {
    pub code: String,
    pub name: String,
}

pub fn all() -> Vec<Locale> {
    let mut codes = parse_supported(SUPPORTED);
    codes.sort();
    codes.dedup();
    let mut seen = HashSet::new();
    let mut items: Vec<Locale> = codes
        .into_iter()
        .filter(|code| {
            let base = code.split('.').next().unwrap_or(code);
            base != "C" && base != "POSIX" && !base.starts_with("C_")
        })
        .filter_map(|code| {
            let name = display_name(&code);
            if seen.insert(name.clone()) {
                Some(Locale { name, code })
            } else {
                None
            }
        })
        .collect();
    items.sort_by(|a, b| a.name.cmp(&b.name));
    items
}

pub fn display(code: &str) -> String {
    display_name(code)
}

fn parse_supported(text: &str) -> Vec<String> {
    let mut codes = Vec::new();
    for token in text.split_whitespace() {
        let Some((name, charset)) = token.rsplit_once('/') else {
            continue;
        };
        if charset != "UTF-8" {
            continue;
        }
        let code = if name.contains('.') {
            name.to_string()
        } else {
            format!("{name}.UTF-8")
        };
        codes.push(code);
    }
    codes
}

fn display_name(code: &str) -> String {
    let base = code.split('.').next().unwrap_or(code);
    let base = base.split('@').next().unwrap_or(base);
    if let Some((_, lang, country)) = LOCALE_NAMES.iter().find(|(c, _, _)| *c == base) {
        if let Some(name) = compose(lang, country) {
            return name;
        }
    }
    code.to_string()
}

fn compose(lang: &str, country: &str) -> Option<String> {
    match (lang.is_empty(), country.is_empty()) {
        (true, true) => None,
        (false, true) => Some(capitalize(lang)),
        (true, false) => Some(country.to_string()),
        (false, false) => Some(format!("{} ({country})", capitalize(lang))),
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}
