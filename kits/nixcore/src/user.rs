pub const RESERVED_USERS: &[&str] = &[
    "root",
    "nixos",
    "bin",
    "daemon",
    "nobody",
    "adm",
    "lp",
    "sync",
    "shutdown",
    "halt",
    "mail",
    "operator",
    "games",
    "ftp",
    "http",
    "systemd-network",
    "systemd-resolve",
    "systemd-timesync",
    "messagebus",
    "polkitd",
    "sshd",
    "dbus",
    "nscd",
    "tss",
];

pub fn username_error(name: &str) -> Option<&'static str> {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() || c == '_' => {}
        Some(c) if c.is_ascii_digit() => return Some("no puede empezar por número"),
        _ => return Some("solo a-z, 0-9, - y _"),
    }
    if name.chars().count() > 32 {
        return Some("máximo 32 caracteres");
    }
    if !chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-') {
        return Some("solo a-z, 0-9, - y _");
    }
    if RESERVED_USERS.contains(&name) {
        return Some("nombre reservado");
    }
    None
}

pub fn full_name_error(name: &str) -> Option<&'static str> {
    if name.trim().is_empty() {
        return Some("no puede estar vacío");
    }
    if name.chars().count() > 128 {
        return Some("máximo 128 caracteres");
    }
    if name.chars().any(|c| c.is_control() || c == ':') {
        return Some("sin caracteres de control ni ':'");
    }
    None
}

pub fn host_name_error(name: &str) -> Option<&'static str> {
    if name.is_empty() {
        return Some("no puede estar vacío");
    }
    if name.chars().count() > 63 {
        return Some("máximo 63 caracteres");
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Some("solo letras, números y -");
    }
    if name.starts_with('-') || name.ends_with('-') {
        return Some("no puede empezar ni terminar por -");
    }
    None
}

pub fn host_name_from_pretty(pretty: &str) -> String {
    let mut out = String::new();
    for ch in pretty.chars() {
        let c = deaccent(ch);
        if c == '\'' || c == '’' || c == '‘' || c == '`' || c == '´' {
            continue;
        }
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let mut name = out.trim_matches('-').to_string();
    name.truncate(63);
    let name = name.trim_end_matches('-');
    if name.is_empty() {
        "keel".to_string()
    } else {
        name.to_string()
    }
}

pub fn generate_username(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        let base = deaccent(ch);
        if base.is_ascii_alphanumeric() {
            out.push(base.to_ascii_lowercase());
        }
    }
    if out
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
    {
        out.insert(0, 'u');
    }
    out.truncate(32);
    out
}

fn deaccent(ch: char) -> char {
    match ch {
        'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' | 'ā' | 'ă' | 'ą' => 'a',
        'Á' | 'À' | 'Â' | 'Ä' | 'Ã' | 'Å' => 'A',
        'é' | 'è' | 'ê' | 'ë' | 'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' => 'e',
        'É' | 'È' | 'Ê' | 'Ë' => 'E',
        'í' | 'ì' | 'î' | 'ï' | 'ī' | 'ĩ' => 'i',
        'Í' | 'Ì' | 'Î' | 'Ï' => 'I',
        'ó' | 'ò' | 'ô' | 'ö' | 'õ' | 'ø' | 'ō' => 'o',
        'Ó' | 'Ò' | 'Ô' | 'Ö' | 'Õ' => 'O',
        'ú' | 'ù' | 'û' | 'ü' | 'ū' | 'ũ' => 'u',
        'Ú' | 'Ù' | 'Û' | 'Ü' => 'U',
        'ñ' => 'n',
        'Ñ' => 'N',
        'ç' => 'c',
        'Ç' => 'C',
        'ý' | 'ÿ' => 'y',
        'Ý' => 'Y',
        _ => ch,
    }
}
