use crate::detect;
use crate::runner::{self, Cmd, Stream};
use crate::settings::Settings;
use color_eyre::eyre::{eyre, Result};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;
use zeroize::Zeroizing;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Step {
    Disk,
    Settings,
    System,
    Boot,
}

impl Step {
    pub const ALL: [Step; 4] = [Step::Disk, Step::Settings, Step::System, Step::Boot];

    pub fn label(self) -> &'static str {
        match self {
            Step::Disk => "disco",
            Step::Settings => "ajustes",
            Step::System => "sistema",
            Step::Boot => "arranque",
        }
    }
}

#[derive(Debug, Clone)]
pub struct WifiSave {
    pub ssid: String,
    pub networkmanager: bool,
}

#[derive(Debug, Clone)]
pub enum DiskMode {
    Whole,
    Alongside {
        reuse_efi: Option<String>,
        shrink: Option<ShrinkSpec>,
    },
}

#[derive(Debug, Clone)]
pub struct ShrinkSpec {
    pub ntfs: String,
    pub target_mib: u64,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Step(Step),
    Log(String),
    Error(String),
    Progress {
        built: u64,
        total: u64,
    },
    Repaint,
    Finished {
        ok: bool,
        error: Option<String>,
        log_path: PathBuf,
    },
}

#[derive(Debug, Clone)]
pub struct Plan {
    pub source: PathBuf,
    pub root: PathBuf,
    pub settings: Settings,
    pub luks_passphrase: Zeroizing<String>,
    pub recovery_key: Zeroizing<String>,
    pub luks_tpm: bool,
    pub device: String,
    pub dry_run: bool,
    pub disk_mode: DiskMode,
    pub secure_boot: bool,
    pub wifi: Option<WifiSave>,
    pub resume_from: Step,
}

impl Plan {
    pub fn installed_repo(&self) -> PathBuf {
        self.root.join("persist/keel/repo")
    }

    pub fn log_path(&self) -> PathBuf {
        PathBuf::from("/tmp/keel-install.log")
    }

    fn esp_uuid(&self) -> String {
        match &self.disk_mode {
            DiskMode::Alongside {
                reuse_efi: Some(path),
                ..
            } => detect::partition_uuid(path).unwrap_or_default(),
            _ => String::new(),
        }
    }
}

fn git_cmd(repo: &Path, args: &[&str]) -> Cmd {
    let mut cmd = Cmd::new("git")
        .arg("-c")
        .arg(format!("safe.directory={}", repo.display()))
        .arg("-C")
        .arg_path(repo);
    for arg in args {
        cmd = cmd.arg(*arg);
    }
    cmd
}

pub fn spawn(plan: Plan, tx: Sender<Msg>) -> thread::JoinHandle<()> {
    thread::spawn(move || run(plan, tx))
}

struct Reporter {
    tx: Sender<Msg>,
    file: Option<File>,
}

impl Reporter {
    fn log(&mut self, line: impl Into<String>) {
        let line = line.into();
        let _ = self.tx.send(Msg::Log(line.clone()));
        self.write(&line);
    }

    fn raw(&mut self, line: &str) {
        self.write(line);
    }

    fn emit(&mut self, line: String, is_error: bool) {
        let msg = if is_error {
            Msg::Error(line)
        } else {
            Msg::Log(line)
        };
        let _ = self.tx.send(msg);
    }

    fn write(&mut self, line: &str) {
        if let Some(file) = self.file.as_mut() {
            let _ = writeln!(file, "{line}");
        }
    }

    fn step(&self, step: Step) {
        let _ = self.tx.send(Msg::Step(step));
    }

    fn progress(&self, built: u64, total: u64) {
        let _ = self.tx.send(Msg::Progress { built, total });
    }

    fn repaint(&self) {
        let _ = self.tx.send(Msg::Repaint);
    }
}

fn run(plan: Plan, tx: Sender<Msg>) {
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(plan.log_path())
        .ok();
    let mut report = Reporter {
        tx: tx.clone(),
        file,
    };

    report.raw("== Keel installer ==");
    report.raw(&format!(
        "modo: {}",
        if plan.dry_run { "simulación" } else { "real" }
    ));
    report.raw(&format!("host: {}", plan.settings.host_name));
    report.raw(&format!("dispositivo: {}", plan.device));
    report.raw(&format!(
        "repo destino: {}",
        plan.installed_repo().display()
    ));

    match run_steps(&plan, &mut report) {
        Ok(()) => {
            let log_path = plan.log_path();
            if !plan.dry_run {
                copy_log_to_persist(&plan, &mut report);
            }
            report.log("== instalación completada ==");
            let _ = tx.send(Msg::Finished {
                ok: true,
                error: None,
                log_path,
            });
        }
        Err(err) => {
            report.log(format!("error: {err}"));
            if !plan.dry_run {
                copy_log_to_persist(&plan, &mut report);
            }
            let _ = tx.send(Msg::Finished {
                ok: false,
                error: Some(err.to_string()),
                log_path: plan.log_path(),
            });
        }
    }

    if !plan.dry_run {
        let _ = fs::remove_dir_all(plan.root.join("tmp"));
        let _ = fs::remove_dir_all(target_build_dir(&plan));
        let _ = fs::remove_dir_all("/run/keel");
    }

    if let Some(file) = report.file.as_mut() {
        let _ = file.flush();
    }
}

fn run_steps(plan: &Plan, report: &mut Reporter) -> Result<()> {
    let resume = plan.resume_from;

    let luks = if resume <= Step::Disk {
        report.step(Step::Disk);
        step_disk(plan, report)?
    } else {
        ensure_disk_ready(plan, report)?;
        detect::luks_partition(&plan.device)
            .ok_or_else(|| eyre!("no se encontró la partición cifrada en {}", plan.device))?
    };

    prepare_target_tmp(plan, report)?;

    if resume <= Step::Settings {
        report.step(Step::Settings);
        step_repo(plan, report)?;
        step_license(plan, report)?;
        step_hardware(plan, report, &luks)?;
        step_persist(plan, report)?;
    }

    if resume <= Step::System {
        ensure_target_space(plan)?;
        let _swap = ensure_build_memory(plan, report)?;
        report.step(Step::System);
        step_system(plan, report)?;
    }

    if resume <= Step::Boot {
        report.step(Step::Boot);
        step_boot(plan, report)?;
    }
    Ok(())
}

fn prepare_target_tmp(plan: &Plan, report: &mut Reporter) -> Result<()> {
    let tmp = plan.root.join("tmp");
    let build = target_build_dir(plan);
    if !plan.dry_run {
        fs::create_dir_all(&tmp)?;
        set_mode(&tmp, 0o755)?;
        fs::create_dir_all(&build)?;
        set_mode(&build, 0o700)?;
    }
    std::env::set_var("TMPDIR", &tmp);
    std::env::set_var("KEEL_ROOT", &plan.root);
    report.log(format!("$ TMPDIR={}", tmp.display()));
    Ok(())
}

fn target_build_dir(plan: &Plan) -> PathBuf {
    plan.root.join("nix/var/nix/builds")
}

const MIN_TARGET_GIB: u64 = 25;

fn ensure_target_space(plan: &Plan) -> Result<()> {
    if plan.dry_run {
        return Ok(());
    }
    let free = target_free_gib(&plan.root)?;
    if free < MIN_TARGET_GIB {
        return Err(eyre!(
            "espacio insuficiente en {}: hay {} GiB libres y se necesitan al menos {} GiB para instalar el sistema",
            plan.root.display(),
            free,
            MIN_TARGET_GIB
        ));
    }
    Ok(())
}

