use serde::Deserialize;
use std::net::{TcpStream, ToSocketAddrs};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

pub fn run_trim(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

pub fn arch() -> Option<String> {
    run_trim("uname", &["-m"])
}

pub fn nix_system() -> Option<String> {
    let arch = arch()?;
    let arch = match arch.as_str() {
        "amd64" => "x86_64",
        other => other,
    };
    Some(format!("{arch}-linux"))
}

pub fn secure_boot() -> bool {
    if run_trim("systemd-detect-virt", &[])
        .as_deref()
        .is_some_and(|virt| virt != "none")
    {
        return false;
    }
    !matches!(arch().as_deref(), Some("aarch64") | Some("arm64"))
}

pub fn environment() -> Option<String> {
    let virt = run_trim("systemd-detect-virt", &[])?;
    match virt.as_str() {
        "qemu" => Some("UTM (qemu)".to_string()),
        _ => Some(virt),
    }
}

pub fn machine_model() -> String {
    let arch = arch().unwrap_or_default();
    let is_arm = arch.starts_with("arm") || arch == "aarch64";

    if let Some(virt) = run_trim("systemd-detect-virt", &[]) {
        if virt != "none" {
            return "Máquina virtual".to_string();
        }
    }

    let mut vendor = String::new();
    let mut candidate = None;
    if !is_arm {
        vendor = dmi_field("sys_vendor").unwrap_or_default();
        let product = dmi_field("product_name");
        let version = dmi_field("product_version");
        candidate = if vendor.to_lowercase().contains("lenovo") {
            version.or(product)
        } else {
            product.or(version)
        };
    }
    if candidate.is_none() {
        candidate = device_tree_model();
    }

    let model = candidate.unwrap_or_else(|| {
        if battery_present() {
            "Laptop".to_string()
        } else {
            "PC".to_string()
        }
    });
    simplify_model(&model, &vendor)
}

fn dmi_field(name: &str) -> Option<String> {
    let text = std::fs::read_to_string(format!("/sys/class/dmi/id/{name}")).ok()?;
    let value = text.trim();
    if value.is_empty() || is_placeholder(value) {
        None
    } else {
        Some(value.to_string())
    }
}

fn device_tree_model() -> Option<String> {
    let text = std::fs::read_to_string("/proc/device-tree/model").ok()?;
    let value = text.trim_end_matches('\0').trim();
    if value.is_empty() || is_placeholder(value) {
        None
    } else {
        Some(value.to_string())
    }
}

fn is_placeholder(value: &str) -> bool {
    matches!(
        value.to_lowercase().as_str(),
        "to be filled by o.e.m."
            | "system product name"
            | "default string"
            | "none"
            | "not applicable"
    )
}

fn battery_present() -> bool {
    std::fs::read_dir("/sys/class/power_supply")
        .map(|entries| {
            entries
                .flatten()
                .any(|entry| entry.file_name().to_string_lossy().starts_with("BAT"))
        })
        .unwrap_or(false)
}

fn simplify_model(model: &str, vendor: &str) -> String {
    let mut model = model.trim().to_string();
    if let Some(idx) = model.rfind(" Gen ") {
        let tail = &model[idx + 5..];
        if !tail.is_empty() && tail.chars().all(|c| c.is_alphanumeric()) {
            model.truncate(idx);
        }
    }
    if !vendor.is_empty() && model.to_lowercase().starts_with(&vendor.to_lowercase()) {
        let rest = model[vendor.len()..].trim_start();
        if !rest.is_empty() {
            model = rest.to_string();
        }
    }
    model.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn network_online() -> bool {
    let Ok(addrs) = ("cache.nixos.org", 443).to_socket_addrs() else {
        return false;
    };
    addrs
        .into_iter()
        .any(|addr| TcpStream::connect_timeout(&addr, Duration::from_secs(4)).is_ok())
}

pub fn is_live_environment() -> bool {
    is_mounted("/nix/.ro-store")
}

pub fn is_mounted(path: &str) -> bool {
    std::fs::read_to_string("/proc/mounts")
        .map(|mounts| {
            mounts
                .lines()
                .any(|line| line.split_whitespace().nth(1) == Some(path))
        })
        .unwrap_or(false)
}

pub fn euid() -> u32 {
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if let Some(rest) = line.strip_prefix("Uid:") {
                if let Some(real) = rest.split_whitespace().nth(1) {
                    return real.parse().unwrap_or(1);
                }
            }
        }
    }
    1
}

