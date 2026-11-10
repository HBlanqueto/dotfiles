//! Network probing, Wi-Fi association and the backend abstraction.
use color_eyre::eyre::{eyre, Result};
use std::process::{Command, Stdio};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wifi {
    pub ssid: String,
    pub signal: u8,
    pub secured: bool,
}

impl Wifi {
    pub fn bars(&self) -> &'static str {
        match self.signal {
            76..=100 => "▂▄▆█",
            51..=75 => "▂▄▆ ",
            26..=50 => "▂▄  ",
            _ => "▂   ",
        }
    }

    pub fn security(&self) -> &'static str {
        if self.secured {
            "WPA2"
        } else {
            "abierta"
        }
    }
}

pub trait WifiBackend: Send + Sync {
    fn name(&self) -> &'static str;
    fn scan(&self) -> Vec<Wifi>;
    fn connect(&self, ssid: &str, pass: &str) -> Result<()>;
    fn status(&self) -> bool;
    fn is_saved(&self, ssid: &str) -> bool {
        let _ = ssid;
        false
    }
}

pub fn backend(dry_run: bool) -> Arc<dyn WifiBackend> {
    if dry_run {
        return Arc::new(Sim);
    }
    if which("nmcli") {
        Arc::new(Nmcli)
    } else if which("iwctl") {
        Arc::new(Iwd)
    } else if which("wpa_cli") {
        Arc::new(WpaCli)
    } else {
        Arc::new(Null)
    }
}

pub fn ethernet_connected() -> bool {
    let Ok(entries) = std::fs::read_dir("/sys/class/net") else {
        return false;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "lo" {
            continue;
        }
        let base = entry.path();
        if base.join("wireless").exists() || base.join("phy80211").exists() {
            continue;
        }
        let operstate = std::fs::read_to_string(base.join("operstate")).unwrap_or_default();
        let carrier = std::fs::read_to_string(base.join("carrier")).unwrap_or_default();
        if operstate.trim() == "up" && carrier.trim() == "1" {
            return true;
        }
    }
    false
}

pub fn online() -> bool {
    crate::detect::network_online()
}

pub fn connection_up() -> bool {
    if let Ok(output) = Command::new("nmcli")
        .args(["-t", "-f", "STATE", "general"])
        .stdin(Stdio::null())
        .output()
    {
        if output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == "connected"
        {
            return true;
        }
    }
    has_default_route() && resolves("cache.nixos.org")
}

fn has_default_route() -> bool {
    let Ok(text) = std::fs::read_to_string("/proc/net/route") else {
        return false;
    };
    text.lines().skip(1).any(|line| {
        let mut fields = line.split_whitespace();
        let _iface = fields.next();
        let dest = fields.next().unwrap_or("");
        let gateway = fields.next().unwrap_or("");
        dest == "00000000" && gateway != "00000000" && !gateway.is_empty()
    })
}

fn resolves(host: &str) -> bool {
    use std::net::ToSocketAddrs;
    (host, 443).to_socket_addrs().is_ok()
}

pub struct Nmcli;
pub struct Iwd;
pub struct WpaCli;
pub struct Null;
pub struct Sim;

impl WifiBackend for Nmcli {
    fn name(&self) -> &'static str {
        "nmcli"
    }
    fn scan(&self) -> Vec<Wifi> {
        scan_nmcli()
    }
    fn connect(&self, ssid: &str, pass: &str) -> Result<()> {
        let mut cmd = Command::new("nmcli");
        cmd.args(["device", "wifi", "connect", ssid]);
        if !pass.is_empty() {
            cmd.args(["password", pass]);
        }
        run_connect(cmd)
    }
    fn status(&self) -> bool {
        online()
    }
    fn is_saved(&self, ssid: &str) -> bool {
        let Ok(output) = Command::new("nmcli")
            .args(["-t", "-f", "NAME", "connection", "show"])
            .stdin(Stdio::null())
            .output()
        else {
            return false;
        };
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .any(|line| line.trim() == ssid)
    }
}

impl WifiBackend for Iwd {
    fn name(&self) -> &'static str {
        "iwctl"
    }
    fn scan(&self) -> Vec<Wifi> {
        scan_iwctl()
    }
    fn connect(&self, ssid: &str, pass: &str) -> Result<()> {
        let Some(device) = wireless_device() else {
            return Err(eyre!("no hay interfaz inalámbrica"));
        };
        let mut cmd = Command::new("iwctl");
        if !pass.is_empty() {
            cmd.args(["--passphrase", pass]);
        }
        cmd.args(["station", &device, "connect", ssid]);
        run_connect(cmd)
    }
    fn status(&self) -> bool {
        online()
    }
}

impl WifiBackend for WpaCli {
    fn name(&self) -> &'static str {
        "wpa_cli"
    }
    fn scan(&self) -> Vec<Wifi> {
        scan_wpa_cli()
    }
    fn connect(&self, _ssid: &str, _pass: &str) -> Result<()> {
        Err(eyre!(
            "wpa_cli no permite asociarse sin editar la configuración; usa nmcli"
        ))
    }
    fn status(&self) -> bool {
        online()
    }
}

impl WifiBackend for Null {
    fn name(&self) -> &'static str {
        "ninguno"
    }
    fn scan(&self) -> Vec<Wifi> {
        Vec::new()
    }
    fn connect(&self, _ssid: &str, _pass: &str) -> Result<()> {
        Err(eyre!("no hay backend de red disponible"))
    }
    fn status(&self) -> bool {
        online()
    }
}