fn target_free_gib(root: &Path) -> Result<u64> {
    let out = runner::capture(&Cmd::new("df").arg("-Pk").arg_path(root))?;
    let text = String::from_utf8_lossy(&out.stdout);
    let avail_kb: u64 = text
        .lines()
        .last()
        .and_then(|line| line.split_whitespace().nth(3))
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| eyre!("no se pudo leer el espacio libre de {}", root.display()))?;
    Ok(avail_kb / 1024 / 1024)
}

fn mem_total_gib() -> Option<u64> {
    let text = fs::read_to_string("/proc/meminfo").ok()?;
    let kb: u64 = text
        .lines()
        .find_map(|line| line.strip_prefix("MemTotal:"))?
        .split_whitespace()
        .next()?
        .parse()
        .ok()?;
    Some(kb / 1024 / 1024)
}

const LOW_MEM_GIB: u64 = 12;

struct TempSwap {
    path: PathBuf,
}

impl TempSwap {
    fn create(plan: &Plan, report: &mut Reporter) -> Result<Self> {
        let path = plan.root.join("persist/.keel-install.swap");
        let mut sink = |_: Stream| {};
        let _ = runner::run(&Cmd::new("swapoff").arg_path(&path), &mut sink);
        let _ = fs::remove_file(&path);

        let mkswap = Cmd::new("btrfs")
            .arg("filesystem")
            .arg("mkswapfile")
            .arg("--size")
            .arg("8g")
            .arg_path(&path);
        ensure_ok(
            exec_in(plan, report, &mkswap, &plan.source)?,
            "no se pudo crear el swap temporal",
        )?;

        let swapon = Cmd::new("swapon").arg_path(&path);
        ensure_ok(
            exec_in(plan, report, &swapon, &plan.source)?,
            "no se pudo activar el swap temporal",
        )?;

        Ok(Self { path })
    }
}

impl Drop for TempSwap {
    fn drop(&mut self) {
        let _ = std::process::Command::new("swapoff")
            .arg(&self.path)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        let _ = fs::remove_file(&self.path);
    }
}

fn ensure_build_memory(plan: &Plan, report: &mut Reporter) -> Result<Option<TempSwap>> {
    if plan.dry_run {
        return Ok(None);
    }
    let ram = mem_total_gib();
    if ram.is_some_and(|g| g >= LOW_MEM_GIB) {
        return Ok(None);
    }
    let ram_label = ram
        .map(|g| format!("{g} GiB"))
        .unwrap_or_else(|| "desconocida".to_string());

    let needed = MIN_TARGET_GIB + 8;
    let free = target_free_gib(&plan.root)?;
    if free < needed {
        report.log(format!(
            "RAM: {ram_label} — sin espacio en {} para el swap temporal ({} GiB libres, se necesitan {needed} GiB); se continúa sin él",
            plan.root.display(),
            free
        ));
        return Ok(None);
    }

    report.log(format!(
        "RAM: {ram_label} — se activa un swap temporal de 8 GiB en el disco destino para la instalación"
    ));
    Ok(Some(TempSwap::create(plan, report)?))
}

fn ensure_disk_ready(plan: &Plan, report: &mut Reporter) -> Result<()> {
    if let DiskMode::Alongside { reuse_efi, .. } = &plan.disk_mode {
        return ensure_alongside_ready(plan, report, reuse_efi.as_deref());
    }
    if detect::is_mounted("/mnt") && Path::new("/dev/mapper/cryptroot").exists() {
        report.log("disco ya montado; se continúa sin reformatear");
        return Ok(());
    }

    if plan.dry_run {
        report.log(format!(
            "[simulación] $ disko --mode mount system/disko.nix --argstr device {} --argstr encrypt true",
            plan.device
        ));
        report.log("(simulación) disco re-montado sin formatear");
        return Ok(());
    }

    let pass = SecretFile::write("luks.pass", &plan.luks_passphrase)?;
    let cmd = Cmd::new("disko")
        .arg("--mode")
        .arg("mount")
        .arg("system/disko.nix")
        .arg("--argstr")
        .arg("device")
        .arg(&plan.device)
        .arg("--argstr")
        .arg("encrypt")
        .arg("true")
        .arg("--argstr")
        .arg("luksPasswordFile")
        .arg_path(pass.path());

    ensure_ok(
        exec_in(plan, report, &cmd, &plan.source)?,
        "no se pudo volver a montar el disco",
    )
}

fn ensure_alongside_ready(
    plan: &Plan,
    report: &mut Reporter,
    reuse_efi: Option<&str>,
) -> Result<()> {
    if detect::is_mounted("/mnt") && Path::new("/dev/mapper/cryptroot").exists() {
        report.log("disco ya montado; se continúa sin reformatear");
        return Ok(());
    }
    if plan.dry_run {
        report.log("(simulación) disco re-montado sin formatear");
        return Ok(());
    }
    let system = find_partition(&plan.device, "System HD")
        .ok_or_else(|| eyre!("no se encontró la partición System HD en {}", plan.device))?;
    let esp = match reuse_efi {
        Some(path) => path.to_string(),
        None => find_partition(&plan.device, "BOOT")
            .ok_or_else(|| eyre!("no se encontró la EFI en {}", plan.device))?,
    };
    let pass = SecretFile::write("luks.pass", &plan.luks_passphrase)?;
    run_ok(
        plan,
        report,
        &Cmd::new("cryptsetup")
            .arg("open")
            .arg("--key-file")
            .arg_path(pass.path())
            .arg(&system)
            .arg("cryptroot"),
        "no se pudo abrir el contenedor cifrado",
    )?;
    mount_alongside(plan, report, "/dev/mapper/cryptroot", &esp)
}

fn step_boot(plan: &Plan, report: &mut Reporter) -> Result<()> {
    if plan.secure_boot {
        report.log("instalando lanzaboote en /boot");
        step_secure_boot(plan, report)?;
        report.log("firmando el kernel y el initrd…");
        Ok(())
    } else {
        report.log("instalando systemd-boot en /boot");
        Ok(())
    }
}

fn step_disk(plan: &Plan, report: &mut Reporter) -> Result<detect::LuksPartition> {
    match &plan.disk_mode {
        DiskMode::Whole => step_disk_whole(plan, report),
        DiskMode::Alongside { reuse_efi, shrink } => {
            step_disk_alongside(plan, report, reuse_efi.as_deref(), shrink.as_ref())
        }
    }
}

