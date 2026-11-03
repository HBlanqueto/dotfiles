use std::fmt::Write as _;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=KEEL_LOCALE_DIR");
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("locale_names.rs");

    let mut entries: Vec<(String, String, String)> = Vec::new();
    if let Ok(dir) = std::env::var("KEEL_LOCALE_DIR") {
        if let Ok(read) = std::fs::read_dir(&dir) {
            for entry in read.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                let Ok(text) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let code = entry.file_name().to_string_lossy().to_string();
                let lang = field(&text, "lang_name").map(|s| decode_glibc(&s));
                let country = field(&text, "country_name").map(|s| decode_glibc(&s));
                if lang.is_some() || country.is_some() {
                    entries.push((code, lang.unwrap_or_default(), country.unwrap_or_default()));
                }
            }
        }
    }
    entries.sort();

    let mut src =
        String::from("/// (locale code, language name, country name), generated from glibc.\n");
    src.push_str("pub static LOCALE_NAMES: &[(&str, &str, &str)] = &[\n");
    for (code, lang, country) in &entries {
        let _ = writeln!(src, "    ({code:?}, {lang:?}, {country:?}),");
    }
    src.push_str("];\n");
    std::fs::write(&out, src).expect("write locale_names.rs");
}

fn field(text: &str, key: &str) -> Option<String> {
    for line in text.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix(key) else {
            continue;
        };
        if !rest.starts_with(|c: char| c.is_whitespace()) {
            continue;
        }
        let start = rest.find('"')?;
        let after = &rest[start + 1..];
        let end = after.find('"')?;
        return Some(after[..end].to_string());
    }
    None
}

fn decode_glibc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find("<U") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find('>') {
            Some(end) => match u32::from_str_radix(&after[..end], 16)
                .ok()
                .and_then(char::from_u32)
            {
                Some(c) => {
                    out.push(c);
                    rest = &after[end + 1..];
                }
                None => {
                    out.push_str("<U");
                    rest = after;
                }
            },
            None => {
                out.push_str("<U");
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}