pub fn is_root() -> bool {
    euid() == 0
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Partition {
    pub name: String,
    pub size: u64,
    pub fstype: String,
    pub label: String,
    pub parttype: String,
    pub mountpoints: Vec<String>,
}

impl Partition {
    pub fn path(&self) -> String {
        format!("/dev/{}", self.name)
    }

    pub fn is_efi(&self) -> bool {
        self.parttype
            .eq_ignore_ascii_case("c12a7328-f81f-11d2-ba4b-00a0c93ec93b")
    }

    pub fn is_ntfs(&self) -> bool {
        self.fstype.eq_ignore_ascii_case("ntfs")
    }

    pub fn is_linux(&self) -> bool {
        matches!(
            self.fstype.as_str(),
            "ext4" | "btrfs" | "xfs" | "ext3" | "f2fs" | "crypto_LUKS"
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Disk {
    pub name: String,
    pub size: u64,
    pub model: String,
    pub note: String,
    pub available: bool,
    pub windows: bool,
    pub free: u64,
    pub partitions: Vec<Partition>,
    pub test: bool,
}

impl Disk {
    pub fn path(&self) -> String {
        format!("/dev/{}", self.name)
    }

    pub fn size_human(&self) -> String {
        human_size(self.size)
    }

    pub fn is_windows(&self) -> bool {
        self.windows
    }

    pub fn short_name(&self) -> String {
        let size = format!("{} GB", self.nominal_gb());
        let avail = 24usize.saturating_sub(size.chars().count() + 1);
        let model: String = self.model.chars().take(avail).collect();
        let model = model.trim_end();
        if model.is_empty() {
            size
        } else {
            format!("{model} {size}")
        }
    }

    pub fn real_size_gb(&self) -> String {
        format!("{:.1} GB", self.size as f64 / 1_073_741_824.0)
    }

    pub fn nominal_gb(&self) -> u64 {
        nominal_gb(self.size)
    }

    pub fn efi(&self) -> Option<&Partition> {
        self.partitions.iter().find(|p| p.is_efi())
    }

    pub fn ntfs(&self) -> Vec<&Partition> {
        let mut ntfs: Vec<&Partition> = self.partitions.iter().filter(|p| p.is_ntfs()).collect();
        ntfs.sort_by_key(|p| std::cmp::Reverse(p.size));
        ntfs
    }

    pub fn unusable_reason(&self) -> Option<&'static str> {
        if self
            .partitions
            .iter()
            .any(|p| p.mountpoints.iter().any(|m| is_live_mount(m)))
        {
            return Some("Es el medio de instalación");
        }
        if !self.available {
            return Some("En uso por el sistema actual");
        }
        if self.size < crate::disk::MIN_GIB * crate::disk::GIB {
            return Some("Demasiado pequeño (mín. 40 GB)");
        }
        None
    }
}

fn partition_in_use(partition: &Partition) -> bool {
    partition.fstype == "swap"
        || partition
            .mountpoints
            .iter()
            .any(|m| m == "/boot" || m.starts_with("/boot/"))
}

pub fn disks() -> Vec<Disk> {
    let Some(json) = run_trim(
        "lsblk",
        &[
            "-J",
            "-b",
            "-o",
            "NAME,SIZE,MODEL,TYPE,FSTYPE,PARTTYPE,LABEL,MOUNTPOINTS",
        ],
    ) else {
        return Vec::new();
    };
    parse_disks(&json)
}

pub fn windows_present() -> bool {
    disks().iter().any(|d| d.windows)
}

pub fn test_disks() -> Vec<Disk> {
    const GB: u64 = 1_000_000_000;
    let make = |name: &str, size_gb: u64, note: &str, windows: bool, free_gb: u64| Disk {
        name: name.to_string(),
        size: size_gb * GB,
        model: "Disco de prueba".to_string(),
        note: note.to_string(),
        available: true,
        windows,
        free: free_gb * GB,
        partitions: Vec::new(),
        test: true,
    };
    vec![
        make("prueba-vacio", 64, "vacío", false, 64),
        make("prueba-windows", 256, "Windows 11", true, 0),
        make("prueba-libre", 256, "Windows 11", true, 96),
    ]
}

#[derive(Deserialize)]
struct Lsblk {
    #[serde(default)]
    blockdevices: Vec<Node>,
}

#[derive(Deserialize)]
struct Node {
    name: String,
    #[serde(default)]
    size: Option<u64>,
    #[serde(default)]
    model: Option<String>,
    #[serde(rename = "type", default)]
    kind: Option<String>,
    #[serde(default)]
    mountpoint: Option<String>,
    #[serde(default)]
    mountpoints: Option<Vec<String>>,
    #[serde(default)]
    fstype: Option<String>,
    #[serde(default)]
    parttype: Option<String>,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    uuid: Option<String>,
    #[serde(default)]
    children: Vec<Node>,
}

fn parse_disks(json: &str) -> Vec<Disk> {
    let Ok(tree) = serde_json::from_str::<Lsblk>(json) else {
        return Vec::new();
    };
    tree.blockdevices
        .into_iter()
        .filter(|d| d.kind.as_deref() == Some("disk"))
        .filter(|d| !is_virtual(&d.name))
        .map(|d| {
            let partitions: Vec<Partition> = d
                .children
                .iter()
                .filter(|c| c.kind.as_deref() == Some("part"))
                .map(node_partition)
                .collect();
            let used: u64 = partitions.iter().map(|p| p.size).sum();
            let size = d.size.unwrap_or(0);
            let windows = partitions.iter().any(Partition::is_ntfs);
            let note = disk_note(&partitions);
            Disk {
                note,
                available: !holds_live_medium(&d) && !partitions.iter().any(partition_in_use),
                windows,
                free: size.saturating_sub(used).saturating_sub(1024 * 1024),
                model: d.model.clone().unwrap_or_default().trim().to_string(),
                size,
                name: d.name.clone(),
                partitions,
                test: false,
            }
        })
        .collect()
}

fn node_partition(node: &Node) -> Partition {
    let mut mountpoints = node.mountpoints.clone().unwrap_or_default();
    if let Some(mp) = &node.mountpoint {
        if !mp.is_empty() && !mountpoints.contains(mp) {
            mountpoints.push(mp.clone());
        }
    }
    Partition {
        name: node.name.clone(),
        size: node.size.unwrap_or(0),
        fstype: node.fstype.clone().unwrap_or_default(),
        label: node.label.clone().unwrap_or_default(),
        parttype: node.parttype.clone().unwrap_or_default(),
        mountpoints,
    }
}

fn is_virtual(name: &str) -> bool {
    ["zram", "loop", "sr", "ram", "fd", "dm-"]
        .iter()
        .any(|p| name.starts_with(p))
}

fn disk_note(partitions: &[Partition]) -> String {
    if partitions.iter().any(Partition::is_ntfs) {
        "Windows 11".to_string()
    } else if partitions.is_empty() {
        "vacío".to_string()
    } else if partitions.iter().any(Partition::is_linux) {
        "Linux".to_string()
    } else {
        "datos".to_string()
    }
}

fn holds_live_medium(node: &Node) -> bool {
    if node
        .mountpoints
        .as_ref()
        .is_some_and(|m| m.iter().any(|p| is_live_mount(p)))
    {
        return true;
    }
    if matches!(
        node.mountpoint.as_deref(),
        Some("/") | Some("/iso") | Some("/run/iso") | Some("/nix")
    ) {
        return true;
    }
    node.children.iter().any(holds_live_medium)
}

fn is_live_mount(path: &str) -> bool {
    matches!(path, "/" | "/iso" | "/run/iso" | "/nix")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NtfsInfo {
    pub min_bytes: u64,
    pub bitlocker: bool,
    pub dirty: bool,
}

pub fn ntfs_info(path: &str) -> NtfsInfo {
    let bitlocker =
        run_trim("blkid", &["-o", "value", "-s", "TYPE", path]).as_deref() == Some("BitLocker");
    let output = Command::new("ntfsresize")
        .args(["--info", "--force", path])
        .stdin(Stdio::null())
        .output();
    match output {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout);
            let dirty = text.to_lowercase().contains("scheduled for check")
                || text.to_lowercase().contains("hibernat");
            NtfsInfo {
                min_bytes: parse_ntfs_min(&text).unwrap_or(0),
                bitlocker,
                dirty,
            }
        }
        _ => NtfsInfo {
            min_bytes: 0,
            bitlocker,
            dirty: true,
        },
    }
}

fn parse_ntfs_min(text: &str) -> Option<u64> {
    for line in text.lines() {
        let lower = line.to_lowercase();
        if let Some(pos) = lower.find("resize at") {
            let rest = &line[pos + "resize at".len()..];
            let digits: String = rest
                .trim_start()
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            if let Ok(value) = digits.parse() {
                return Some(value);
            }
        }
    }
    None
}

pub fn efi_free(efi: &Partition) -> Option<u64> {
    if let Some(mp) = efi.mountpoints.iter().find(|m| !m.is_empty()) {
        return df_free(mp);
    }
    let dir = std::env::temp_dir().join("keel-efi");
    let _ = std::fs::create_dir_all(&dir);
    let mounted = Command::new("mount")
        .args(["-o", "ro"])
        .arg(efi.path())
        .arg(&dir)
        .stdin(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !mounted {
        return None;
    }
    let free = df_free(&dir.to_string_lossy());
    let _ = Command::new("umount")
        .arg(&dir)
        .stdin(Stdio::null())
        .status();
    free
}

fn df_free(path: &str) -> Option<u64> {
    let output = Command::new("df")
        .args(["-B1", "--output=avail"])
        .arg(path)
        .stdin(Stdio::null())
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .nth(1)
        .and_then(|line| line.trim().parse::<u64>().ok())
}

pub fn partition_uuid(path: &str) -> Option<String> {
    run_trim("blkid", &["-s", "UUID", "-o", "value", path])
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LuksPartition {
    pub path: String,
    pub uuid: String,
}

pub fn luks_partition(disk: &str) -> Option<LuksPartition> {
    let json = run_trim("lsblk", &["-J", "-o", "NAME,TYPE,FSTYPE,UUID", disk])?;
    let mut part = parse_luks(&json)?;
    if part.uuid.is_empty() {
        part.uuid = run_trim("blkid", &["-s", "UUID", "-o", "value", &part.path])?;
    }
    Some(part)
}

fn parse_luks(json: &str) -> Option<LuksPartition> {
    let tree = serde_json::from_str::<Lsblk>(json).ok()?;
    let node = tree.blockdevices.into_iter().next()?;
    let part = node
        .children
        .into_iter()
        .find(|c| c.fstype.as_deref() == Some("crypto_LUKS"))?;
    Some(LuksPartition {
        path: format!("/dev/{}", part.name),
        uuid: part.uuid.unwrap_or_default(),
    })
}

pub fn tpm2_present() -> bool {
    let dir = Path::new("/sys/class/tpm/tpm0");
    if !dir.exists() {
        return false;
    }
    match std::fs::read_to_string(dir.join("tpm_version_major")) {
        Ok(version) => version.trim() == "2",
        Err(_) => true,
    }
}

#[derive(Debug, Clone)]
pub struct RemovableMedium {
    pub path: String,
    pub label: String,
}

pub fn removable_media(target: &str) -> Vec<RemovableMedium> {
    let target_name = target.rsplit('/').next().unwrap_or("").to_string();
    let Some(json) = run_trim(
        "lsblk",
        &[
            "-J",
            "-o",
            "NAME,PATH,RM,TRAN,TYPE,FSTYPE,LABEL,SIZE,PKNAME",
        ],
    ) else {
        return Vec::new();
    };
    let Ok(tree) = serde_json::from_str::<serde_json::Value>(&json) else {
        return Vec::new();
    };

    let truthy = |value: &serde_json::Value| {
        value
            .as_bool()
            .unwrap_or_else(|| value.as_str() == Some("1"))
    };

    let mut out = Vec::new();
    let Some(disks) = tree.get("blockdevices").and_then(|d| d.as_array()) else {
        return out;
    };
    for disk in disks {
        let disk_name = disk.get("name").and_then(|v| v.as_str()).unwrap_or("");
        if disk_name == target_name {
            continue;
        }
        let removable = disk.get("rm").map(&truthy).unwrap_or(false)
            || disk.get("tran").and_then(|v| v.as_str()) == Some("usb");
        if !removable {
            continue;
        }
        let Some(parts) = disk.get("children").and_then(|c| c.as_array()) else {
            continue;
        };
        for part in parts {
            if part.get("type").and_then(|v| v.as_str()) != Some("part") {
                continue;
            }
            let path = part.get("path").and_then(|v| v.as_str()).unwrap_or("");
            if path.is_empty() {
                continue;
            }
            let Some(fstype) = part.get("fstype").and_then(|v| v.as_str()) else {
                continue;
            };
            if fstype.is_empty() || fstype == "iso9660" {
                continue;
            }
            let name = part.get("name").and_then(|v| v.as_str()).unwrap_or(path);
            let label = part
                .get("label")
                .and_then(|v| v.as_str())
                .filter(|l| !l.is_empty())
                .map(|l| l.to_string())
                .unwrap_or_else(|| name.to_string());
            let size = part
                .get("size")
                .and_then(|v| v.as_u64())
                .map(human_size)
                .unwrap_or_default();
            out.push(RemovableMedium {
                path: path.to_string(),
                label: format!("{label}  {size}"),
            });
        }
    }
    out
}

pub fn nominal_gb(bytes: u64) -> u64 {
    (bytes as f64 / 1_000_000_000.0).round() as u64
}

pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["B", "K", "M", "G", "T", "P"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_size_scales() {
        assert_eq!(human_size(0), "0 B");
        assert_eq!(human_size(1024), "1.0 K");
        assert_eq!(human_size(1024 * 1024 * 1024), "1.0 G");
    }

    #[test]
    fn disks_skip_live_medium() {
        let json = r#"{
            "blockdevices": [
                { "name": "sda", "size": 8000000000, "model": "USB", "type": "disk",
                  "mountpoint": null, "pkname": null,
                  "children": [ { "name": "sda1", "size": 7000000000, "type": "part",
                                  "mountpoint": "/iso", "pkname": "sda", "model": null,
                                  "children": [] } ] },
                { "name": "nvme0n1", "size": 500000000000, "model": "NVMe", "type": "disk",
                  "mountpoint": null, "pkname": null, "children": [] }
            ]
        }"#;
        let disks = parse_disks(json);
        assert_eq!(disks.len(), 2);
        let live = disks.iter().find(|d| d.name == "sda").unwrap();
        assert!(!live.available);
        let target = disks.iter().find(|d| d.name == "nvme0n1").unwrap();
        assert!(target.available);
    }

    #[test]
    fn finds_crypto_luks_partition_by_type() {
        let json = r#"{
            "blockdevices": [
                { "name": "vda", "type": "disk", "fstype": null, "uuid": null,
                  "children": [
                    { "name": "vda1", "type": "part", "fstype": "vfat", "uuid": "B7C1-02A5",
                      "children": [] },
                    { "name": "vda2", "type": "part", "fstype": "crypto_LUKS",
                      "uuid": "1234abcd-0000-0000-0000-000000000000", "children": [] }
                  ] }
            ]
        }"#;
        let part = parse_luks(json).unwrap();
        assert_eq!(part.path, "/dev/vda2");
        assert_eq!(part.uuid, "1234abcd-0000-0000-0000-000000000000");
    }

    #[test]
    fn no_luks_partition_is_none() {
        let json = r#"{
            "blockdevices": [
                { "name": "vda", "type": "disk", "children": [
                    { "name": "vda1", "type": "part", "fstype": "vfat", "children": [] } ] }
            ]
        }"#;
        assert!(parse_luks(json).is_none());
    }
}