fn step_disk_whole(plan: &Plan, report: &mut Reporter) -> Result<detect::LuksPartition> {
    if !plan.dry_run {
        clean_target(report);
        if let Some(luks) = detect::luks_partition(&plan.device) {
            let mut sink = |_: Stream| {};
            let _ = runner::run(&Cmd::new("wipefs").arg("-a").arg(&luks.path), &mut sink);
        }
    }

    report.log("creando las particiones y formateando el disco");
    let pass = if plan.dry_run {
        None
    } else {
        Some(SecretFile::write("luks.pass", &plan.luks_passphrase)?)
    };

    let mut cmd = Cmd::new("disko")
        .arg("--mode")
        .arg("destroy,format,mount")
        .arg("--yes-wipe-all-disks")
        .arg("system/disko.nix")
        .arg("--argstr")
        .arg("device")
        .arg(&plan.device)
        .arg("--argstr")
        .arg("encrypt")
        .arg("true");
    if let Some(pass) = &pass {
        cmd = cmd
            .arg("--argstr")
            .arg("luksPasswordFile")
            .arg_path(pass.path());
    }
    let cmd = cmd.with_display(format!(
        "disko --mode destroy,format,mount --yes-wipe-all-disks system/disko.nix --argstr device {} --argstr encrypt true",
        plan.device
    ));

    ensure_ok(
        exec_in(plan, report, &cmd, &plan.source)?,
        "no se pudo cifrar y formatear el disco",
    )?;

    let luks = if plan.dry_run {
        detect::LuksPartition {
            path: format!("{}2", plan.device),
            uuid: "00000000-0000-0000-0000-000000000000".to_string(),
        }
    } else {
        detect::luks_partition(&plan.device)
            .ok_or_else(|| eyre!("no se encontró la partición cifrada en {}", plan.device))?
    };
    report.log(format!(
        "$ partición cifrada: {} (UUID {})",
        luks.path, luks.uuid
    ));

    enroll_keys(
        plan,
        report,
        &luks.path,
        pass.as_ref().map(SecretFile::path),
    )?;
    Ok(luks)
}

fn enroll_keys(
    plan: &Plan,
    report: &mut Reporter,
    device: &str,
    pass_path: Option<&Path>,
) -> Result<()> {
    if let Some(pass_path) = pass_path {
        verify_key(
            report,
            device,
            pass_path,
            "la frase del disco no coincide con la configurada",
        )?;
    }
    step_recovery_key(plan, report, device, pass_path)?;
    if plan.luks_tpm {
        step_tpm_enroll(plan, report, device, pass_path)?;
    }
    Ok(())
}

fn step_disk_alongside(
    plan: &Plan,
    report: &mut Reporter,
    reuse_efi: Option<&str>,
    shrink: Option<&ShrinkSpec>,
) -> Result<detect::LuksPartition> {
    if plan.dry_run {
        report.log("(simulación) particiones nuevas creadas en el espacio libre");
        let luks = detect::LuksPartition {
            path: format!("{}4", plan.device),
            uuid: "00000000-0000-0000-0000-000000000000".to_string(),
        };
        enroll_keys(plan, report, &luks.path, None)?;
        return Ok(luks);
    }

    clean_target(report);
    remove_created_partitions(plan, report)?;

    if let Some(spec) = shrink {
        shrink_ntfs(plan, report, spec)?;
    }

    let pass = SecretFile::write("luks.pass", &plan.luks_passphrase)?;

    let esp = match reuse_efi {
        Some(path) => {
            report.log(format!("EFI de Windows reutilizada: {path}"));
            path.to_string()
        }
        None => create_partition(plan, report, "1G", "EF00", "BOOT")?,
    };
    let system = create_partition(plan, report, "0", "8300", "System HD")?;

    if reuse_efi.is_none() {
        run_ok(
            plan,
            report,
            &Cmd::new("mkfs.vfat").arg("-n").arg("BOOT").arg(&esp),
            "no se pudo formatear la EFI",
        )?;
    }
    let pass_path = pass.path();
    run_ok(
        plan,
        report,
        &Cmd::new("cryptsetup")
            .arg("luksFormat")
            .arg("--type")
            .arg("luks2")
            .arg("--key-file")
            .arg_path(pass_path)
            .arg(&system),
        "no se pudo cifrar la partición del sistema",
    )?;
    run_ok(
        plan,
        report,
        &Cmd::new("cryptsetup")
            .arg("open")
            .arg("--key-file")
            .arg_path(pass_path)
            .arg(&system)
            .arg("cryptroot"),
        "no se pudo abrir el contenedor cifrado",
    )?;
    let uuid = runner::capture(
        &Cmd::new("blkid")
            .arg("-s")
            .arg("UUID")
            .arg("-o")
            .arg("value")
            .arg(&system),
    )
    .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
    .unwrap_or_default();
    enroll_keys(plan, report, &system, Some(pass_path))?;
    let data = "/dev/mapper/cryptroot";

    run_ok(
        plan,
        report,
        &Cmd::new("mkfs.btrfs").arg("-L").arg("nixos").arg(data),
        "no se pudo crear el btrfs",
    )?;
    setup_subvolumes(plan, report, data)?;
    mount_alongside(plan, report, data, &esp)?;

    Ok(detect::LuksPartition { path: system, uuid })
}

fn shrink_ntfs(plan: &Plan, report: &mut Reporter, spec: &ShrinkSpec) -> Result<()> {
    let current = current_ntfs_size(&spec.ntfs);
    let target = spec.target_mib << 20;
    if current == 0 || target == 0 {
        return Err(eyre!("no se pudo calcular el tamaño final de Windows"));
    }
    if current <= target {
        report.log(format!(
            "Windows ya mide {}; no se vuelve a reducir",
            detect::gb(current)
        ));
        return Ok(());
    }
    report.log(format!(
        "Windows: se reducirá {} a {}",
        spec.ntfs,
        detect::gb(target)
    ));

    let check = Cmd::new("ntfsresize")
        .arg("--no-action")
        .arg("--force")
        .arg("--size")
        .arg(target.to_string())
        .arg(&spec.ntfs);
    run_ok(
        plan,
        report,
        &check,
        "ntfsresize rechazó el tamaño propuesto",
    )?;
    // Matar ntfsresize mientras mueve datos corrompe el NTFS.
    let real = Cmd::new("ntfsresize")
        .arg("--force")
        .arg("--size")
        .arg(target.to_string())
        .arg(&spec.ntfs)
        .without_idle_timeout();
    run_ok(plan, report, &real, "no se pudo reducir Windows")?;

    let index = partition_index(&spec.ntfs)
        .ok_or_else(|| eyre!("no se pudo leer el número de partición de {}", spec.ntfs))?;
    let entry = gpt_entry(&plan.device, index)?;
    // Una sola llamada atómica (sgdisk solo escribe si todo sale bien) y mismo GUID: el BCD lo usa.
    let cmd = Cmd::new("sgdisk")
        .arg("--set-alignment=1")
        .arg(format!("--delete={index}"))
        .arg(format!(
            "--new={index}:{}:+{}M",
            entry.first_sector, spec.target_mib
        ))
        .arg(format!("--typecode={index}:{}", entry.type_guid))
        .arg(format!("--partition-guid={index}:{}", entry.unique_guid))
        .arg(format!("--attributes={index}:=:{}", entry.attributes))
        .arg(format!("--change-name={index}:{}", entry.name))
        .arg(&plan.device);
    run_ok(
        plan,
        report,
        &cmd,
        "no se pudo reescribir la partición de Windows",
    )?;
    rescan(plan, report);
    Ok(())
}

fn current_ntfs_size(ntfs: &str) -> u64 {
    runner::capture(&Cmd::new("blockdev").arg("--getsize64").arg(ntfs))
        .ok()
        .and_then(|o| {
            String::from_utf8_lossy(&o.stdout)
                .trim()
                .parse::<u64>()
                .ok()
        })
        .unwrap_or(0)
}