impl WifiBackend for Sim {
    fn name(&self) -> &'static str {
        "simulación"
    }
    fn scan(&self) -> Vec<Wifi> {
        sample_networks()
    }
    fn connect(&self, _ssid: &str, pass: &str) -> Result<()> {
        if pass.chars().count() >= 8 || pass.is_empty() {
            Ok(())
        } else {
            Err(eyre!("contraseña incorrecta"))
        }
    }
    fn status(&self) -> bool {
        true
    }
}

fn which(cmd: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| dir.join(cmd).is_file())
}

fn scan_nmcli() -> Vec<Wifi> {
    let Ok(output) = Command::new("nmcli")
        .args(["-t", "-f", "SSID,SIGNAL,SECURITY", "device", "wifi", "list"])
        .stdin(Stdio::null())
        .output()
    else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    let mut networks: Vec<Wifi> = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let fields = split_escaped(line);
        if fields.len() < 3 {
            continue;
        }
        let ssid = fields[0].trim().to_string();
        if ssid.is_empty() {
            continue;
        }
        let signal = fields[1].trim().parse::<u8>().unwrap_or(0);
        let security = fields[2].trim();
        let secured = !security.is_empty() && security != "--";
        push_unique(
            &mut networks,
            Wifi {
                ssid,
                signal,
                secured,
            },
        );
    }
    networks.sort_by_key(|n| std::cmp::Reverse(n.signal));
    networks
}

fn split_escaped(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut escaped = false;
    for ch in line.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == ':' {
            fields.push(std::mem::take(&mut current));
        } else {
            current.push(ch);
        }
    }
    fields.push(current);
    fields
}

fn scan_iwctl() -> Vec<Wifi> {
    let Some(device) = wireless_device() else {
        return Vec::new();
    };
    let _ = Command::new("iwctl")
        .args(["station", &device, "scan"])
        .stdin(Stdio::null())
        .output();
    let Ok(output) = Command::new("iwctl")
        .args(["station", &device, "get-networks"])
        .stdin(Stdio::null())
        .output()
    else {
        return Vec::new();
    };
    let mut networks = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("---") || trimmed.contains("Network name") {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        let Some(ssid) = parts.next() else { continue };
        let rest: Vec<&str> = parts.collect();
        let security = rest.first().copied().unwrap_or("");
        let secured = security == "psk" || security == "8021x";
        push_unique(
            &mut networks,
            Wifi {
                ssid: ssid.to_string(),
                signal: 70,
                secured,
            },
        );
    }
    networks
}

fn scan_wpa_cli() -> Vec<Wifi> {
    let _ = Command::new("wpa_cli")
        .arg("scan")
        .stdin(Stdio::null())
        .output();
    let Ok(output) = Command::new("wpa_cli")
        .arg("scan_results")
        .stdin(Stdio::null())
        .output()
    else {
        return Vec::new();
    };
    let mut networks = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines().skip(1) {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 5 {
            continue;
        }
        let signal = parts[2].trim().parse::<i32>().unwrap_or(-100);
        let signal = ((signal + 100).clamp(0, 100)) as u8;
        let secured = parts[3].contains("WPA") || parts[3].contains("WEP");
        let ssid = parts[4].trim().to_string();
        if ssid.is_empty() {
            continue;
        }
        push_unique(
            &mut networks,
            Wifi {
                ssid,
                signal,
                secured,
            },
        );
    }
    networks.sort_by_key(|n| std::cmp::Reverse(n.signal));
    networks
}

fn push_unique(networks: &mut Vec<Wifi>, wifi: Wifi) {
    if !networks.iter().any(|n| n.ssid == wifi.ssid) {
        networks.push(wifi);
    }
}

fn wireless_device() -> Option<String> {
    let entries = std::fs::read_dir("/sys/class/net").ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if entry.path().join("wireless").exists() {
            return Some(name);
        }
    }
    None
}

fn run_connect(mut cmd: Command) -> Result<()> {
    let output = cmd.stdin(Stdio::null()).output()?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let lower = stderr.to_lowercase();
    if lower.contains("secret") || lower.contains("password") || lower.contains("passphrase") {
        Err(eyre!("contraseña incorrecta"))
    } else {
        Err(eyre!("{}", stderr.trim()))
    }
}

pub fn sample_networks() -> Vec<Wifi> {
    vec![
        Wifi {
            ssid: "system-lab".into(),
            signal: 90,
            secured: true,
        },
        Wifi {
            ssid: "Casa-2.4G".into(),
            signal: 62,
            secured: true,
        },
        Wifi {
            ssid: "Cafe-Invitados".into(),
            signal: 34,
            secured: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bars_reflect_signal() {
        assert_eq!(
            Wifi {
                ssid: "x".into(),
                signal: 90,
                secured: false
            }
            .bars(),
            "▂▄▆█"
        );
        assert_eq!(
            Wifi {
                ssid: "x".into(),
                signal: 10,
                secured: false
            }
            .bars(),
            "▂   "
        );
    }

    #[test]
    fn nmcli_unescapes_colons() {
        let fields = split_escaped(r"Casa\:2.4G:80:WPA2");
        assert_eq!(fields[0], "Casa:2.4G");
        assert_eq!(fields[1], "80");
    }

    #[test]
    fn which_finds_common_tools() {
        assert!(which("sh"));
        assert!(!which("definitely-not-a-real-binary-xyz"));
    }

    #[test]
    fn sim_backend_accepts_long_and_rejects_short() {
        let sim = Sim;
        assert!(sim.connect("red", "12345678").is_ok());
        assert!(sim.connect("red", "").is_ok());
        let err = sim.connect("red", "corta").unwrap_err().to_string();
        assert_eq!(err, "contraseña incorrecta");
    }

    #[test]
    fn sim_backend_is_deterministic() {
        let sim = Sim;
        assert!(!sim.scan().is_empty());
        assert!(sim.status());
    }
}
