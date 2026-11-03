use crate::detect::{self, Disk};
use crate::disk::{self, DiskDecision};
use crate::install::{self, Msg as InstallMsg, Plan, Step};
use crate::keyboard;
use crate::net;
use crate::settings::Settings;
use crate::theme::{self, Palette};
use crate::widgets::{self, Btn, ButtonRow};
use color_eyre::eyre::Result;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};
use tui_input::Input;
use zeroize::Zeroizing;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Splash,
    License,
    Network,
    Locale,
    Keyboard,
    Timezone,
    Host,
    Disk,
    Encryption,
    Summary,
    Installing,
    Done,
}

impl Screen {
    pub fn flow(show_network: bool) -> Vec<Screen> {
        let mut steps = vec![
            Screen::Locale,
            Screen::Keyboard,
            Screen::Timezone,
            Screen::Host,
            Screen::Disk,
            Screen::Encryption,
            Screen::Summary,
        ];
        if show_network {
            steps.insert(0, Screen::Network);
        }
        steps
    }

    pub fn title(self) -> &'static str {
        match self {
            Screen::Splash => "Inicio",
            Screen::License => "Licencia",
            Screen::Network => "Red",
            Screen::Locale => "Lenguaje",
            Screen::Keyboard => "Teclado",
            Screen::Timezone => "Zona horaria",
            Screen::Host => "Equipo",
            Screen::Disk => "Disco",
            Screen::Encryption => "Cifrado",
            Screen::Summary => "Resumen",
            Screen::Installing => "Instalando",
            Screen::Done => "Listo",
        }
    }

    fn next(self, show_network: bool) -> Option<Screen> {
        match self {
            Screen::Splash => Some(Screen::License),
            Screen::License => Some(if show_network {
                Screen::Network
            } else {
                Screen::Locale
            }),
            Screen::Network => Some(Screen::Locale),
            Screen::Locale => Some(Screen::Keyboard),
            Screen::Keyboard => Some(Screen::Timezone),
            Screen::Timezone => Some(Screen::Host),
            Screen::Host => Some(Screen::Disk),
            Screen::Disk => Some(Screen::Encryption),
            Screen::Encryption => Some(Screen::Summary),
            Screen::Summary => Some(Screen::Installing),
            _ => None,
        }
    }

    fn previous(self, show_network: bool) -> Option<Screen> {
        match self {
            Screen::Splash | Screen::Installing | Screen::Done => None,
            Screen::License => Some(Screen::Splash),
            Screen::Network => Some(Screen::License),
            Screen::Locale => Some(if show_network {
                Screen::Network
            } else {
                Screen::License
            }),
            Screen::Keyboard => Some(Screen::Locale),
            Screen::Timezone => Some(Screen::Keyboard),
            Screen::Host => Some(Screen::Timezone),
            Screen::Disk => Some(Screen::Host),
            Screen::Encryption => Some(Screen::Disk),
            Screen::Summary => Some(Screen::Encryption),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusKind {
    Field { last: bool },
    List,
    Checkbox,
    Buttons,
    Dialog,
}

impl App {
    pub fn focus_kind(&self) -> FocusKind {
        if self.license_modal
            || self.quit.is_some()
            || self.install_cancel
            || self.disks.erase_modal
        {
            return FocusKind::Dialog;
        }
        if self.focus_buttons {
            return FocusKind::Buttons;
        }
        match self.screen {
            Screen::Host => match self.host.focus {
                0 => FocusKind::Checkbox,
                _ => FocusKind::Field { last: true },
            },
            Screen::Encryption => match self.enc.focus {
                ENC_TPM | ENC_NOTED => FocusKind::Checkbox,
                _ => FocusKind::Field {
                    last: self.enc.focus == ENC_CONFIRM,
                },
            },
            Screen::Keyboard if self.key_test_focus => FocusKind::Field { last: true },
            Screen::Locale | Screen::Timezone | Screen::Keyboard | Screen::Network => {
                if self.list_focus {
                    FocusKind::List
                } else {
                    FocusKind::Field { last: true }
                }
            }
            Screen::Summary => FocusKind::Buttons,
            _ => FocusKind::Buttons,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    None,
    Back,
    Next,
    Quit,
    Reboot,
    Finish,
    KeepInstalling,
    CancelInstall,
    RetryInstall,
    LicenseAccept,
    Connect,
    Rescan,
    NetMenu,
    NetRetry,
}

impl Action {
    fn is_primary(self) -> bool {
        matches!(
            self,
            Action::Next
                | Action::LicenseAccept
                | Action::Reboot
                | Action::RetryInstall
                | Action::KeepInstalling
                | Action::Connect
                | Action::NetRetry
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quit {
    Normal,
}

pub struct PickItem {
    pub code: String,
    pub name: String,
    pub extra: String,
}

pub struct Picker {
    pub items: Vec<PickItem>,
    pub filtered: Vec<usize>,
    pub selected: Option<usize>,
    pub query: Input,
}

impl Picker {
    pub fn new(items: Vec<PickItem>) -> Self {
        let filtered = (0..items.len()).collect();
        Self {
            items,
            filtered,
            selected: None,
            query: Input::default(),
        }
    }

    pub fn refilter(&mut self) {
        let needle = fold(self.query.value());
        self.filtered = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                needle.is_empty()
                    || fold(&item.code).contains(&needle)
                    || fold(&item.name).contains(&needle)
                    || fold(&item.extra).contains(&needle)
            })
            .map(|(i, _)| i)
            .collect();
        if let Some(sel) = self.selected {
            if sel >= self.filtered.len() {
                self.selected = None;
            }
        }
        if self.selected.is_none() && !self.query.value().is_empty() && !self.filtered.is_empty() {
            self.selected = Some(0);
        }
    }

    pub fn up(&mut self) {
        if self.filtered.is_empty() {
            return;
        }
        self.selected = Some(match self.selected {
            None => 0,
            Some(0) => self.filtered.len() - 1,
            Some(s) => s - 1,
        });
    }

    pub fn down(&mut self) {
        if self.filtered.is_empty() {
            return;
        }
        self.selected = Some(match self.selected {
            None => 0,
            Some(s) if s + 1 >= self.filtered.len() => 0,
            Some(s) => s + 1,
        });
    }

    pub fn move_by(&mut self, delta: isize) {
        if self.filtered.is_empty() {
            return;
        }
        let current = self.selected.unwrap_or(0) as isize;
        let index = (current + delta).clamp(0, self.filtered.len() as isize - 1) as usize;
        self.selected = Some(index);
    }

    pub fn current(&self) -> Option<&PickItem> {
        let idx = self.filtered.get(self.selected?)?;
        self.items.get(*idx)
    }
}

pub enum RedFase {
    Lista,
    Contrasena {
        ssid: String,
        input: Input,
        visible: bool,
    },
    Conectando {
        ssid: String,
    },
    Error {
        ssid: String,
        msg: String,
    },
    Conectado {
        ssid: String,
    },
}

impl RedFase {
    pub fn ssid(&self) -> Option<&str> {
        match self {
            RedFase::Lista => None,
            RedFase::Contrasena { ssid, .. }
            | RedFase::Conectando { ssid }
            | RedFase::Error { ssid, .. }
            | RedFase::Conectado { ssid } => Some(ssid),
        }
    }

    pub fn is_connected(&self) -> bool {
        matches!(self, RedFase::Conectado { .. })
    }
}

pub struct NetState {
    pub backend: std::sync::Arc<dyn net::WifiBackend>,
    pub phase: RedFase,
    pub ethernet: bool,
    pub online: bool,
    pub wifi: Vec<net::Wifi>,
    pub selected: usize,
    pub connecting_since: Option<Instant>,
}

pub struct HostState {
    pub custom: bool,
    pub name: Input,
    pub focus: usize,
}

pub struct DiskState {
    pub disks: Vec<Disk>,
    pub index: usize,
    pub decision: DiskDecision,
    pub erase_all: bool,
    pub shrink_bytes: u64,
    pub details: bool,
    pub erase_modal: bool,
    pub erase_focus: usize,
    pub bar_focus: bool,
}

impl DiskState {
    pub fn refresh_decision(&mut self) {
        let decision = match self.disks.get(self.index) {
            Some(disk) => match disk.unusable_reason() {
                Some(reason) => DiskDecision::Blocked(reason.to_string()),
                None => disk::decide(disk),
            },
            None => DiskDecision::TooSmall {
                missing: disk::MIN_GIB * disk::GIB,
            },
        };
        if let DiskDecision::Shrink { min_free, .. } = &decision {
            self.shrink_bytes = *min_free;
        } else {
            self.shrink_bytes = 0;
        }
        self.decision = decision;
        self.erase_all = false;
        self.bar_focus = false;
    }

    pub fn effective(&self) -> DiskDecision {
        if self.erase_all {
            DiskDecision::WholeDisk
        } else {
            self.decision.clone()
        }
    }
}

pub const ENC_PASS: usize = 0;
pub const ENC_CONFIRM: usize = 1;
pub const ENC_TPM: usize = 2;
pub const ENC_NOTED: usize = 3;

pub struct EncState {
    pub pass: Input,
    pub confirm: Input,
    pub focus: usize,
    pub tpm: bool,
    pub tpm_present: bool,
    pub noted: bool,
    pub recovery: Zeroizing<String>,
    pub usb_open: bool,
    pub usb: Vec<crate::detect::RemovableMedium>,
    pub usb_index: usize,
    pub usb_saved: Option<String>,
}

impl EncState {
    pub fn new(tpm_present: bool) -> Self {
        Self {
            pass: Input::default(),
            confirm: Input::default(),
            focus: ENC_PASS,
            tpm: tpm_present,
            tpm_present,
            noted: false,
            recovery: crate::crypto::recovery_key(),
            usb_open: false,
            usb: Vec::new(),
            usb_index: 0,
            usb_saved: None,
        }
    }

    pub fn passphrase_matches(&self) -> bool {
        !self.pass.value().is_empty() && self.pass.value() == self.confirm.value()
    }

    pub fn targets(&self) -> Vec<usize> {
        let mut targets = vec![ENC_PASS, ENC_CONFIRM];
        if self.tpm_present {
            targets.push(ENC_TPM);
        }
        targets.push(ENC_NOTED);
        targets
    }

    pub fn ready(&self) -> bool {
        self.noted && self.passphrase_matches()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckState {
    Idle,
    Running,
    Ok,
    Failed(String),
}

pub struct InstallView {
    pub started: Instant,
    pub current: Option<Step>,
    pub done: Vec<Step>,
    pub failed: Option<Step>,
    pub log: VecDeque<String>,
    pub built: u64,
    pub total: u64,
    pub finished: bool,
    pub finished_at: Option<Instant>,
    pub error: Option<String>,
    pub log_path: Option<PathBuf>,
    pub last_error: Option<String>,
    step_started: Instant,
    pub durations: Vec<(Step, Duration)>,
    samples: VecDeque<(Instant, u64)>,
}

impl InstallView {
    fn new() -> Self {
        Self {
            started: Instant::now(),
            current: None,
            done: Vec::new(),
            failed: None,
            log: VecDeque::new(),
            built: 0,
            total: 0,
            finished: false,
            finished_at: None,
            error: None,
            log_path: None,
            last_error: None,
            step_started: Instant::now(),
            durations: Vec::new(),
            samples: VecDeque::new(),
        }
    }

    fn resume_from(&mut self, resume: Step) {
        if resume <= Step::Disk {
            *self = InstallView::new();
        } else {
            self.current = None;
            self.failed = None;
            self.error = None;
            self.finished = false;
            self.finished_at = None;
            self.built = 0;
            self.total = 0;
            self.step_started = Instant::now();
            self.samples.clear();
        }
    }

    pub fn push_log(&mut self, line: String) {
        self.log.push_back(line);
        while self.log.len() > 400 {
            self.log.pop_front();
        }
    }

    pub fn push_error(&mut self, line: String) {
        self.last_error = Some(line.clone());
        self.push_log(format!("\u{1}{line}"));
    }

    pub fn visible_log(&self, lines: usize) -> Vec<&String> {
        let skip = self.log.len().saturating_sub(lines);
        self.log.iter().skip(skip).collect()
    }

    pub fn record(&mut self, built: u64) {
        let now = Instant::now();
        self.samples.push_back((now, built));
        while let Some(&(t, _)) = self.samples.front() {
            if now.duration_since(t) > Duration::from_secs(30) {
                self.samples.pop_front();
            } else {
                break;
            }
        }
    }

    pub fn eta_secs(&self) -> Option<u64> {
        let &(first_t, first_b) = self.samples.front()?;
        let &(last_t, last_b) = self.samples.back()?;
        let dt = last_t.duration_since(first_t).as_secs_f64();
        if dt < 3.0 || last_b <= first_b || self.total <= self.built {
            return None;
        }
        let rate = (last_b - first_b) as f64 / dt;
        if rate <= 0.0 {
            return None;
        }
        Some(((self.total - self.built) as f64 / rate).ceil() as u64)
    }
}

pub struct App {
    pub palette: Palette,
    pub screen: Screen,

    pub dry_run: bool,
    pub repo_spec: String,
    pub repo_dir: PathBuf,
    pub source_ok: bool,
    pub network: bool,
    pub show_network: bool,
    pub source_check: CheckState,
    source_check_rx: Option<Receiver<Result<(), String>>>,

    pub settings: Settings,
    pub settings_ok: bool,

    pub secure_boot: bool,

    pub virt: Option<String>,
    pub model: String,
    model_rx: Option<Receiver<String>>,

    pub net: NetState,

    pub license_scroll: usize,
    pub license_max_scroll: usize,
    pub license_modal: bool,
    pub license_modal_focus: usize,
    pub license_line: usize,
    pub license_total: usize,
    pub license_text_area: Rect,

    pub timezones: Option<Picker>,
    pub loc: Option<Picker>,

    pub keyboards: Option<Picker>,
    pub keyboard_suggested: Option<usize>,
    pub key_test_focus: bool,
    pub key_test: Input,
    pub key_modal: Option<Picker>,
    pub keyboard_active: String,
    pub keyboard_layout: String,
    pub keyboard_variant: String,

    pub host: HostState,
    pub disks: DiskState,
    pub enc: EncState,

    pub install: InstallView,
    install_rx: Option<Receiver<InstallMsg>>,
    net_rx: Option<Receiver<(String, std::result::Result<bool, String>)>>,
    pub install_cancel: bool,
    pub install_log_view: bool,
    pub install_commands: bool,
    pub install_follow: bool,
    pub install_log_scroll: usize,
    pub install_max_scroll: usize,

    pub button_focus: usize,
    pub focus_buttons: bool,
    pub list_focus: bool,
    pub show_commands: bool,
    pub status: Option<String>,
    pub quit: Option<Quit>,
    pub should_quit: bool,
    pub force_clear: bool,
    pub tick: u64,
}

impl App {
    pub fn new(dry_run: bool, repo_spec: String, repo_dir: PathBuf) -> Self {
        let palette = if theme::console::is_linux_console() {
            Palette::console()
        } else {
            Palette::truecolor()
        };
        let (mut settings, settings_ok) = match Settings::read(&repo_dir) {
            Ok(settings) => (settings, true),
            Err(_) => (Settings::fallback(), false),
        };
        let secure_boot = detect::secure_boot();
        settings.host_name = String::new();
        settings.username = None;
        settings.userdescription = None;
        settings.git_name = None;
        settings.git_email = None;
        settings.time_zone = String::new();
        settings.default_locale = String::new();

        let kb_active = keyboard::detect_active();
        let (kb_layout, kb_variant) = keyboard::by_id(&kb_active)
            .map(|l| (l.xkb, l.variant))
            .unwrap_or_else(|| ("us".to_string(), String::new()));
        settings.keyboard_layout = kb_layout.clone();
        settings.keyboard_variant = kb_variant.clone();

        let mut disks = detect::disks();
        if dry_run {
            disks.extend(detect::test_disks());
        }
        let disk_index = disks
            .iter()
            .position(|d| d.unusable_reason().is_none())
            .unwrap_or(0);
        let (model_tx, model_rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = model_tx.send(detect::machine_model());
        });
        let show_network = dry_run || !net::connection_up();
        let mut app = Self {
            palette,
            screen: Screen::Splash,
            dry_run,
            repo_spec,
            repo_dir,
            source_ok: false,
            network: false,
            show_network,
            source_check: CheckState::Idle,
            source_check_rx: None,
            settings_ok,
            secure_boot,
            host: HostState {
                custom: false,
                name: Input::default(),
                focus: 0,
            },
            disks: DiskState {
                disks,
                index: disk_index,
                decision: DiskDecision::WholeDisk,
                erase_all: false,
                shrink_bytes: 0,
                details: false,
                erase_modal: false,
                erase_focus: 0,
                bar_focus: false,
            },
            enc: EncState::new(detect::tpm2_present() && secure_boot),
            net: NetState {
                backend: net::backend(dry_run),
                phase: RedFase::Lista,
                ethernet: net::ethernet_connected(),
                online: false,
                wifi: Vec::new(),
                selected: 0,
                connecting_since: None,
            },
            net_rx: None,
            license_scroll: 0,
            license_max_scroll: 0,
            license_modal: false,
            license_modal_focus: 1,
            license_line: 1,
            license_total: 1,
            license_text_area: Rect::default(),
            timezones: None,
            loc: None,
            keyboards: None,
            keyboard_suggested: None,
            key_test_focus: false,
            key_test: Input::default(),
            key_modal: None,
            keyboard_active: kb_active,
            keyboard_layout: kb_layout,
            keyboard_variant: kb_variant,
            install: InstallView::new(),
            install_rx: None,
            install_cancel: false,
            install_log_view: false,
            install_commands: false,
            install_follow: true,
            install_log_scroll: 0,
            install_max_scroll: 0,
            button_focus: 0,
            focus_buttons: true,
            list_focus: false,
            show_commands: false,
            status: None,
            quit: None,
            should_quit: false,
            force_clear: false,
            tick: 0,
            virt: detect::environment(),
            model: String::new(),
            model_rx: Some(model_rx),
            settings,
        };
        app.disks.refresh_decision();
        app
    }

    pub fn run(
        &mut self,
        terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    ) -> Result<()> {
        loop {
            if self.force_clear {
                terminal.clear()?;
                self.force_clear = false;
            }
            terminal.draw(|f| self.draw(f))?;

            if ratatui::crossterm::event::poll(Duration::from_millis(100))? {
                let event = ratatui::crossterm::event::read()?;
                if let ratatui::crossterm::event::Event::Resize(_, _) = event {
                    terminal.clear()?;
                }
                self.handle_event(event);
            }
            self.tick = self.tick.wrapping_add(1);
            self.poll_install();
            self.maybe_finish_install();
            self.poll_network();
            self.poll_source_check();
            self.poll_model();

            if self.should_quit {
                break;
            }
        }
        Ok(())
    }

    pub fn cycle_disk(&mut self, delta: isize) {
        if self.disks.disks.is_empty() {
            return;
        }
        let n = self.disks.disks.len() as isize;
        if n == 0 {
            return;
        }
        let mut idx = self.disks.index as isize;
        for _ in 0..n {
            idx = (idx + delta).rem_euclid(n);
            if self.disks.disks[idx as usize].unusable_reason().is_none() {
                self.disks.index = idx as usize;
                break;
            }
        }
        self.disks.refresh_decision();
    }

    fn disk_ready(&self) -> bool {
        let Some(disk) = self.selected_disk() else {
            return false;
        };
        disk.unusable_reason().is_none()
            && matches!(
                self.disks.effective(),
                DiskDecision::WholeDisk
                    | DiskDecision::FreeSpace { .. }
                    | DiskDecision::Shrink { .. }
            )
    }

    pub fn adjust_shrink(&mut self, delta_gb: i64) {
        let DiskDecision::Shrink {
            min_free, max_free, ..
        } = self.disks.decision
        else {
            return;
        };
        let step = 10 * disk::GIB;
        if delta_gb < 0 {
            self.disks.shrink_bytes = self
                .disks
                .shrink_bytes
                .saturating_sub(delta_gb.unsigned_abs() * step)
                .max(min_free);
        } else {
            self.disks.shrink_bytes =
                (self.disks.shrink_bytes + delta_gb as u64 * step).min(max_free);
        }
    }

    fn ensure_timezones(&mut self) {
        if self.timezones.is_some() {
            return;
        }
        let items = crate::screens::timezone::load_items();
        self.timezones = Some(Picker::new(items));
    }

    fn ensure_locales(&mut self) {
        if self.loc.is_some() {
            return;
        }
        let items = crate::screens::locale::load_items();
        self.loc = Some(Picker::new(items));
    }

    fn keyboard_items(&self) -> Vec<PickItem> {
        keyboard::all()
            .iter()
            .map(|l| {
                let name = if l.code == self.keyboard_active {
                    format!("{} · actual", l.name)
                } else {
                    l.name.clone()
                };
                PickItem {
                    code: l.code.clone(),
                    name,
                    extra: String::new(),
                }
            })
            .collect()
    }

    fn keyboard_index(&self) -> Option<usize> {
        keyboard::all()
            .iter()
            .position(|l| l.code == self.keyboard_active)
    }

    fn ensure_keyboards(&mut self) {
        if self.keyboards.is_some() {
            return;
        }
        let items = self.keyboard_items();
        self.keyboards = Some(Picker::new(items));
        self.keyboard_suggested = self.suggested_keyboard();
    }

    fn suggested_keyboard(&self) -> Option<usize> {
        let code = keyboard::suggest(&self.settings.default_locale)?;
        keyboard::all().iter().position(|l| l.code == code)
    }

    pub fn open_key_modal(&mut self) {
        if self.key_modal.is_some() {
            return;
        }
        let mut picker = Picker::new(self.keyboard_items());
        picker.selected = self.keyboard_index();
        self.key_modal = Some(picker);
    }

    pub fn apply_keyboard(&mut self, id: &str) {
        self.apply_keyboard_q(id, true);
    }

    pub fn apply_keyboard_q(&mut self, id: &str, announce: bool) {
        let Some(layout) = keyboard::by_id(id) else {
            return;
        };
        self.keyboard_active = id.to_string();
        self.keyboard_layout = layout.xkb.clone();
        self.keyboard_variant = layout.variant.clone();
        self.settings.keyboard_layout = layout.xkb.clone();
        self.settings.keyboard_variant = layout.variant.clone();
        self.status = None;
        if theme::console::is_linux_console() {
            match keyboard::apply_console(&layout) {
                keyboard::ConsoleOutcome::Applied => {}
                keyboard::ConsoleOutcome::Fallback => {
                    self.status = Some("sin equivalente en consola: se usa us".into())
                }
                keyboard::ConsoleOutcome::Failed => {
                    self.status = Some(format!("no se pudo aplicar el teclado {}", layout.xkb))
                }
            }
        } else if announce {
            self.status = Some("la distribución la controla tu escritorio".into());
        }
        let items = self.keyboard_items();
        if let Some(picker) = self.keyboards.as_mut() {
            picker.items = items;
        }
    }

    pub fn screen_title(&self) -> String {
        match self.screen {
            Screen::License => "Licencia · GNU GPL v2.0".to_string(),
            s => s.title().to_string(),
        }
    }

    pub fn final_host_name(&self) -> String {
        if self.host.custom && !self.host.name.value().is_empty() {
            self.host.name.value().to_string()
        } else {
            let model = if self.model.is_empty() {
                "PC"
            } else {
                self.model.as_str()
            };
            nixcore::user::host_name_from_pretty(model)
        }
    }

    pub fn host_name_error(&self) -> Option<&'static str> {
        if !self.host.custom || self.host.name.value().is_empty() {
            return None;
        }
        nixcore::user::host_name_error(self.host.name.value())
    }

    pub fn host_name_valid(&self) -> bool {
        self.host_name_error().is_none()
    }

    fn apply_current(&mut self) {
        match self.screen {
            Screen::Host => {
                self.settings.host_name = self.final_host_name();
                self.settings.pretty_host_name = self.model.clone();
            }
            Screen::Timezone => {
                if let Some(p) = &self.timezones {
                    if let Some(item) = p.current() {
                        self.settings.time_zone = item.code.clone();
                    }
                }
            }
            Screen::Locale => {
                if let Some(p) = &self.loc {
                    if let Some(item) = p.current() {
                        self.settings.default_locale = item.code.clone();
                    }
                }
            }
            _ => {}
        }
    }

    pub fn selected_disk(&self) -> Option<&Disk> {
        self.disks.disks.get(self.disks.index)
    }

    pub fn start_install(&mut self) {
        self.start_install_from(Step::Disk);
    }

    pub fn start_install_from(&mut self, resume: Step) {
        let Some(disk) = self.selected_disk().cloned() else {
            self.status = Some("no hay disco seleccionado".into());
            return;
        };
        let (tx, rx) = mpsc::channel();
        self.install_rx = Some(rx);
        self.install.resume_from(resume);

        let root = if self.dry_run {
            let dir = std::env::temp_dir().join("keel-mnt");
            let _ = std::fs::create_dir_all(&dir);
            dir
        } else {
            PathBuf::from("/mnt")
        };

        let wifi = match &self.net.phase {
            RedFase::Conectado { ssid } if ssid != "ethernet" => Some(install::WifiSave {
                ssid: ssid.clone(),
                networkmanager: self.net.backend.name() == "nmcli",
            }),
            _ => None,
        };

        let luks_passphrase = Zeroizing::new(self.enc.pass.value().to_string());
        let recovery_key = Zeroizing::new(self.enc.recovery.to_string());

        let disk_mode = match self.disks.effective() {
            DiskDecision::WholeDisk => install::DiskMode::Whole,
            DiskDecision::FreeSpace { reuse_efi } => install::DiskMode::Alongside {
                reuse_efi: if reuse_efi {
                    disk.efi().map(|e| e.path())
                } else {
                    None
                },
                shrink: None,
            },
            DiskDecision::Shrink {
                ntfs, reuse_efi, ..
            } => install::DiskMode::Alongside {
                reuse_efi: if reuse_efi {
                    disk.efi().map(|e| e.path())
                } else {
                    None
                },
                shrink: Some(install::ShrinkSpec {
                    ntfs,
                    free_bytes: self.disks.shrink_bytes,
                }),
            },
            _ => install::DiskMode::Whole,
        };

        let plan = Plan {
            source: self.repo_dir.clone(),
            root,
            settings: self.settings.clone(),
            luks_passphrase,
            recovery_key,
            luks_tpm: self.enc.tpm && self.enc.tpm_present,
            device: disk.path(),
            disk_mode,
            dry_run: self.dry_run,
            secure_boot: self.secure_boot,
            wifi,
            resume_from: resume,
        };
        install::spawn(plan, tx);
    }

    fn poll_install(&mut self) {
        let Some(rx) = self.install_rx.take() else {
            return;
        };
        let mut finished: Option<(bool, Option<String>)> = None;
        while let Ok(msg) = rx.try_recv() {
            match msg {
                InstallMsg::Step(step) => {
                    let now = Instant::now();
                    if let Some(prev) = self.install.current.replace(step) {
                        if !self.install.done.contains(&prev) {
                            self.install.done.push(prev);
                            self.install
                                .durations
                                .push((prev, now.duration_since(self.install.step_started)));
                        }
                    }
                    self.install.step_started = now;
                }
                InstallMsg::Log(line) => self.install.push_log(line),
                InstallMsg::Error(line) => self.install.push_error(line),
                InstallMsg::Repaint => self.force_clear = true,
                InstallMsg::Progress { built, total } => {
                    self.install.built = built;
                    self.install.total = total;
                    self.install.record(built);
                }
                InstallMsg::Finished {
                    ok,
                    error,
                    log_path,
                } => {
                    if let Some(step) = self.install.current.take() {
                        let dur = Instant::now().duration_since(self.install.step_started);
                        self.install.durations.push((step, dur));
                        if ok {
                            self.install.done.push(step);
                        } else {
                            self.install.failed = Some(step);
                        }
                    }
                    if ok {
                        self.install.built = self.install.total;
                    }
                    self.install.log_path = Some(log_path);
                    finished = Some((ok, error));
                }
            }
        }
        if let Some((ok, error)) = finished {
            self.install.finished = true;
            self.install.error = error;
            if ok {
                self.install.finished_at = Some(Instant::now());
            } else {
                self.focus_buttons = true;
                self.button_focus = 0;
                self.install_follow = true;
            }
        } else {
            self.install_rx = Some(rx);
        }
    }

    fn maybe_finish_install(&mut self) {
        if self.screen != Screen::Installing
            || !self.install.finished
            || self.install.error.is_some()
        {
            return;
        }
        if let Some(at) = self.install.finished_at {
            if at.elapsed() >= Duration::from_secs(1) {
                self.install_cancel = false;
                self.screen = Screen::Done;
                self.reset_focus();
            }
        }
    }

    pub fn handle_event(&mut self, event: Event) {
        let Event::Key(key) = event else { return };
        if key.kind != KeyEventKind::Press && key.kind != KeyEventKind::Repeat {
            return;
        }

        if self.quit.is_some() {
            self.handle_quit(key);
            return;
        }

        if self.key_modal.is_some() {
            self.key_modal_key(key);
            return;
        }

        if self.enc.usb_open {
            crate::screens::encryption::usb_key(self, key);
            return;
        }

        if self.license_modal {
            crate::screens::license::modal_key(self, key);
            return;
        }

        if key.code == KeyCode::F(2)
            && !matches!(
                self.screen,
                Screen::Splash | Screen::Installing | Screen::Done
            )
        {
            self.open_key_modal();
            return;
        }

        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            if self.screen == Screen::Installing {
                self.open_install_cancel();
            } else {
                self.quit = Some(Quit::Normal);
            }
            return;
        }

        if self.route_screen_key(key) {
            return;
        }

        match key.code {
            KeyCode::Char('v') => {
                self.show_commands = !self.show_commands;
            }
            KeyCode::Char('q') if self.screen != Screen::Installing => {
                self.quit = Some(Quit::Normal);
            }
            KeyCode::Tab => self.tab_focus(false),
            KeyCode::BackTab => self.tab_focus(true),
            KeyCode::Left | KeyCode::Right => {
                if self.focus_buttons {
                    self.move_button_key(key.code);
                } else {
                    self.focus_buttons_on_primary();
                }
            }
            KeyCode::Enter => {
                if self.focus_buttons {
                    self.activate_focused();
                } else {
                    self.enter_content();
                }
            }
            KeyCode::Esc if !self.clear_search() => self.go_back(),
            _ => {}
        }
    }

    fn enter_content(&mut self) {
        match self.focus_kind() {
            FocusKind::Field { last: true } => {
                if self.primary_enabled() {
                    self.advance();
                } else {
                    self.focus_first_incomplete();
                }
            }
            FocusKind::List => {
                if self.primary_enabled() {
                    self.advance();
                }
            }
            _ => self.focus_buttons_on_primary(),
        }
    }

    fn focus_first_incomplete(&mut self) {
        match self.screen {
            Screen::Host => {
                self.host.focus = if self.host_name_valid() { 0 } else { 1 };
            }
            Screen::Encryption => {
                self.enc.focus = if self.enc.pass.value().is_empty() {
                    ENC_PASS
                } else if !self.enc.passphrase_matches() {
                    ENC_CONFIRM
                } else {
                    ENC_NOTED
                };
            }
            _ => self.focus_buttons_on_primary(),
        }
    }

    fn clear_search(&mut self) -> bool {
        let picker = match self.screen {
            Screen::Locale => self.loc.as_mut(),
            Screen::Keyboard => self.keyboards.as_mut(),
            Screen::Timezone => self.timezones.as_mut(),
            _ => None,
        };
        if let Some(picker) = picker {
            if !picker.query.value().is_empty() {
                picker.query = Input::default();
                picker.refilter();
                self.list_focus = false;
                return true;
            }
        }
        false
    }

    fn open_install_cancel(&mut self) {
        if self.screen == Screen::Installing {
            self.install_cancel = true;
            self.install_follow = true;
            self.focus_buttons = true;
            self.button_focus = 0;
        }
    }

    fn route_screen_key(&mut self, key: KeyEvent) -> bool {
        match self.screen {
            Screen::Splash => false,
            Screen::Network => crate::screens::network::key(self, key),
            Screen::License => crate::screens::license::key(self, key),
            Screen::Timezone => crate::screens::timezone::key(self, key),
            Screen::Locale => crate::screens::locale::key(self, key),
            Screen::Keyboard => crate::screens::keyboard::key(self, key),
            Screen::Host => crate::screens::host::key(self, key),
            Screen::Disk => crate::screens::disk::key(self, key),
            Screen::Encryption => crate::screens::encryption::key(self, key),
            Screen::Summary => crate::screens::summary::key(self, key),
            Screen::Installing => crate::screens::installing::key(self, key),
            Screen::Done => crate::screens::done::key(self, key),
        }
    }

    fn primary_index(&self) -> usize {
        let buttons = self.buttons();
        buttons
            .iter()
            .position(|(b, action)| action.is_primary() && b.enabled)
            .or_else(|| buttons.iter().position(|(b, _)| b.enabled))
            .unwrap_or(0)
    }

    pub fn focus_buttons_on_primary(&mut self) {
        self.focus_buttons = true;
        self.button_focus = self.primary_index();
    }

    fn next_enabled(&self, from: usize, dir: isize) -> Option<usize> {
        let buttons = self.buttons();
        let n = buttons.len();
        if n == 0 {
            return None;
        }
        let mut idx = from as isize;
        for _ in 0..n {
            idx = (idx + dir).rem_euclid(n as isize);
            let i = idx as usize;
            if buttons[i].0.enabled && i != from {
                return Some(i);
            }
        }
        None
    }

    fn tab_focus(&mut self, back: bool) {
        let buttons = self.buttons();
        let n = buttons.len();
        if n == 0 {
            self.focus_buttons = false;
            return;
        }
        if !self.focus_buttons {
            self.focus_buttons_on_primary();
            return;
        }
        let dir = if back { -1 } else { 1 };
        let mut idx = self.button_focus as isize;
        for _ in 0..n {
            idx += dir;
            if idx < 0 || idx >= n as isize {
                self.focus_buttons = false;
                return;
            }
            if buttons[idx as usize].0.enabled {
                self.button_focus = idx as usize;
                return;
            }
        }
        self.focus_buttons = false;
    }

    fn move_button_key(&mut self, code: KeyCode) {
        let dir = if code == KeyCode::Left { -1 } else { 1 };
        if let Some(i) = self.next_enabled(self.button_focus, dir) {
            self.button_focus = i;
        }
    }

    pub fn buttons(&self) -> Vec<(Btn, Action)> {
        match self.screen {
            Screen::Splash => vec![(Btn::new("Continuar"), Action::Next)],
            Screen::Network => crate::screens::network::buttons(self),
            Screen::License => crate::screens::license::buttons(self),
            Screen::Timezone => crate::screens::timezone::buttons(self),
            Screen::Locale => crate::screens::locale::buttons(self),
            Screen::Keyboard => crate::screens::keyboard::buttons(self),
            Screen::Host => crate::screens::host::buttons(self),
            Screen::Disk => crate::screens::disk::buttons(self),
            Screen::Encryption => crate::screens::encryption::buttons(self),
            Screen::Summary => crate::screens::summary::buttons(self),
            Screen::Installing => {
                if self.install_cancel {
                    vec![
                        (Btn::new("Seguir"), Action::KeepInstalling),
                        (Btn::new("Cancelar"), Action::CancelInstall),
                    ]
                } else if self.install.error.is_some() {
                    vec![
                        (Btn::new("Reintentar"), Action::RetryInstall),
                        (Btn::new("Salir"), Action::Finish),
                    ]
                } else {
                    Vec::new()
                }
            }
            Screen::Done => crate::screens::done::buttons(self),
        }
    }

    fn activate_focused(&mut self) {
        let buttons = self.buttons();
        if let Some((button, action)) = buttons.get(self.button_focus) {
            if button.enabled {
                let action = *action;
                self.activate(action);
            }
        }
    }

    pub fn choose_keyboard(&mut self, index: usize) {
        if let Some(picker) = self.keyboards.as_mut() {
            picker.selected = Some(index);
        }
        let code = self
            .keyboards
            .as_ref()
            .and_then(|p| p.current())
            .map(|item| item.code.clone());
        if let Some(code) = code {
            self.apply_keyboard_q(&code, false);
        }
    }

    fn activate(&mut self, action: Action) {
        match action {
            Action::Back => self.go_back(),
            Action::Next => self.advance(),
            Action::Quit => self.quit = Some(Quit::Normal),
            Action::Reboot => crate::screens::done::restart(self),
            Action::Finish => crate::screens::done::salir(self),
            Action::KeepInstalling => {
                self.install_cancel = false;
                self.focus_buttons = false;
            }
            Action::CancelInstall => {
                self.install_cancel = false;
                self.should_quit = true;
            }
            Action::RetryInstall => self.restart_install(),
            Action::LicenseAccept => {
                self.license_modal = true;
                self.license_modal_focus = 1;
            }
            Action::Connect => crate::screens::network::connect(self),
            Action::Rescan => crate::screens::network::rescan(self),
            Action::NetMenu => crate::screens::network::to_menu(self),
            Action::NetRetry => crate::screens::network::retry(self),
            Action::None => {}
        }
    }

    fn handle_quit(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('s') | KeyCode::Enter => self.should_quit = true,
            KeyCode::Char('n') | KeyCode::Esc => self.quit = None,
            _ => {}
        }
    }

    fn key_modal_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => self.key_modal = None,
            KeyCode::Enter => {
                let id = self
                    .key_modal
                    .as_ref()
                    .and_then(|p| p.current())
                    .map(|i| i.code.clone());
                self.key_modal = None;
                if let Some(id) = id {
                    self.apply_keyboard(&id);
                }
            }
            _ => {
                if let Some(picker) = self.key_modal.as_mut() {
                    crate::screens::picker::key(picker, key);
                }
            }
        }
    }

    pub fn focus_primary(&mut self) {
        self.button_focus = self.primary_index();
    }

    pub fn no_focus(&self) -> bool {
        self.license_modal
            || self.quit.is_some()
            || self.install_cancel
            || self.disks.erase_modal
            || self.enc.usb_open
            || self.key_modal.is_some()
    }

    pub fn blink(&self) -> bool {
        (self.tick / 5).is_multiple_of(2)
    }

    pub fn draw_buttons(&self, f: &mut Frame, area: Rect) {
        let enabled = self.primary_enabled();
        let buttons: Vec<Btn> = self
            .buttons()
            .into_iter()
            .map(|(mut b, action)| {
                if action == Action::Next {
                    b.enabled = enabled;
                }
                b
            })
            .collect();
        if buttons.is_empty() {
            return;
        }
        let row = ButtonRow {
            buttons: &buttons,
            focus: self.button_focus,
            focused: self.focus_buttons && !self.no_focus(),
            palette: &self.palette,
        };
        f.render_widget(row, area);
    }

    pub fn reset_focus(&mut self) {
        self.button_focus = self.primary_index();
        self.list_focus = matches!(
            self.screen,
            Screen::Locale | Screen::Keyboard | Screen::Timezone | Screen::Network
        );
        self.focus_buttons = self.primary_enabled() && !self.list_focus;
        if self.screen == Screen::Host {
            self.host.focus = 0;
        }
        if self.screen == Screen::Keyboard {
            self.key_test_focus = false;
            self.keyboard_suggested = self.suggested_keyboard();
        }
        if self.screen == Screen::License {
            self.license_modal = false;
            self.license_modal_focus = 1;
        }
        if self.screen == Screen::Disk {
            self.disks.bar_focus = false;
            self.disks.erase_modal = false;
            self.disks.refresh_decision();
        }
        if self.screen == Screen::Encryption {
            self.enc.focus = ENC_PASS;
            self.enc.usb_open = false;
        }
    }

    pub fn open_usb_modal(&mut self) {
        let target = self.selected_disk().map(|d| d.path()).unwrap_or_default();
        self.enc.usb = detect::removable_media(&target);
        self.enc.usb_index = 0;
        self.enc.usb_open = true;
    }

    pub fn go_back(&mut self) {
        if let Some(prev) = self.screen.previous(self.show_network) {
            self.screen = prev;
            self.reset_focus();
            self.status = None;
        }
    }

    pub fn advance(&mut self) {
        self.apply_current();
        if !self.primary_enabled() {
            return;
        }
        if self.screen == Screen::License {
            self.spawn_source_check();
        }
        match self.screen.next(self.show_network) {
            Some(Screen::Installing) => {
                self.screen = Screen::Installing;
                self.reset_install_view();
                self.start_install();
            }
            Some(next) => {
                self.screen = next;
                self.reset_focus();
                self.status = None;
                match next {
                    Screen::Timezone => self.ensure_timezones(),
                    Screen::Locale => self.ensure_locales(),
                    Screen::Keyboard => self.ensure_keyboards(),
                    Screen::Network => self.refresh_network(),
                    _ => {}
                }
            }
            None => {}
        }
    }

    fn reset_install_view(&mut self) {
        self.focus_buttons = false;
        self.button_focus = 0;
        self.install_cancel = false;
        self.install_log_view = false;
        self.install_commands = false;
        self.install_follow = true;
        self.install_log_scroll = 0;
        self.install_max_scroll = 0;
    }

    pub fn restart_install(&mut self) {
        self.reset_install_view();
        let resume = self.install.failed.unwrap_or(Step::Disk);
        self.start_install_from(resume);
    }

    pub fn refresh_network(&mut self) {
        self.net.ethernet = net::ethernet_connected();
        self.net.online = self.net.backend.status();
        if self.net.ethernet && self.net.online {
            self.net.phase = RedFase::Conectado {
                ssid: "ethernet".to_string(),
            };
            return;
        }
        self.net.wifi = self.net.backend.scan();
        if self.net.wifi.is_empty() && self.dry_run {
            self.net.wifi = net::sample_networks();
        }
        self.net.selected = 0;
        self.net.phase = RedFase::Lista;
    }

    pub fn start_net_connect(&mut self, ssid: String, pass: String) {
        let backend = self.net.backend.clone();
        let (tx, rx) = mpsc::channel();
        self.net_rx = Some(rx);
        self.net.connecting_since = Some(Instant::now());
        self.net.phase = RedFase::Conectando { ssid: ssid.clone() };
        std::thread::spawn(move || {
            let result = backend
                .connect(&ssid, &pass)
                .map(|()| backend.status())
                .map_err(|err| err.to_string());
            let _ = tx.send((ssid, result));
        });
    }

    pub(crate) fn poll_network(&mut self) {
        let Some(rx) = self.net_rx.take() else {
            return;
        };
        let mut reply = None;
        while let Ok(msg) = rx.try_recv() {
            reply = Some(msg);
        }
        if let Some((ssid, result)) = reply {
            self.net.connecting_since = None;
            self.net.phase = match result {
                Ok(true) => {
                    self.net.online = true;
                    RedFase::Conectado { ssid }
                }
                Ok(false) => RedFase::Error {
                    ssid,
                    msg: "sin salida a cache.nixos.org".into(),
                },
                Err(msg) => RedFase::Error { ssid, msg },
            };
            if self.net.phase.is_connected() {
                self.advance();
            } else {
                self.focus_primary();
            }
        } else if let Some(since) = self.net.connecting_since {
            if since.elapsed() > Duration::from_secs(30) {
                let ssid = self.net.phase.ssid().unwrap_or_default().to_string();
                self.net.connecting_since = None;
                self.net.phase = RedFase::Error {
                    ssid,
                    msg: "sin respuesta".into(),
                };
            } else {
                self.net_rx = Some(rx);
            }
        } else {
            self.net_rx = Some(rx);
        }
    }

    pub fn faltante(&self) -> Option<String> {
        match self.screen {
            Screen::Splash | Screen::License | Screen::Installing | Screen::Done => None,
            Screen::Network => match &self.net.phase {
                RedFase::Contrasena { input, .. } if input.value().is_empty() => {
                    Some("falta: escribir la contraseña".into())
                }
                _ => None,
            },
            Screen::Locale => self
                .loc
                .as_ref()
                .filter(|p| p.selected.is_none())
                .map(|_| "falta: elegir un lenguaje".to_string()),
            Screen::Keyboard => self
                .keyboards
                .as_ref()
                .filter(|p| p.selected.is_none())
                .map(|_| "falta: elegir una distribución".to_string()),
            Screen::Timezone => self
                .timezones
                .as_ref()
                .filter(|p| p.selected.is_none())
                .map(|_| "falta: elegir una zona horaria".to_string()),
            Screen::Host => {
                if self.host_name_valid() {
                    None
                } else {
                    Some("nombre de red no válido".into())
                }
            }
            Screen::Disk => {
                if !crate::screens::disk::has_usable_disk(self) {
                    Some("falta: un disco donde instalar".into())
                } else if !self.disk_ready() {
                    Some("falta: elegir un disco".into())
                } else {
                    None
                }
            }
            Screen::Encryption => {
                if !self.enc.passphrase_matches() {
                    Some("falta: confirmar la frase de desbloqueo".into())
                } else if !self.enc.noted {
                    Some("falta: marcar que anotaste la clave de recuperación".into())
                } else {
                    None
                }
            }
            Screen::Summary => match &self.source_check {
                CheckState::Failed(_) => Some("la configuración no evalúa".into()),
                _ => None,
            },
        }
    }

    pub fn primary_enabled(&self) -> bool {
        match self.screen {
            Screen::Network => self.net.phase.is_connected(),
            Screen::License => true,
            Screen::Timezone => self
                .timezones
                .as_ref()
                .map(|p| p.selected.is_some())
                .unwrap_or(false),
            Screen::Locale => self
                .loc
                .as_ref()
                .map(|p| p.selected.is_some())
                .unwrap_or(false),
            Screen::Keyboard => self
                .keyboards
                .as_ref()
                .map(|p| p.selected.is_some())
                .unwrap_or(false),
            Screen::Host => self.host_name_valid(),
            Screen::Summary => !matches!(self.source_check, CheckState::Failed(_)),
            Screen::Disk => self.disk_ready(),
            Screen::Encryption => self.enc.ready(),
            _ => true,
        }
    }

    pub fn spawn_source_check(&mut self) {
        if self.source_check != CheckState::Idle {
            return;
        }
        if self.dry_run {
            self.source_check = CheckState::Running;
            return;
        }
        if !self.source_ok {
            self.source_check = CheckState::Failed("no hay repositorio para evaluar".into());
            return;
        }
        let repo = self.repo_dir.clone();
        let (tx, rx) = mpsc::channel();
        self.source_check_rx = Some(rx);
        self.source_check = CheckState::Running;
        std::thread::spawn(move || {
            let result = crate::install::check_source(&repo);
            let _ = tx.send(result);
        });
    }

    fn poll_source_check(&mut self) {
        if self.dry_run {
            if self.source_check == CheckState::Running {
                self.source_check = CheckState::Ok;
            }
            return;
        }
        let Some(rx) = self.source_check_rx.take() else {
            return;
        };
        let mut reply = None;
        while let Ok(result) = rx.try_recv() {
            reply = Some(result);
        }
        match reply {
            Some(Ok(())) => self.source_check = CheckState::Ok,
            Some(Err(err)) => self.source_check = CheckState::Failed(err),
            None => self.source_check_rx = Some(rx),
        }
    }

    fn poll_model(&mut self) {
        let Some(rx) = self.model_rx.take() else {
            return;
        };
        match rx.try_recv() {
            Ok(model) => self.model = model,
            Err(std::sync::mpsc::TryRecvError::Empty) => self.model_rx = Some(rx),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {}
        }
    }

    pub fn draw(&mut self, f: &mut Frame) {
        let area = f.area();
        f.render_widget(
            Block::default().style(Style::default().bg(self.palette.reset)),
            area,
        );

        if area.width < 64 || area.height < 20 {
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    "Agranda la ventana (mín. 64×20)",
                    Style::default()
                        .fg(self.palette.warning)
                        .add_modifier(Modifier::BOLD),
                )))
                .alignment(Alignment::Center),
                Rect::new(area.x, area.y + area.height / 2, area.width, 1),
            );
            return;
        }

        if self.screen == Screen::Splash {
            self.draw_splash(f, area);
            if self.quit.is_some() {
                self.draw_quit(f, area);
            }
            return;
        }

        if self.screen == Screen::Installing {
            self.draw_installing(f, area);
            if self.install_cancel {
                self.draw_install_cancel(f, area);
            }
            return;
        }
        if self.screen == Screen::Done {
            crate::screens::done::draw(f, self, area);
            return;
        }

        if self.screen == Screen::License {
            crate::screens::license::draw(f, self, area);
            if self.quit.is_some() {
                self.draw_quit(f, area);
            }
            return;
        }

        if self.screen == Screen::Disk {
            let layout = widgets::StepLayout::compute(area, 0, 0);
            self.draw_progress(f, layout.progress);
            let content = Rect::new(
                area.x,
                layout.title.y,
                area.width,
                layout.buttons.y.saturating_sub(layout.title.y),
            );
            crate::screens::disk::draw(f, self, content);
            self.draw_buttons(f, layout.buttons);
            if layout.shortcuts.height > 0 {
                self.draw_shortcuts(f, layout.shortcuts);
            }
            if self.quit.is_some() {
                self.draw_quit(f, layout.body);
            }
            return;
        }
        let commands_lines = self.command_list();
        let commands_h = if self.show_commands {
            (commands_lines.len() as u16 + 2).clamp(2, area.height / 2)
        } else {
            0
        };
        let layout = widgets::StepLayout::compute(area, self.note_height(), commands_h);

        self.draw_progress(f, layout.progress);
        self.draw_heading(f, layout.title);
        self.draw_body(f, layout.body);
        match self.screen {
            Screen::Splash | Screen::License | Screen::Installing | Screen::Done | Screen::Disk => {
            }
            Screen::Network => crate::screens::network::draw(f, self, layout.content),
            Screen::Timezone => crate::screens::timezone::draw(f, self, layout.content),
            Screen::Locale => crate::screens::locale::draw(f, self, layout.content),
            Screen::Keyboard => crate::screens::keyboard::draw(f, self, layout.content),
            Screen::Host => crate::screens::host::draw(f, self, layout.content),
            Screen::Encryption => crate::screens::encryption::draw(f, self, layout.content),
            Screen::Summary => crate::screens::summary::draw(f, self, layout.content),
        }
        self.draw_note(f, layout.note);
        self.draw_buttons(f, layout.buttons);
        self.draw_faltante(f, layout.faltante);
        if layout.shortcuts.height > 0 {
            self.draw_shortcuts(f, layout.shortcuts);
        }
        if self.show_commands {
            self.draw_commands(f, layout.commands, &commands_lines, commands_h);
        }

        if self.enc.usb_open {
            crate::screens::encryption::draw_usb(f, self, area);
        }
        if self.key_modal.is_some() {
            self.draw_key_modal(f, area);
        }
        if self.quit.is_some() {
            self.draw_quit(f, layout.body);
        }
    }

    fn draw_heading(&self, f: &mut Frame, area: Rect) {
        let mut text = self.screen_title();
        if self.dry_run {
            text.push_str(" · simulación");
        }
        f.render_widget(
            Paragraph::new(Span::styled(
                text,
                Style::default()
                    .fg(self.palette.white)
                    .add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Center),
            area,
        );
    }

    fn note_height(&self) -> u16 {
        match self.screen {
            Screen::Locale => 1,
            Screen::Summary => 2,
            _ => 0,
        }
    }

    fn draw_note(&self, f: &mut Frame, area: Rect) {
        if area.height == 0 {
            return;
        }
        match self.screen {
            Screen::Locale => {
                let code = self
                    .loc
                    .as_ref()
                    .and_then(|p| p.current())
                    .map(|i| i.code.clone())
                    .unwrap_or_default();
                f.render_widget(
                    Paragraph::new(Span::styled(code, Style::default().fg(self.palette.muted)))
                        .alignment(Alignment::Center),
                    area,
                );
            }
            Screen::Keyboard => {}
            Screen::Summary => {
                f.render_widget(
                    Paragraph::new(Span::styled(
                        "Al instalar se aplican los cambios al disco. No se puede deshacer.",
                        Style::default().fg(self.palette.warning),
                    ))
                    .alignment(Alignment::Center)
                    .wrap(ratatui::widgets::Wrap { trim: true }),
                    area,
                );
            }
            _ => {}
        }
    }

    fn draw_key_modal(&self, f: &mut Frame, area: Rect) {
        widgets::dim_buffer(f, &self.palette);
        let w = 54.min(area.width);
        let h = 13.min(area.height);
        let x = area.x + area.width.saturating_sub(w) / 2;
        let y = area.y + area.height.saturating_sub(h) / 2;
        let rect = Rect::new(x, y, w, h);
        f.render_widget(Clear, rect);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(self.palette.primary))
            .title(Span::styled(
                " Teclado ",
                Style::default().fg(self.palette.primary),
            ));
        let inner = block.inner(rect);
        f.render_widget(block, rect);

        if let Some(picker) = &self.key_modal {
            let content = Rect::new(
                inner.x,
                inner.y,
                inner.width,
                inner.height.saturating_sub(1),
            );
            crate::screens::picker::draw(f, &self.palette, content, picker, true);
        }
        f.render_widget(
            Paragraph::new(Span::styled(
                "enter aplicar · esc cerrar",
                Style::default().fg(self.palette.muted),
            ))
            .alignment(Alignment::Center),
            Rect::new(inner.x, inner.bottom().saturating_sub(1), inner.width, 1),
        );
    }

    fn draw_splash(&mut self, f: &mut Frame, area: Rect) {
        let console = theme::console::is_linux_console();
        let show_logo = area.height >= 24 && area.width >= 40;

        let logo: Vec<Line> = if show_logo {
            crate::logo::nixos_logo(console)
        } else {
            Vec::new()
        };
        let logo_w = logo
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.chars().count())
                    .sum::<usize>()
            })
            .max()
            .unwrap_or(0) as u16;
        let logo_h = logo.len() as u16;

        let gap_after_logo: u16 = if show_logo { 3 } else { 1 };
        let total_h = logo_h + gap_after_logo + 2 + 1 + 1 + 1 + 1;
        let mut y = area.y + area.height.saturating_sub(total_h) / 2;

        if show_logo && logo_h > 0 {
            let x = area.x + area.width.saturating_sub(logo_w) / 2;
            f.render_widget(Paragraph::new(logo), Rect::new(x, y, logo_w, logo_h));
            y += logo_h;
        }
        y += gap_after_logo;

        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("Welcome ", Style::default().fg(self.palette.highlight)),
                Span::styled("~/*", Style::default().fg(self.palette.logo_light)),
                Span::styled(".", Style::default().fg(self.palette.highlight)),
            ]))
            .alignment(Alignment::Center),
            Rect::new(area.x, y, area.width, 1),
        );
        y += 1;
        f.render_widget(
            Paragraph::new(Span::styled(
                "Watch your step, it vanishes on boot.",
                Style::default()
                    .fg(self.palette.muted)
                    .add_modifier(Modifier::ITALIC),
            ))
            .alignment(Alignment::Center),
            Rect::new(area.x, y, area.width, 1),
        );
        y += 2;

        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("Presiona ", Style::default().fg(self.palette.text)),
                Span::styled(
                    "Continuar",
                    Style::default()
                        .fg(self.palette.text)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " para iniciar la configuración del sistema",
                    Style::default().fg(self.palette.text),
                ),
            ]))
            .alignment(Alignment::Center),
            Rect::new(area.x, y, area.width, 1),
        );
        y += 2;

        self.draw_buttons(f, Rect::new(area.x, y, area.width, 1));
    }

    fn draw_installing(&mut self, f: &mut Frame, area: Rect) {
        f.render_widget(Clear, area);
        let width = (area.width * 80 / 100).clamp(40, 100).min(area.width);
        let x = area.x + area.width.saturating_sub(width) / 2;

        let buttons: Vec<Btn> = if self.install_cancel {
            Vec::new()
        } else {
            self.buttons().into_iter().map(|(b, _)| b).collect()
        };
        let has_buttons = !buttons.is_empty();
        let has_error = self.install.error.is_some();

        let mut below = 2u16;
        if has_error {
            below += 2;
        }
        if has_buttons {
            below += 2;
        }
        let max_box_h = area.height.saturating_sub(4 + below).max(4);

        let inner_w = width.saturating_sub(8) as usize;
        let rows = self.install_rows(inner_w);
        let box_h = (rows.len() as u16 + 4).min(max_box_h);
        let visible = box_h.saturating_sub(4) as usize;
        self.install_max_scroll = rows.len().saturating_sub(visible);
        if self.install_follow {
            self.install_log_scroll = self.install_max_scroll;
        }
        let offset = self.install_log_scroll.min(self.install_max_scroll);

        let content_h = 4 + box_h + below;
        let top = area.y + area.height.saturating_sub(content_h) / 2;

        let pct = crate::screens::installing::overall_percent(self);
        let eta = self
            .install
            .eta_secs()
            .map(|s| format!(" · ~{} min", s.div_ceil(60)))
            .unwrap_or_default();
        let left = format!("Instalando en {}", self.settings.host_name);
        let right = format!("{pct}%{eta}");
        let right_w = right.chars().count();
        let title = truncate_ellipsis(&left, (width as usize).saturating_sub(right_w + 2));
        let pad = (width as usize).saturating_sub(title.chars().count() + right_w + 1);
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    format!(" {title}"),
                    Style::default()
                        .fg(self.palette.white)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" ".repeat(pad)),
                Span::styled(
                    format!("{pct}%"),
                    Style::default()
                        .fg(self.palette.white)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(eta.clone(), Style::default().fg(self.palette.muted)),
                Span::raw(" "),
            ])),
            Rect::new(x, top, width, 1),
        );

        let bar_y = top + 2;
        let filled = (pct as usize * width as usize / 100).min(width as usize);
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    "━".repeat(filled),
                    Style::default().fg(self.palette.primary),
                ),
                Span::styled(
                    "─".repeat(width as usize - filled),
                    Style::default().fg(self.palette.row_bg),
                ),
            ])),
            Rect::new(x, bar_y, width, 1),
        );

        let box_y = bar_y + 2;
        let box_rect = Rect::new(x, box_y, width, box_h);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(self.palette.muted));
        let inner = block.inner(box_rect);
        f.render_widget(block, box_rect);

        let content = Rect::new(
            inner.x + 3,
            inner.y + 1,
            inner.width.saturating_sub(6),
            inner.height.saturating_sub(2),
        );
        let shown: Vec<Line> = rows.into_iter().skip(offset).take(visible).collect();
        f.render_widget(Paragraph::new(shown), content);
        widgets::draw_scrollbar(
            f,
            &self.palette,
            content,
            self.install_max_scroll + visible,
            offset as u16,
            visible as u16,
        );

        let mut y = box_y + box_h;
        if has_error {
            y += 1;
            f.render_widget(
                Paragraph::new(Span::styled(
                    "registro completo: /tmp/keel-install.log",
                    Style::default().fg(self.palette.muted),
                ))
                .alignment(Alignment::Center),
                Rect::new(area.x, y, area.width, 1),
            );
            y += 1;
        }
        if has_buttons {
            y += 1;
            self.draw_buttons(f, Rect::new(area.x, y, area.width, 1));
            y += 1;
        }
        y += 1;
        let hint = if self.install_log_view {
            "v ocultar registro · c comandos · ↑/↓ desplazar · ctrl+c cancelar"
        } else {
            "v ver registro · ctrl+c cancelar"
        };
        f.render_widget(
            Paragraph::new(Span::styled(hint, Style::default().fg(self.palette.muted)))
                .alignment(Alignment::Center),
            Rect::new(area.x, y, area.width, 1),
        );
    }

    fn install_rows(&self, width: usize) -> Vec<Line<'static>> {
        let current = self.install.current;
        let spinner = ["|", "/", "-", "\\"][(self.tick as usize) % 4];
        let sub_steps = if current.is_some() && self.install_log_view {
            self.sub_steps()
        } else {
            Vec::new()
        };
        let mut rows: Vec<Line> = Vec::new();
        for (i, step) in Step::ALL.iter().enumerate() {
            if i > 0 {
                rows.push(Line::from(""));
            }
            let step = *step;
            let (mark, color) = if self.install.failed == Some(step) {
                ("✗", self.palette.error)
            } else if self.install.done.contains(&step) {
                ("✓", self.palette.success)
            } else if current == Some(step) {
                (spinner, self.palette.primary)
            } else {
                ("○", self.palette.muted)
            };
            let name = capitalize(step.label());
            let summary = self.step_summary(step);
            let dur = self
                .install
                .durations
                .iter()
                .find(|(s, _)| *s == step)
                .map(|(_, d)| human_duration(*d))
                .unwrap_or_default();
            let dur_w = dur.chars().count();
            let avail = width.saturating_sub(3 + 12 + dur_w + 1);
            let summary = truncate_ellipsis(&summary, avail);
            let pad = avail.saturating_sub(summary.chars().count());
            rows.push(Line::from(vec![
                Span::styled(format!("{mark:<3}"), Style::default().fg(color)),
                Span::styled(
                    format!("{name:<12}"),
                    Style::default().fg(self.palette.text),
                ),
                Span::styled(summary, Style::default().fg(self.palette.muted)),
                Span::raw(" ".repeat(pad + 1)),
                Span::styled(dur, Style::default().fg(self.palette.muted)),
            ]));

            if current == Some(step) {
                for (j, line) in sub_steps.iter().enumerate() {
                    let last = j + 1 == sub_steps.len();
                    let color = if last {
                        self.palette.highlight
                    } else {
                        self.palette.muted
                    };
                    let text = truncate_ellipsis(line, width.saturating_sub(4));
                    rows.push(Line::from(vec![
                        Span::styled("   │ ", Style::default().fg(self.palette.rule)),
                        Span::styled(text, Style::default().fg(color)),
                    ]));
                }
            }

            if self.install.failed == Some(step) {
                for line in self.error_lines() {
                    for chunk in hard_wrap(&line, width.saturating_sub(4).max(1)) {
                        rows.push(Line::from(vec![
                            Span::styled("   │ ", Style::default().fg(self.palette.rule)),
                            Span::styled(chunk, Style::default().fg(self.palette.error)),
                        ]));
                    }
                }
            }
        }

        if self.install_commands {
            let cmds = self.command_lines();
            if !cmds.is_empty() {
                rows.push(Line::from(Span::styled(
                    "─".repeat(width.max(1)),
                    Style::default().fg(self.palette.rule),
                )));
                for cmd in cmds {
                    let text = truncate_ellipsis(&cmd, width.saturating_sub(2));
                    rows.push(Line::from(vec![
                        Span::styled("│ ", Style::default().fg(self.palette.rule)),
                        Span::styled(text, Style::default().fg(self.palette.primary)),
                    ]));
                }
            }
        }

        rows
    }

    fn error_lines(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        if let Some(err) = &self.install.error {
            for line in err.lines() {
                if !line.trim().is_empty() {
                    out.push(line.to_string());
                }
            }
        }
        if let Some(last) = &self.install.last_error {
            for line in last.lines() {
                if !line.trim().is_empty() && !out.iter().any(|e| e == line) {
                    out.push(line.to_string());
                }
            }
        }
        out
    }

    fn command_lines(&self) -> Vec<String> {
        let mut cmds: Vec<String> = self
            .install
            .log
            .iter()
            .filter(|l| is_command(l))
            .map(|l| self.shorten_command(l))
            .collect();
        let n = cmds.len();
        cmds.split_off(n.saturating_sub(4))
    }

    fn shorten_command(&self, line: &str) -> String {
        let root = std::env::temp_dir().join("keel-mnt");
        let mut s = line.replace(&root.to_string_lossy().to_string(), "/mnt");
        for secret in [
            self.enc.pass.value().to_string(),
            self.enc.recovery.to_string(),
        ] {
            if !secret.is_empty() {
                s = s.replace(&secret, "••••");
            }
        }
        s
    }

    fn step_summary(&self, step: Step) -> String {
        if self.install.current == Some(step) {
            if let Some(activity) = self.last_activity() {
                if step == Step::System && self.install.total > 0 {
                    return format!(
                        "{activity} · {} de {}",
                        self.install.built, self.install.total
                    );
                }
                return activity;
            }
        }
        match step {
            Step::Disk => {
                let action = match self.disks.effective() {
                    DiskDecision::WholeDisk => "todo el disco",
                    DiskDecision::FreeSpace { .. } => "espacio libre",
                    DiskDecision::Shrink { .. } => "junto a Windows",
                    _ => "sin decidir",
                };
                format!("{action} · EFI + System HD (btrfs, cifrado)")
            }
            Step::Settings => "machine.toml".to_string(),
            Step::System => {
                if self.install.total > 0 {
                    format!(
                        "{} paquetes · {} de {}",
                        self.install.total, self.install.built, self.install.total
                    )
                } else {
                    "compilando el sistema".to_string()
                }
            }
            Step::Boot => {
                if self.secure_boot {
                    "lanzaboote · claves de Secure Boot".to_string()
                } else {
                    "systemd-boot".to_string()
                }
            }
        }
    }

    fn last_activity(&self) -> Option<String> {
        self.install.log.iter().rev().find_map(|line| {
            if line.starts_with('\u{1}') {
                return None;
            }
            let line = line.trim();
            if line.is_empty() || is_command(line) || line.starts_with('(') || line.starts_with('=')
            {
                return None;
            }
            Some(line.to_string())
        })
    }

    fn sub_steps(&self) -> Vec<String> {
        self.install
            .log
            .iter()
            .rev()
            .filter(|line| {
                if line.starts_with('\u{1}') {
                    return false;
                }
                let line = line.trim();
                !line.is_empty()
                    && !is_command(line)
                    && !line.starts_with('(')
                    && !line.starts_with('=')
            })
            .take(6)
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    fn draw_progress(&self, f: &mut Frame, area: Rect) {
        let flow = Screen::flow(self.show_network);
        let total = flow.len().max(1);
        let step = flow
            .iter()
            .position(|&s| s == self.screen)
            .map(|i| i + 1)
            .unwrap_or(0);
        let count = format!("{step}/{total}");
        let bar_w = (area.width as usize).saturating_sub(count.chars().count() + 1);
        let filled = step * bar_w / total;
        let line = Line::from(vec![
            Span::styled(
                "━".repeat(filled),
                Style::default().fg(self.palette.primary),
            ),
            Span::styled(
                "─".repeat(bar_w - filled),
                Style::default().fg(self.palette.row_bg),
            ),
            Span::styled(format!(" {count}"), Style::default().fg(self.palette.muted)),
        ]);
        f.render_widget(Paragraph::new(line), area);
    }

    fn draw_body(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(self.palette.muted));
        f.render_widget(Clear, area);
        f.render_widget(block, area);
    }

    fn context_hints(&self) -> Vec<Span<'static>> {
        let muted = Style::default().fg(self.palette.muted);
        let accent = Style::default()
            .fg(self.palette.primary)
            .add_modifier(Modifier::BOLD);
        let enter = || Span::styled("⏎", accent);
        let sep = || Span::styled(" · ", muted);
        let text = |s: &str| Span::styled(s.to_string(), muted);
        let mut spans: Vec<Span> = Vec::new();
        match self.focus_kind() {
            FocusKind::Field { last: false } => {
                spans.push(enter());
                spans.push(text(" siguiente campo"));
                spans.push(sep());
                spans.push(text("esc atrás"));
            }
            FocusKind::Field { last: true } => {
                spans.push(enter());
                spans.push(text(" continuar"));
                spans.push(sep());
                spans.push(text("shift+tab campo anterior"));
                spans.push(sep());
                spans.push(text("esc atrás"));
            }
            FocusKind::List => {
                spans.push(text("↑↓ elegir"));
                spans.push(sep());
                spans.push(enter());
                spans.push(text(" continuar"));
                spans.push(sep());
                spans.push(text("escribe para buscar"));
                spans.push(sep());
                spans.push(text("esc atrás"));
            }
            FocusKind::Checkbox => {
                spans.push(text("espacio marcar"));
                spans.push(sep());
                spans.push(enter());
                spans.push(text(" siguiente"));
                spans.push(sep());
                spans.push(text("esc atrás"));
            }
            FocusKind::Buttons => {
                spans.push(text("←→ cambiar"));
                spans.push(sep());
                spans.push(enter());
                spans.push(text(" aceptar"));
                spans.push(sep());
                spans.push(text("esc atrás"));
            }
            FocusKind::Dialog => {
                spans.push(text("←→ cambiar"));
                spans.push(sep());
                spans.push(enter());
                spans.push(text(" aceptar"));
                spans.push(sep());
                spans.push(text("esc cerrar"));
            }
        }
        spans
    }

    fn draw_faltante(&self, f: &mut Frame, area: Rect) {
        if let Some(text) = self.faltante() {
            f.render_widget(
                Paragraph::new(Span::styled(
                    truncate_ellipsis(&text, area.width as usize),
                    Style::default().fg(self.palette.muted),
                ))
                .alignment(Alignment::Center),
                area,
            );
        }
    }

    fn draw_shortcuts(&self, f: &mut Frame, area: Rect) {
        if let Some(status) = &self.status {
            f.render_widget(
                Paragraph::new(Span::styled(
                    truncate_ellipsis(status, area.width as usize),
                    Style::default().fg(self.palette.warning),
                ))
                .alignment(Alignment::Center),
                area,
            );
            return;
        }
        let muted = Style::default().fg(self.palette.muted);
        let mut spans = self.context_hints();
        spans.push(Span::styled(" · ", muted));
        spans.push(Span::styled(
            format!("F2 teclado: {}", self.keyboard_active),
            muted,
        ));
        if !matches!(
            self.screen,
            Screen::Splash | Screen::License | Screen::Installing | Screen::Done
        ) {
            spans.push(Span::styled(" · ", muted));
            spans.push(Span::styled(
                format!(
                    "v comandos: {}",
                    if self.show_commands { "on" } else { "off" }
                ),
                muted,
            ));
        }
        f.render_widget(
            Paragraph::new(Line::from(spans)).alignment(Alignment::Center),
            area,
        );
    }

    fn draw_commands(&self, f: &mut Frame, area: Rect, lines: &[String], height: u16) {
        let block = Block::default()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(self.palette.border));
        let inner = block.inner(area);
        f.render_widget(Clear, inner);
        f.render_widget(block, area);

        let raw: Vec<String> = if self.screen == Screen::Installing {
            self.install
                .visible_log(height.saturating_sub(1) as usize)
                .into_iter()
                .cloned()
                .collect()
        } else {
            lines.iter().map(|l| format!("$ {l}")).collect()
        };

        let width = inner.width.max(1) as usize;
        let mut rows: Vec<Line> = Vec::new();
        for line in raw {
            self.log_rows(&line, width, &mut rows);
        }
        let rows = rows.split_off(rows.len().saturating_sub(inner.height as usize));
        f.render_widget(Paragraph::new(rows), inner);
    }

    fn log_line(&self, line: &str) -> Line<'static> {
        if let Some(text) = line.strip_prefix('\u{1}') {
            return Line::from(Span::styled(
                text.to_string(),
                Style::default().fg(self.palette.error),
            ));
        }
        let color = if line.starts_with('$') {
            self.palette.primary
        } else if line.starts_with("error") || line.contains("error:") {
            self.palette.error
        } else {
            self.palette.text
        };
        Line::from(Span::styled(line.to_string(), Style::default().fg(color)))
    }

    fn log_rows(&self, line: &str, width: usize, out: &mut Vec<Line>) {
        let is_error = line.starts_with('\u{1}');
        let text = line.strip_prefix('\u{1}').unwrap_or(line);
        let avail = if is_error {
            width.saturating_sub(2).max(1)
        } else {
            width
        };
        for chunk in hard_wrap(text, avail) {
            if is_error {
                out.push(Line::from(Span::styled(
                    format!("  {chunk}"),
                    Style::default().fg(self.palette.error),
                )));
            } else {
                out.push(self.log_line(&chunk));
            }
        }
    }

    fn draw_quit(&self, f: &mut Frame, over: Rect) {
        let palette = self.palette;
        let message = Line::from(Span::styled(
            "¿Salir del instalador?",
            Style::default().fg(palette.text),
        ));
        widgets::draw_dialog(f, over, &palette, message, |f, row| {
            f.render_widget(
                Paragraph::new(Span::styled(
                    "s / enter · n / esc",
                    Style::default().fg(palette.muted),
                ))
                .alignment(Alignment::Center),
                row,
            );
        });
    }

    fn draw_install_cancel(&self, f: &mut Frame, over: Rect) {
        let palette = self.palette;
        let buttons: Vec<Btn> = self.buttons().into_iter().map(|(b, _)| b).collect();
        let focus = self.button_focus;
        let focused = self.focus_buttons;
        let message = Line::from(Span::styled(
            "¿Cancelar la instalación? El disco quedará a medias.",
            Style::default().fg(palette.text),
        ));
        widgets::draw_dialog(f, over, &palette, message, |f, row| {
            let widget = ButtonRow {
                buttons: &buttons,
                focus,
                focused,
                palette: &palette,
            };
            f.render_widget(widget, row);
        });
    }

    pub fn command_list(&self) -> Vec<String> {
        match self.screen {
            Screen::Splash => Vec::new(),
            Screen::Network => crate::screens::network::commands(self),
            Screen::License => crate::screens::license::commands(self),
            Screen::Timezone => crate::screens::timezone::commands(self),
            Screen::Locale => crate::screens::locale::commands(self),
            Screen::Keyboard => crate::screens::keyboard::commands(self),
            Screen::Host => crate::screens::host::commands(self),
            Screen::Disk => crate::screens::disk::commands(self),
            Screen::Encryption => crate::screens::encryption::commands(self),
            Screen::Summary => crate::screens::summary::commands(self),
            Screen::Installing => crate::screens::installing::commands(self),
            Screen::Done => crate::screens::done::commands(self),
        }
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn human_duration(d: Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        format!("{secs}s")
    } else {
        format!("{}m {}s", secs / 60, secs % 60)
    }
}

fn fold(s: &str) -> String {
    s.chars()
        .flat_map(|c| c.to_lowercase())
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' | 'õ' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            other => other,
        })
        .collect()
}

fn is_command(line: &str) -> bool {
    line.starts_with('$') || line.starts_with("[simulación] $")
}

fn truncate_ellipsis(s: &str, width: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= width {
        return s.to_string();
    }
    if width == 0 {
        return String::new();
    }
    let mut out: String = chars[..width.saturating_sub(1)].iter().collect();
    out.push('…');
    out
}

fn hard_wrap(s: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![s.to_string()];
    }
    let chars: Vec<char> = s.chars().collect();
    if chars.is_empty() {
        return vec![String::new()];
    }
    chars.chunks(width).map(|c| c.iter().collect()).collect()
}
