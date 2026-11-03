const ZONE1970_TAB: &str = include_str!(env!("KEEL_ZONE1970_TAB"));

pub fn zones() -> Vec<String> {
    let mut zones = vec!["UTC".to_string()];
    for line in ZONE1970_TAB.lines() {
        if line.starts_with('#') {
            continue;
        }
        let Some(zone) = line.split('\t').nth(2) else {
            continue;
        };
        let zone = zone.trim();
        if !zone.is_empty() {
            zones.push(zone.to_string());
        }
    }
    zones
}