fn create_partition(
    plan: &Plan,
    report: &mut Reporter,
    size: &str,
    type_code: &str,
    label: &str,
) -> Result<String> {
    let end = if size == "0" {
        "0".to_string()
    } else {
        format!("+{size}")
    };
    if let Some(existing) = find_partition(&plan.device, label) {
        return Err(eyre!(
            "{existing} ya se llama «{label}»; bórrala o elige otro disco antes de instalar"
        ));
    }
    let cmd = Cmd::new("sgdisk")
        .arg(format!("--new=0:0:{end}"))
        .arg(format!("--typecode=0:{type_code}"))
        .arg(format!("--change-name=0:{label}"))
        .arg(&plan.device);
    run_ok(plan, report, &cmd, "no se pudo crear la partición")?;
    rescan(plan, report);
    let path = find_partition(&plan.device, label)
        .ok_or_else(|| eyre!("no se encontró la partición nueva {label}"))?;
    created_partitions().push(path.clone());
    Ok(path)
}

fn find_partition(device: &str, label: &str) -> Option<String> {
    let out = runner::capture(
        &Cmd::new("lsblk")
            .arg("-J")
            .arg("-o")
            .arg("NAME,PARTLABEL")
            .arg(device),
    )
    .ok()?;
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    let children = json
        .get("blockdevices")?
        .get(0)?
        .get("children")?
        .as_array()?;
    for part in children {
        if part.get("partlabel").and_then(|v| v.as_str()) == Some(label) {
            return Some(format!("/dev/{}", part.get("name")?.as_str()?));
        }
    }
    None
}

fn partition_index(device: &str) -> Option<u32> {
    let digits: String = device
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    let digits: String = digits.chars().rev().collect();
    digits.parse().ok()
}

struct GptEntry {
    first_sector: String,
    type_guid: String,
    unique_guid: String,
    attributes: String,
    name: String,
}

fn gpt_entry(device: &str, index: u32) -> Result<GptEntry> {
    let out = runner::capture(
        &Cmd::new("sgdisk")
            .arg(format!("--info={index}"))
            .arg(device),
    )?;
    parse_gpt_entry(&String::from_utf8_lossy(&out.stdout))
        .ok_or_else(|| eyre!("no se pudo leer la partición {index} de {device}"))
}

fn parse_gpt_entry(info: &str) -> Option<GptEntry> {
    let field = |key: &str| {
        info.lines()
            .find_map(|line| line.strip_prefix(key))
            .and_then(|rest| rest.split_whitespace().next())
            .map(str::to_string)
    };
    let name = info
        .lines()
        .find_map(|line| line.strip_prefix("Partition name: "))?
        .trim()
        .trim_matches('\'')
        .to_string();
    Some(GptEntry {
        first_sector: field("First sector:")?,
        type_guid: field("Partition GUID code:")?,
        unique_guid: field("Partition unique GUID:")?,
        attributes: field("Attribute flags:")?,
        name,
    })
}

fn rescan(plan: &Plan, report: &mut Reporter) {
    let mut sink = |_: Stream| {};
    let _ = runner::run(&Cmd::new("partprobe").arg(&plan.device), &mut sink);
    let _ = runner::run(
        &Cmd::new("udevadm")
            .arg("settle")
            .arg("--timeout")
            .arg("120"),
        &mut sink,
    );
    report.repaint();
}

fn setup_subvolumes(plan: &Plan, report: &mut Reporter, data: &str) -> Result<()> {
    run_ok(
        plan,
        report,
        &Cmd::new("mount").arg(data).arg_path(&plan.root),
        "no se pudo montar el btrfs",
    )?;
    for sub in ["@rootfs", "@nix", "@home", "@persist", "@log"] {
        let path = plan.root.join(sub);
        run_ok(
            plan,
            report,
            &Cmd::new("btrfs")
                .arg("subvolume")
                .arg("create")
                .arg_path(&path),
            "no se pudo crear el subvolumen",
        )?;
    }
    let src = plan.root.join("@rootfs");
    let dst = plan.root.join("@rootfs-blank");
    run_ok(
        plan,
        report,
        &Cmd::new("btrfs")
            .arg("subvolume")
            .arg("snapshot")
            .arg("-r")
            .arg_path(&src)
            .arg_path(&dst),
        "no se pudo crear la plantilla",
    )?;
    run_ok(
        plan,
        report,
        &Cmd::new("umount").arg_path(&plan.root),
        "no se pudo desmontar el btrfs",
    )?;
    Ok(())
}

fn mount_alongside(plan: &Plan, report: &mut Reporter, data: &str, esp: &str) -> Result<()> {
    let opts = "noatime,compress=zstd";
    let mounts = [
        ("@rootfs", ""),
        ("@nix", "nix"),
        ("@home", "home"),
        ("@persist", "persist"),
        ("@log", "var/log"),
    ];
    for (sub, rel) in mounts {
        let target = if rel.is_empty() {
            plan.root.clone()
        } else {
            plan.root.join(rel)
        };
        if !rel.is_empty() {
            fs::create_dir_all(&target)?;
        }
        let cmd = Cmd::new("mount")
            .arg("-o")
            .arg(format!("subvol={sub},{opts}"))
            .arg(data)
            .arg_path(&target);
        run_ok(plan, report, &cmd, "no se pudo montar un subvolumen")?;
    }
    let boot = plan.root.join("boot");
    fs::create_dir_all(&boot)?;
    run_ok(
        plan,
        report,
        &Cmd::new("mount").arg(esp).arg_path(&boot),
        "no se pudo montar la EFI",
    )?;
    Ok(())
}

fn run_ok(plan: &Plan, report: &mut Reporter, cmd: &Cmd, what: &str) -> Result<()> {
    ensure_ok(exec_in(plan, report, cmd, &plan.source)?, what)
}

fn clean_target(report: &mut Reporter) {
    report.log("$ umount -R /mnt; cryptsetup close cryptroot   (limpieza previa)");
    let mut sink = |_: Stream| {};
    let _ = runner::run(&Cmd::new("umount").arg("-R").arg("/mnt"), &mut sink);
    let _ = runner::run(
        &Cmd::new("cryptsetup").arg("close").arg("cryptroot"),
        &mut sink,
    );
    let _ = fs::remove_dir_all("/run/keel");
}

fn remove_created_partitions(plan: &Plan, report: &mut Reporter) -> Result<()> {
    let created = std::mem::take(&mut *created_partitions());
    for path in created.iter().rev() {
        let index = partition_index(path)
            .ok_or_else(|| eyre!("no se pudo leer el número de partición de {path}"))?;
        report.log(format!(
            "$ wipefs -a {path}; sgdisk --delete={index}   (intento anterior)"
        ));
        let mut sink = |_: Stream| {};
        let _ = runner::run(&Cmd::new("wipefs").arg("-a").arg(path), &mut sink);
        run_ok(
            plan,
            report,
            &Cmd::new("sgdisk")
                .arg(format!("--delete={index}"))
                .arg(&plan.device),
            "no se pudo borrar la partición del intento anterior",
        )?;
    }
    if !created.is_empty() {
        rescan(plan, report);
    }
    Ok(())
}

// Solo se borra lo que creó este proceso: junto a Windows, el resto del disco no es nuestro.
fn created_partitions() -> std::sync::MutexGuard<'static, Vec<String>> {
    static CREATED: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
    CREATED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn verify_key(report: &mut Reporter, device: &str, key_file: &Path, what: &str) -> Result<()> {
    let cmd = Cmd::new("cryptsetup")
        .arg("open")
        .arg("--test-passphrase")
        .arg("--key-file")
        .arg_path(key_file)
        .arg(device)
        .with_display(format!(
            "cryptsetup open --test-passphrase --key-file {} {device}",
            key_file.display()
        ));
    report.log(format!("$ {}", cmd.display));
    let mut sink = |_: Stream| {};
    let code = runner::run(&cmd, &mut sink);
    report.repaint();
    ensure_ok(code, what)
}

struct SecretFile {
    path: PathBuf,
}

impl SecretFile {
    fn write(name: &str, secret: &str) -> Result<Self> {
        let dir = Path::new("/run/keel");
        fs::create_dir_all(dir)?;
        set_mode(dir, 0o700)?;
        let path = dir.join(name);
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(&path)
            .map_err(|err| eyre!("no se pudo crear {}: {err}", path.display()))?;
        file.write_all(secret.as_bytes())?;
        file.flush()?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for SecretFile {
    fn drop(&mut self) {
        let _ = std::process::Command::new("shred")
            .arg("-u")
            .arg("--")
            .arg(&self.path)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
}

fn cryptsetup_has_new_keyfile() -> bool {
    static CACHE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *CACHE.get_or_init(|| {
        match std::process::Command::new("cryptsetup")
            .arg("luksAddKey")
            .arg("--help")
            .stdin(std::process::Stdio::null())
            .output()
        {
            Ok(out) => {
                String::from_utf8_lossy(&out.stdout).contains("--new-keyfile")
                    || String::from_utf8_lossy(&out.stderr).contains("--new-keyfile")
            }
            Err(_) => false,
        }
    })
}

fn step_recovery_key(
    plan: &Plan,
    report: &mut Reporter,
    device: &str,
    pass_path: Option<&Path>,
) -> Result<()> {
    if plan.dry_run {
        report.log(format!(
            "(simulación) cryptsetup luksAddKey {device}  (clave de recuperación)"
        ));
        thread::sleep(Duration::from_millis(300));
        return Ok(());
    }

    let pass_path = pass_path.ok_or_else(|| eyre!("falta el archivo de la frase del disco"))?;
    let recovery = SecretFile::write("luks.recovery", &plan.recovery_key)?;

    let mut cmd = Cmd::new("cryptsetup").arg("luksAddKey");
    if cryptsetup_has_new_keyfile() {
        cmd = cmd
            .arg("--key-file")
            .arg_path(pass_path)
            .arg("--new-keyfile")
            .arg_path(recovery.path())
            .arg(device);
    } else {
        cmd = cmd
            .arg("--key-file")
            .arg_path(pass_path)
            .arg(device)
            .arg_path(recovery.path());
    }
    let cmd = cmd.with_display(format!(
        "cryptsetup luksAddKey --key-file {} {} {}",
        pass_path.display(),
        device,
        recovery.path().display()
    ));
    report.log(format!("$ {}", cmd.display));

    let code = run_logged(report, &cmd, |_| {});
    ensure_ok(
        code,
        &format!("no se pudo añadir la clave de recuperación a {device}"),
    )?;

    verify_key(
        report,
        device,
        recovery.path(),
        "la clave de recuperación no coincide con la mostrada",
    )
}

fn step_tpm_enroll(
    plan: &Plan,
    report: &mut Reporter,
    device: &str,
    pass_path: Option<&Path>,
) -> Result<()> {
    if plan.dry_run {
        report.log(format!(
            "(simulación) systemd-cryptenroll --tpm2-device=auto --tpm2-pcrs= {device}"
        ));
        thread::sleep(Duration::from_millis(300));
        return Ok(());
    }

    let pass_path = pass_path.ok_or_else(|| eyre!("falta el archivo de la frase del disco"))?;
    let cmd = Cmd::new("systemd-cryptenroll")
        .arg("--tpm2-device=auto")
        .arg("--tpm2-pcrs=")
        .arg("--unlock-key-file")
        .arg_path(pass_path)
        .arg(device)
        .with_display(format!(
            "systemd-cryptenroll --tpm2-device=auto --tpm2-pcrs= --unlock-key-file {} {device}",
            pass_path.display()
        ));
    report.log(format!("$ {}", cmd.display));

    let code = run_logged(report, &cmd, |_| {});
    ensure_ok(
        code,
        &format!("no se pudo inscribir el token TPM2 puente de {device}"),
    )
}

fn step_repo(plan: &Plan, report: &mut Reporter) -> Result<()> {
    let dest = plan.installed_repo();
    if plan.dry_run {
        report.log(format!(
            "$ cp -a {} {}",
            plan.source.display(),
            dest.display()
        ));
        fs::create_dir_all(&dest)?;
        report.log("(simulación) repositorio copiado");
        return Ok(());
    }

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    if dest.exists() {
        fs::remove_dir_all(&dest)?;
    }
    let cmd = Cmd::new("cp")
        .arg("-a")
        .arg_path(&plan.source)
        .arg_path(&dest)
        .with_display(format!(
            "cp -a {} {}",
            plan.source.display(),
            dest.display()
        ));
    let code = exec_in(
        plan,
        report,
        &cmd,
        plan.source.parent().unwrap_or(&plan.root),
    )?;
    ensure_ok(code, "no se pudo copiar el repositorio")?;
    ensure_repo_ready(plan, report, &dest)?;
    Ok(())
}

fn ensure_repo_ready(plan: &Plan, report: &mut Reporter, repo: &Path) -> Result<()> {
    let chmod = Cmd::new("chmod")
        .arg("-R")
        .arg("u+w")
        .arg_path(repo)
        .with_display(format!("chmod -R u+w {}", repo.display()));
    let _ = exec_in(plan, report, &chmod, &plan.source);

    if repo.join(".git").exists() {
        return Ok(());
    }

    report.log("git: repo sin .git; se crea un commit inicial");

    let init = git_cmd(repo, &["init"]);
    ensure_ok(
        exec_in(plan, report, &init, &plan.source)?,
        "git init falló",
    )?;
    let add = git_cmd(repo, &["add", "-A"]);
    ensure_ok(exec_in(plan, report, &add, &plan.source)?, "git add falló")?;
    let commit = git_cmd(
        repo,
        &[
            "-c",
            "user.name=nixdots-installer",
            "-c",
            "user.email=installer@localhost",
            "commit",
            "-m",
            "keel: initial import",
        ],
    );
    ensure_ok(
        exec_in(plan, report, &commit, &plan.source)?,
        "git commit falló",
    )
}

fn step_license(plan: &Plan, report: &mut Reporter) -> Result<()> {
    let repo = plan.installed_repo();
    let lic_dst = plan.root.join("etc/keel");
    let lic_src = repo.join("LICENSE");
    report.log(format!(
        "$ cp {} {}/LICENSE",
        lic_src.display(),
        lic_dst.display()
    ));
    if !plan.dry_run && lic_src.exists() {
        fs::create_dir_all(&lic_dst)?;
        fs::copy(&lic_src, lic_dst.join("LICENSE"))?;
    }
    Ok(())
}

fn step_hardware(plan: &Plan, report: &mut Reporter, luks: &detect::LuksPartition) -> Result<()> {
    let dir = plan.root.join("persist/keel");
    fs::create_dir_all(&dir)?;

    let facter = dir.join("facter.json");
    let cmd = Cmd::new("nixos-facter")
        .arg("-o")
        .arg_path(&facter)
        .with_display(format!("nixos-facter -o {}", facter.display()));
    ensure_ok(
        exec_in(plan, report, &cmd, &plan.source)?,
        "nixos-facter falló",
    )?;
    if !plan.dry_run {
        set_mode(&facter, 0o644)?;
    }

    let machine = dir.join("machine.toml");
    let mut fields = serde_json::Map::new();
    fields.insert("hostName".into(), plan.settings.host_name.clone().into());
    fields.insert("encrypt".into(), true.into());
    fields.insert("luksUuid".into(), luks.uuid.clone().into());
    fields.insert("espUuid".into(), plan.esp_uuid().into());
    fields.insert("secureBoot".into(), plan.secure_boot.into());
    fields.insert("tpmUnlock".into(), plan.luks_tpm.into());
    fields.insert("extras".into(), serde_json::json!({}));
    if !plan.settings.default_locale.is_empty() {
        fields.insert(
            "defaultLocale".into(),
            plan.settings.default_locale.clone().into(),
        );
    }
    if !plan.settings.time_zone.is_empty() {
        fields.insert("timeZone".into(), plan.settings.time_zone.clone().into());
    }
    if !plan.settings.keyboard_layout.is_empty() {
        fields.insert(
            "keyboardLayout".into(),
            plan.settings.keyboard_layout.clone().into(),
        );
    }
    fields.insert(
        "keyboardVariant".into(),
        plan.settings.keyboard_variant.clone().into(),
    );
    if !plan.settings.pretty_host_name.is_empty() {
        fields.insert(
            "prettyHostName".into(),
            plan.settings.pretty_host_name.clone().into(),
        );
    }
    report.log(format!("$ escribir {}", machine.display()));
    nixcore::machine::merge_fields_at(&machine, fields)?;
    if !plan.dry_run {
        set_mode(&machine, 0o644)?;
    }
    Ok(())
}

fn step_persist(plan: &Plan, report: &mut Reporter) -> Result<()> {
    let repo = plan.installed_repo();
    let persist = plan.root.join("persist");

    let expr = format!(
        "{}#nixosConfigurations.{}.config.environment.persistence",
        repo.display(),
        plan.settings.host_name
    );
    let apply = r#"p: { "/persist" = let e = p."/persist" or {}; in { directories = map (d: { directory = d.directory; user = d.user or null; group = d.group or null; mode = d.mode or null; }) (e.directories or []); files = map (f: { file = f.file; }) (e.files or []); }; }"#;
    let cmd = Cmd::new("nix")
        .arg("eval")
        .arg("--impure")
        .arg("--extra-experimental-features")
        .arg("nix-command flakes")
        .arg("--json")
        .arg("--apply")
        .arg(apply)
        .arg(&expr)
        .with_display(format!("nix eval --impure --json --apply <filtro> {expr}"));

    if plan.dry_run {
        report.log(format!("$ {}", cmd.display));
        fs::create_dir_all(persist.join("etc/NetworkManager/system-connections"))?;
        fs::create_dir_all(persist.join("secrets"))?;
        report.log("(simulación) /persist preparado");
        persist_wifi(plan, report)?;
        return Ok(());
    }

    let mut captured = String::new();
    let mut eval_err = String::new();
    let code = exec_capture(plan, report, &cmd, &repo, &mut captured, &mut eval_err)?;
    if code != 0 {
        report_eval_failure(plan, report, &repo, &eval_err);
    }
    ensure_ok(code, "no se pudo evaluar environment.persistence")?;
    let value: serde_json::Value =
        serde_json::from_str(&captured).map_err(|e| eyre!("persistencia ilegible: {e}"))?;
    let entries = value
        .get("/persist")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    if let Some(dirs) = entries.get("directories").and_then(|d| d.as_array()) {
        let mut log_dup = false;
        for entry in dirs {
            if entry_path(entry).as_deref() == Some("/var/log") {
                log_dup = true;
            }
            create_persist_dir(report, &persist, entry)?;
        }
        if log_dup {
            report.log("aviso: /var/log definido dos veces: subvolumen @log e impermanence");
        }
    }
    if let Some(files) = entries.get("files").and_then(|f| f.as_array()) {
        for entry in files {
            create_persist_file(report, &persist, entry)?;
        }
    }

    let secrets = persist.join("secrets");
    fs::create_dir_all(&secrets)?;
    set_mode(&secrets, 0o711)?;

    persist_wifi(plan, report)?;
    Ok(())
}

fn entry_path(entry: &serde_json::Value) -> Option<String> {
    if let Some(path) = entry.as_str() {
        Some(path.to_string())
    } else {
        entry
            .get("directory")
            .or_else(|| entry.get("file"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }
}

fn create_persist_dir(
    report: &mut Reporter,
    persist: &Path,
    entry: &serde_json::Value,
) -> Result<()> {
    let Some(path) = entry_path(entry) else {
        return Ok(());
    };
    let target = persist.join(path.trim_start_matches('/'));
    fs::create_dir_all(&target)?;
    let mode = if path.contains("NetworkManager/system-connections") {
        0o700
    } else {
        entry
            .get("mode")
            .and_then(|m| m.as_u64())
            .map(|m| m as u32)
            .unwrap_or(0o755)
    };
    set_mode(&target, mode)?;

    if let Some(user) = entry.get("user").and_then(|u| u.as_str()) {
        let group = entry.get("group").and_then(|g| g.as_str()).unwrap_or(user);
        let owner = format!("{user}:{group}");
        if chown(&target, &owner).is_err() {
            report.log(format!(
                "aviso: no se pudo fijar el dueño {owner} en {}",
                target.display()
            ));
        }
    }
    report.log(format!("$ mkdir -p {}", target.display()));
    Ok(())
}

fn create_persist_file(
    report: &mut Reporter,
    persist: &Path,
    entry: &serde_json::Value,
) -> Result<()> {
    let Some(path) = entry_path(entry) else {
        return Ok(());
    };
    if path.ends_with("etc/machine-id") {
        report.log("$ systemd-machine-id-setup --root=/mnt/persist");
        let code = runner::run(
            &Cmd::new("systemd-machine-id-setup")
                .arg("--root")
                .arg_path(persist)
                .with_display("systemd-machine-id-setup --root=/mnt/persist"),
            &mut |_| {},
        );
        if code != 0 {
            let target = persist.join(path.trim_start_matches('/'));
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&target, "")?;
        }
    } else {
        let target = persist.join(path.trim_start_matches('/'));
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        if !target.exists() {
            fs::write(&target, "")?;
        }
    }
    Ok(())
}

fn persist_wifi(plan: &Plan, report: &mut Reporter) -> Result<()> {
    let Some(wifi) = &plan.wifi else {
        return Ok(());
    };
    if !wifi.networkmanager {
        report.log(format!(
            "Wi-Fi {ssid}: guardada por iwd, reconecta tras instalar",
            ssid = wifi.ssid
        ));
        return Ok(());
    }
    let src = PathBuf::from(format!(
        "/etc/NetworkManager/system-connections/{}.nmconnection",
        wifi.ssid
    ));
    let dst_dir = plan
        .root
        .join("persist/etc/NetworkManager/system-connections");
    fs::create_dir_all(&dst_dir)?;
    set_mode(&dst_dir, 0o700)?;
    let dst = dst_dir.join(format!("{}.nmconnection", wifi.ssid));
    report.log(format!("$ cp {} {}", src.display(), dst.display()));
    if plan.dry_run {
        fs::write(&dst, "")?;
    } else {
        match fs::copy(&src, &dst) {
            Ok(_) => set_mode(&dst, 0o600)?,
            Err(err) => report.log(format!("aviso: no se pudo copiar la conexión Wi-Fi: {err}")),
        }
    }
    Ok(())
}

fn chown(path: &Path, owner: &str) -> std::io::Result<()> {
    let status = std::process::Command::new("chown")
        .arg("-R")
        .arg(owner)
        .arg(path)
        .stdin(std::process::Stdio::null())
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other("chown falló"))
    }
}

fn step_secure_boot(plan: &Plan, report: &mut Reporter) -> Result<()> {
    let system = detect::nix_system().unwrap_or_default();
    if !system.starts_with("x86_64") {
        return Err(eyre!(
            "lanzaboote no soporta {system}; secure boot se detiene en lugar de improvisar"
        ));
    }

    let keys = plan.root.join("persist/var/lib/sbctl");
    let keys_dir = keys.join("keys");
    if keys_dir.is_dir() {
        report.log("sbctl: llaves ya presentes");
        return Ok(());
    }

    report.log("generando claves de Secure Boot en /mnt/persist/var/lib/sbctl");
    let cmd = Cmd::new("sbctl")
        .arg("create-keys")
        .arg("--directory")
        .arg_path(&keys)
        .with_display("sbctl create-keys --directory /mnt/persist/var/lib/sbctl");
    let code = exec_in(plan, report, &cmd, &plan.source)?;
    ensure_ok(code, "sbctl create-keys falló")?;
    Ok(())
}

fn step_system(plan: &Plan, report: &mut Reporter) -> Result<()> {
    let repo = plan.installed_repo();
    let flake = format!("{}#keel", repo.display());
    let build = target_build_dir(plan);

    let mut cmd = Cmd::new("nixos-install")
        .arg("--flake")
        .arg(&flake)
        .arg("--root")
        .arg_path(&plan.root)
        .arg("--no-root-passwd")
        .arg("--impure")
        .arg("--option")
        .arg("build-dir")
        .arg_path(&build)
        .arg("--log-format")
        .arg("internal-json");

    let mut display = format!(
        "nixos-install --flake {flake} --root {} --no-root-passwd --option build-dir {}",
        plan.root.display(),
        build.display()
    );

    if mem_total_gib().is_none_or(|g| g < LOW_MEM_GIB) {
        cmd = cmd
            .arg("--max-jobs")
            .arg("2")
            .arg("--option")
            .arg("max-substitution-jobs")
            .arg("4")
            .arg("--option")
            .arg("http-connections")
            .arg("8");
        display
            .push_str(" --max-jobs 2 --option max-substitution-jobs 4 --option http-connections 8");
    }
    let cmd = cmd.with_display(display);

    let code = run_with_progress(plan, report, &cmd);
    if code == 137 {
        return Err(eyre!(
            "nixos-install se quedó sin memoria (código 137). Da más RAM a la máquina (8 GB o más) y pulsa Reintentar."
        ));
    }
    ensure_ok(code, "nixos-install falló")?;
    Ok(())
}

fn run_with_progress(plan: &Plan, report: &mut Reporter, cmd: &Cmd) -> i32 {
    report.log(format!("$ {}", cmd.display));
    if plan.dry_run {
        simulate_build(report);
        return 0;
    }
    run_logged(report, cmd, |_| {})
}

fn run_logged(report: &mut Reporter, cmd: &Cmd, mut capture: impl FnMut(&Stream)) -> i32 {
    let mut nixlog = NixLog::default();
    let code = runner::run(cmd, &mut |out: Stream| {
        capture(&out);
        let (Stream::Stdout(line) | Stream::Stderr(line)) = out;
        report.raw(&line);
        let (display, progress) = nixlog.feed(&line);
        if let Some((built, total)) = progress {
            report.progress(built, total);
        }
        for (is_err, d) in display {
            report.emit(d, is_err);
        }
    });
    report.repaint();
    code
}

fn command_line(plan: &Plan, cmd: &Cmd) -> String {
    if plan.dry_run {
        format!("[simulación] $ {}", cmd.display)
    } else {
        format!("$ {}", cmd.display)
    }
}

fn exec_in(plan: &Plan, report: &mut Reporter, cmd: &Cmd, cwd: &Path) -> Result<i32> {
    report.log(command_line(plan, cmd));
    if plan.dry_run {
        report.log("(simulación) comando no ejecutado");
        thread::sleep(Duration::from_millis(500));
        return Ok(0);
    }

    let mut wrapped = Cmd::new("sh")
        .arg("-c")
        .arg(format!(
            "cd {} && exec {}",
            shell_quote(&cwd.to_string_lossy()),
            shell_command(cmd)
        ))
        .with_display(cmd.display.clone());
    wrapped.idle_timeout = cmd.idle_timeout;
    let code = run_logged(report, &wrapped, |_| {});
    Ok(code)
}

fn exec_capture(
    plan: &Plan,
    report: &mut Reporter,
    cmd: &Cmd,
    cwd: &Path,
    captured: &mut String,
    err: &mut String,
) -> Result<i32> {
    report.log(command_line(plan, cmd));
    if plan.dry_run {
        report.log("(simulación) comando no ejecutado");
        return Ok(0);
    }
    let wrapped = Cmd::new("sh").arg("-c").arg(format!(
        "cd {} && exec {}",
        shell_quote(&cwd.to_string_lossy()),
        shell_command(cmd)
    ));
    Ok(run_logged(report, &wrapped, |out| {
        let (buffer, line) = match out {
            Stream::Stdout(line) => (&mut *captured, line),
            Stream::Stderr(line) => (&mut *err, line),
        };
        buffer.push_str(line);
        buffer.push('\n');
    }))
}

fn report_eval_failure(plan: &Plan, report: &mut Reporter, repo: &Path, err: &str) {
    match missing_path_hint(err) {
        Some(path) => report.log(format!(
            "error: falta el archivo {path} en {} (revisa el commit del repo)",
            repo.display()
        )),
        None => report.log("error: la evaluación del repo falló; revisa el log"),
    }

    let rev = git_cmd(repo, &["log", "-1", "--oneline"]);
    let mut out = String::new();
    let mut git_err = String::new();
    if exec_capture(plan, report, &rev, &plan.source, &mut out, &mut git_err).unwrap_or(1) == 0 {
        let line = out.trim();
        if !line.is_empty() {
            report.log(format!("repo en /mnt: {line}"));
        }
    }
}

fn missing_path_hint(stderr: &str) -> Option<String> {
    for line in stderr.lines() {
        if !line.contains("does not exist") {
            continue;
        }
        if let (Some(a), Some(b)) = (line.find('\''), line.rfind('\'')) {
            if b > a + 1 {
                return Some(line[a + 1..b].to_string());
            }
        }
        return Some(line.trim().to_string());
    }
    None
}

fn ensure_ok(code: i32, what: &str) -> Result<()> {
    if code == 0 {
        Ok(())
    } else {
        Err(eyre!("{what} (código {code})"))
    }
}

#[derive(Default)]
struct NixLog {
    activities: std::collections::HashMap<u64, u64>,
}

type NixLogOut = (Vec<(bool, String)>, Option<(u64, u64)>);

impl NixLog {
    fn feed(&mut self, line: &str) -> NixLogOut {
        let line = line.trim_end();
        let Some(json) = line.strip_prefix("@nix ") else {
            let text = translate_plain(line).unwrap_or_else(|| strip_ansi(line));
            return (vec![(false, text)], None);
        };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else {
            return (Vec::new(), None);
        };
        let id = v.get("id").and_then(|i| i.as_u64());
        let ty = v.get("type").and_then(|t| t.as_u64()).unwrap_or(0);
        let fields = v.get("fields").and_then(|f| f.as_array());
        let field = |i: usize| -> Option<String> {
            fields
                .and_then(|f| f.get(i))
                .and_then(|s| s.as_str())
                .map(str::to_string)
        };
        match v.get("action").and_then(|a| a.as_str()) {
            Some("start") => {
                if let Some(id) = id {
                    self.activities.insert(id, ty);
                }
                let disp = match ty {
                    105 => field(0).map(|p| format!("compilando {}", store_name(&p))),
                    108 => field(0).map(|p| {
                        let name = store_name(&p);
                        match field(1) {
                            Some(src) if src.starts_with("local://") => {
                                format!("copiando {name}")
                            }
                            Some(src) => format!("descargando {name} · {}", cache_host(&src)),
                            None => format!("descargando {name}"),
                        }
                    }),
                    _ => None,
                };
                (disp.map(|d| vec![(false, d)]).unwrap_or_default(), None)
            }
            Some("result") if ty == 105 => {
                if let Some(id) = id {
                    if matches!(self.activities.get(&id), Some(104) | Some(103)) {
                        let done = field(0).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
                        let expected = field(1).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
                        if expected > 0 {
                            return (Vec::new(), Some((done, expected)));
                        }
                    }
                }
                (Vec::new(), None)
            }
            Some("msg") => {
                let level = v.get("level").and_then(|l| l.as_u64()).unwrap_or(3);
                if level > 3 {
                    return (Vec::new(), None);
                }
                let Some(text) = v
                    .get("msg")
                    .or_else(|| v.get("text"))
                    .and_then(|t| t.as_str())
                else {
                    return (Vec::new(), None);
                };
                let is_error = level == 0;
                let lines = text.lines().map(|l| (is_error, strip_ansi(l))).collect();
                (lines, None)
            }
            _ => (Vec::new(), None),
        }
    }
}

fn translate_plain(line: &str) -> Option<String> {
    let (verb, rest) = if let Some(rest) = line.strip_prefix("building '") {
        ("compilando", rest)
    } else {
        ("copiando", line.strip_prefix("copying path '")?)
    };
    let path = rest.split('\'').next()?;
    let name = store_name(path.strip_suffix(".drv").unwrap_or(path));
    Some(format!("{verb} {name}"))
}

fn store_name(p: &str) -> String {
    let base = p.rsplit('/').next().unwrap_or(p);
    match base.split_once('-') {
        Some((hash, name)) if hash.len() == 32 => name.to_string(),
        _ => base.to_string(),
    }
}

fn cache_host(url: &str) -> String {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    rest.split('/').next().unwrap_or(rest).to_string()
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&n) = chars.peek() {
                    chars.next();
                    if n.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

pub fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn shell_command(cmd: &Cmd) -> String {
    let mut parts = vec![shell_quote(&cmd.program)];
    for arg in &cmd.args {
        parts.push(shell_quote(arg));
    }
    parts.join(" ")
}

pub fn check_source(repo: &Path) -> Result<(), String> {
    let tmp = std::env::temp_dir().join("keel-eval");
    let _ = fs::remove_dir_all(&tmp);
    copy_tree(repo, &tmp).map_err(|err| format!("no se pudo copiar el repo: {err}"))?;
    let _ = std::process::Command::new("chmod")
        .args(["-R", "u+w"])
        .arg(&tmp)
        .status();

    ensure_git_tree(&tmp);

    let expr = format!(
        "{}#nixosConfigurations.keel.config.system.build.toplevel.drvPath",
        tmp.display()
    );
    let output = std::process::Command::new("nix")
        .args([
            "eval",
            "--raw",
            "--impure",
            "--extra-experimental-features",
            "nix-command flakes",
        ])
        .arg(&expr)
        .env("KEEL_ROOT", &tmp)
        .env("NIX_CONFIG", runner::nix_config())
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|err| format!("no se pudo ejecutar nix eval: {err}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(last_error_line(&String::from_utf8_lossy(&output.stderr)))
    }
}

fn copy_tree(src: &Path, dest: &Path) -> Result<(), std::io::Error> {
    let status = std::process::Command::new("cp")
        .arg("-a")
        .arg(src)
        .arg(dest)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other("cp -a falló"))
    }
}

fn ensure_git_tree(repo: &Path) {
    if repo.join(".git").exists() {
        return;
    }
    let run = |args: &[&str]| {
        let _ = std::process::Command::new("git")
            .args(args)
            .arg(repo)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    };
    run(&["init", "-q"]);
    run(&["add", "-A"]);
    run(&[
        "-c",
        "user.name=keel-installer",
        "-c",
        "user.email=installer@localhost",
        "commit",
        "-q",
        "-m",
        "keel: check snapshot",
    ]);
}

fn last_error_line(stderr: &str) -> String {
    let lines: Vec<&str> = stderr.lines().filter(|l| !l.trim().is_empty()).collect();
    lines
        .iter()
        .rev()
        .find(|l| l.contains("error:"))
        .or_else(|| lines.last())
        .map(|l| l.trim().to_string())
        .unwrap_or_else(|| "la configuración no evalúa".to_string())
}

fn set_mode(path: &Path, mode: u32) -> Result<()> {
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(mode);
    fs::set_permissions(path, perms)?;
    Ok(())
}

fn copy_log_to_persist(plan: &Plan, report: &mut Reporter) {
    let dest_dir = plan.root.join("var/log");
    if !dest_dir.is_dir() {
        return;
    }
    let dest = dest_dir.join("keel-install.log");
    match fs::copy(plan.log_path(), &dest) {
        Ok(_) => report.log(format!("log copiado a {}", dest.display())),
        Err(err) => report.log(format!("no se pudo copiar el log: {err}")),
    }
}

fn simulate_build(report: &mut Reporter) {
    const PKGS: [&str; 8] = [
        "glibc-2.42",
        "gcc-15.2.0",
        "mesa-25.2.1",
        "gnome-shell-49.0",
        "firefox-143.0",
        "wezterm-20240203",
        "python3-3.13.7",
        "linux-6.16.4",
    ];
    let steps = 100u64;
    for i in 1..=steps {
        report.progress(i, steps);
        if i % 20 == 0 {
            let pkg = PKGS[(i as usize / 20 - 1) % PKGS.len()];
            let raw = format!("building '/nix/store/{i:0>32}-{pkg}.drv'...");
            report.log(translate_plain(&raw).unwrap_or(raw));
        }
        thread::sleep(Duration::from_millis(180));
    }
    report.log("(simulación) sistema construido");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_have_labels() {
        assert_eq!(Step::ALL.len(), 4);
        assert_eq!(Step::Disk.label(), "disco");
        assert_eq!(Step::Settings.label(), "ajustes");
        assert_eq!(Step::System.label(), "sistema");
        assert_eq!(Step::Boot.label(), "arranque");
    }
}
