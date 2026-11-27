use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use eframe::egui::{
    self, Align, Color32, CornerRadius, FontId, Frame, Margin, RichText, ScrollArea, Stroke,
    SystemTheme, TextureHandle, Ui, Vec2, ViewportCommand,
};

use crate::core::clicker::{ClickEngine, ClickSettings, StopReason};
use crate::core::config::{AppConfig, PROFILE_NAMES};
use crate::core::hotkeys::{
    conflicts_with_numeric_entry, display_name as hotkey_display_name, TriggerEvent,
    TriggerListener,
};
use crate::core::input_backend::{create_native_backend, InputBackend, MouseButton};
use crate::core::installer;
use crate::core::sound::play_cue;
use crate::core::system::startup_warnings;
use crate::core::updater::{self, ReleaseInfo};
use crate::core::validation::{
    allow_float, allow_int, allow_uint, effective_rate_hint, fmt_number, fmt_with_commas,
    parse_float, parse_int, sanitize_speed, to_interval_ms,
};
use crate::meta::{APP_NAME, CODENAME, ICON_PNG, VERSION};
use crate::ui::icons::HeaderIconKind;
use crate::ui::picker::{CoordinatePicker, PickerOutcome};
use crate::ui::theme::{apply_visuals, install_native_fonts, system_prefers_dark, Palette};
use crate::ui::widgets::{
    accent_button, card_frame, ghost_button, header_icon_button, keycap_button, number_field,
    row_label, section_title, segmented_control, status_pill, toggle_switch, CONTROL_H,
    WINDOW_WIDTH,
};

const STARTUP_UPDATE_DELAY_MS: u64 = 600;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpeedUnit {
    Ms,
    Sec,
    Min,
    Cps,
}

impl SpeedUnit {
    fn from_cfg(s: &str) -> Self {
        match s {
            "sec" => Self::Sec,
            "min" => Self::Min,
            "cps" => Self::Cps,
            _ => Self::Ms,
        }
    }
    fn as_str(self) -> &'static str {
        match self {
            Self::Ms => "ms",
            Self::Sec => "sec",
            Self::Min => "min",
            Self::Cps => "cps",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RepeatMode {
    Infinite,
    Fixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClickCadence {
    Single,
    Double,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PositionMode {
    Cursor,
    Fixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TriggerMode {
    Toggle,
    Hold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProfileChoice {
    Default,
    Fast,
    Humanized,
    Custom,
}

impl ProfileChoice {
    fn from_str(s: &str) -> Self {
        match s {
            "Fast" => Self::Fast,
            "Humanized" => Self::Humanized,
            "Custom" => Self::Custom,
            _ => Self::Default,
        }
    }
    fn as_str(self) -> &'static str {
        match self {
            Self::Default => PROFILE_NAMES[0],
            Self::Fast => PROFILE_NAMES[1],
            Self::Humanized => PROFILE_NAMES[2],
            Self::Custom => PROFILE_NAMES[3],
        }
    }
}

enum AppEvent {
    EngineStopped(StopReason),
    Hotkey(TriggerEvent),
    UpdateFound(ReleaseInfo, bool),
    UpdateError(String, bool),
    UpdateProgress(u64, u64),
    UpdateReady(PathBuf, PathBuf, ReleaseInfo),
    SetupProgress(u64, u64),
    SetupDone(&'static str, PathBuf),
    SetupError(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BannerTone {
    Accent,
    Neutral,
    Danger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BannerAction {
    UpdateNow,
    RestartAndUpdate,
    Later,
    SkipUpdate,
    CancelTransfer,
    OpenReleases,
    LaunchInstalled,
    Dismiss,
}

struct BannerState {
    title: String,
    detail: String,
    tone: BannerTone,
    progress: Option<f32>,
    actions: Vec<(&'static str, BannerAction, bool)>,
    auto_hide_at: Option<Instant>,
}

enum ModalDialog {
    UpdatePrompt { release: ReleaseInfo },
    SetupPrompt { create_desktop_shortcut: bool, has_local: bool, portable_ok: bool },
}

pub struct StartupOptions {
    pub preview_theme: Option<String>,
    pub preview_dialog: Option<String>,
    pub screenshot_out: Option<PathBuf>,
}

pub struct ClickForgeApp {
    config: AppConfig,
    backend: Arc<dyn InputBackend>,
    engine: Arc<ClickEngine>,
    hotkeys: TriggerListener,
    events_tx: Sender<AppEvent>,
    events_rx: Receiver<AppEvent>,

    logo_texture: Option<TextureHandle>,
    picker: CoordinatePicker,

    // UI text buffers + validation error flash timestamps
    speed_buf: String,
    speed_err: Option<Instant>,
    repeat_buf: String,
    repeat_err: Option<Instant>,
    x_buf: String,
    y_buf: String,
    coords_err: Option<Instant>,
    jitter_buf: String,
    pos_jitter_buf: String,
    hold_min_buf: String,
    hold_max_buf: String,
    human_err: Option<Instant>,

    // Runtime & status
    recording_hotkey: bool,
    any_field_focused: bool,
    last_os_dark: Option<bool>,
    last_theme_mode: String,
    last_target_h: f32,
    platform_warnings_checked: bool,

    // Auto-updater & Setup state
    startup_check_at: Option<Instant>,
    update_busy: bool,
    setup_busy: bool,
    update_release: Option<ReleaseInfo>,
    staged_update: Option<(PathBuf, PathBuf, ReleaseInfo)>,
    transfer_cancel: Option<Arc<AtomicBool>>,
    installed_exe_path: Option<PathBuf>,
    banner: Option<BannerState>,
    modal: Option<ModalDialog>,
    preview_mode: bool,
    screenshot_out: Option<PathBuf>,
    screenshot_frames: u32,
    screenshot_requested: bool,
}

impl ClickForgeApp {
    pub fn new(cc: &eframe::CreationContext<'_>, opts: StartupOptions) -> Self {
        install_native_fonts(&cc.egui_ctx);

        let mut config = AppConfig::load_default();
        if let Some(t) = &opts.preview_theme {
            if matches!(t.as_str(), "Dark" | "Light" | "System") {
                config.theme = t.clone();
            }
        }

        let pal = Palette::for_mode(&config.theme, &cc.egui_ctx);
        apply_visuals(&cc.egui_ctx, pal);
        cc.egui_ctx.send_viewport_cmd(ViewportCommand::SetTheme(
            if pal.is_dark {
                SystemTheme::Dark
            } else {
                SystemTheme::Light
            },
        ));
        cc.egui_ctx
            .send_viewport_cmd(ViewportCommand::WindowLevel(if config.always_on_top {
                egui::WindowLevel::AlwaysOnTop
            } else {
                egui::WindowLevel::Normal
            }));

        let logo_texture = decode_logo_texture(&cc.egui_ctx);
        let (events_tx, events_rx) = mpsc::channel();

        let backend: Arc<dyn InputBackend> = Arc::from(create_native_backend());
        let tx_engine = events_tx.clone();
        let egui_for_engine = cc.egui_ctx.clone();
        let engine = Arc::new(ClickEngine::with_backend(
            Arc::clone(&backend),
            Some(Arc::new(move |reason| {
                let _ = tx_engine.send(AppEvent::EngineStopped(reason));
                egui_for_engine.request_repaint();
            })),
        ));

        let tx_hk = events_tx.clone();
        let egui_for_hk = cc.egui_ctx.clone();
        let hotkeys = TriggerListener::new(
            &config.hotkey,
            Arc::new(move |ev| {
                let _ = tx_hk.send(AppEvent::Hotkey(ev));
                egui_for_hk.request_repaint();
            }),
        );
        hotkeys.start();

        let preview_mode = opts.preview_dialog.is_some()
            || opts.preview_theme.is_some()
            || opts.screenshot_out.is_some();
        let modal = match opts.preview_dialog.as_deref() {
            Some("setup") => Some(ModalDialog::SetupPrompt {
                create_desktop_shortcut: true,
                has_local: false,
                portable_ok: false,
            }),
            Some("update") => Some(ModalDialog::UpdatePrompt {
                release: ReleaseInfo {
                    tag: "v4.1.0".to_string(),
                    version: (4, 1, 0),
                    title: "v4.1.0 — \"Ferrous\"".to_string(),
                    page_url: updater::releases_page_url(),
                    notes: "### Highlights\n- Faster startup and lower idle RAM\n- Crisp Carbon & Precision Emerald polish\n- Seamless cross-platform CI verification".to_string(),
                    assets: vec![],
                },
            }),
            _ => None,
        };

        let startup_check_at = if !preview_mode && config.auto_update_check {
            Some(Instant::now() + Duration::from_millis(STARTUP_UPDATE_DELAY_MS))
        } else {
            None
        };

        let s = &config.settings;
        Self {
            speed_buf: fmt_number(s.speed_value),
            repeat_buf: s.repeat_count.to_string(),
            x_buf: s.fixed_x.to_string(),
            y_buf: s.fixed_y.to_string(),
            jitter_buf: fmt_number(s.jitter_pct),
            pos_jitter_buf: s.pos_jitter_px.to_string(),
            hold_min_buf: fmt_number(s.hold_min_ms),
            hold_max_buf: fmt_number(s.hold_max_ms),
            config,
            backend,
            engine,
            hotkeys,
            events_tx,
            events_rx,
            logo_texture,
            picker: CoordinatePicker::new(),
            speed_err: None,
            repeat_err: None,
            coords_err: None,
            human_err: None,
            recording_hotkey: false,
            any_field_focused: false,
            last_os_dark: Some(pal.is_dark),
            last_theme_mode: String::new(),
            last_target_h: 0.0,
            platform_warnings_checked: preview_mode,
            startup_check_at,
            update_busy: false,
            setup_busy: false,
            update_release: None,
            staged_update: None,
            transfer_cancel: None,
            installed_exe_path: None,
            banner: None,
            modal,
            preview_mode,
            screenshot_out: opts.screenshot_out,
            screenshot_frames: 0,
            screenshot_requested: false,
        }
    }

    fn sync_buffers_from_config(&mut self) {
        let s = &self.config.settings;
        self.speed_buf = fmt_number(s.speed_value);
        self.repeat_buf = s.repeat_count.to_string();
        self.x_buf = s.fixed_x.to_string();
        self.y_buf = s.fixed_y.to_string();
        self.jitter_buf = fmt_number(s.jitter_pct);
        self.pos_jitter_buf = s.pos_jitter_px.to_string();
        self.hold_min_buf = fmt_number(s.hold_min_ms);
        self.hold_max_buf = fmt_number(s.hold_max_ms);
    }

    fn commit_all_fields(&mut self) {
        self.commit_speed();
        self.commit_repeat();
        self.commit_coords();
        self.commit_humanizer();
    }

    fn commit_speed(&mut self) {
        if let Some(val) = sanitize_speed(&self.speed_buf, &self.config.settings.speed_unit) {
            if (val - self.config.settings.speed_value).abs() > 1e-6 {
                self.config.settings.speed_value = val;
                self.mark_custom();
            }
            self.speed_buf = fmt_number(val);
        } else {
            self.speed_err = Some(Instant::now() + Duration::from_millis(850));
            self.speed_buf = fmt_number(self.config.settings.speed_value);
        }
    }

    fn commit_repeat(&mut self) {
        if let Some(n) = parse_int(&self.repeat_buf) {
            if n >= 1 {
                let clamped = (n as u64).clamp(1, 10_000_000);
                if clamped != self.config.settings.repeat_count {
                    self.config.settings.repeat_count = clamped;
                    self.mark_custom();
                }
                self.repeat_buf = clamped.to_string();
                return;
            }
        }
        self.repeat_err = Some(Instant::now() + Duration::from_millis(850));
        self.repeat_buf = self.config.settings.repeat_count.to_string();
    }

    fn commit_coords(&mut self) {
        let x = parse_int(&self.x_buf);
        let y = parse_int(&self.y_buf);
        if let (Some(xv), Some(yv)) = (x, y) {
            let xc = xv.clamp(-32768, 32767);
            let yc = yv.clamp(-32768, 32767);
            if xc != self.config.settings.fixed_x || yc != self.config.settings.fixed_y {
                self.config.settings.fixed_x = xc;
                self.config.settings.fixed_y = yc;
                self.mark_custom();
            }
            self.x_buf = xc.to_string();
            self.y_buf = yc.to_string();
        } else {
            self.coords_err = Some(Instant::now() + Duration::from_millis(850));
            self.x_buf = self.config.settings.fixed_x.to_string();
            self.y_buf = self.config.settings.fixed_y.to_string();
        }
    }

    fn commit_humanizer(&mut self) {
        let j = parse_float(&self.jitter_buf);
        let pj = parse_int(&self.pos_jitter_buf);
        let hmin = parse_float(&self.hold_min_buf);
        let hmax = parse_float(&self.hold_max_buf);
        if let (Some(jv), Some(pjv), Some(minv), Some(maxv)) = (j, pj, hmin, hmax) {
            let j_c = jv.clamp(0.0, 80.0);
            let pj_c = pjv.clamp(0, 50);
            let mut lo = minv.clamp(1.0, 500.0);
            let mut hi = maxv.clamp(1.0, 500.0);
            if lo > hi {
                std::mem::swap(&mut lo, &mut hi);
            }
            self.config.settings.jitter_pct = j_c;
            self.config.settings.pos_jitter_px = pj_c;
            self.config.settings.hold_min_ms = lo;
            self.config.settings.hold_max_ms = hi;
            self.mark_custom();
            self.sync_buffers_from_config();
        } else {
            self.human_err = Some(Instant::now() + Duration::from_millis(850));
            self.sync_buffers_from_config();
        }
    }

    fn mark_custom(&mut self) {
        self.config.profile = "Custom".to_string();
        self.config.custom = self.config.settings.clone();
        if !self.preview_mode {
            let _ = self.config.save();
        }
    }

    fn build_click_settings(&self) -> ClickSettings {
        let s = &self.config.settings;
        ClickSettings {
            interval_ms: s.interval_ms(),
            button: MouseButton::from_str_lossy(&s.button),
            double_click: s.click_type == "double",
            repeat_count: if s.repeat_mode == "fixed" {
                s.repeat_count
            } else {
                0
            },
            fixed_position: if s.position_mode == "fixed" {
                Some((s.fixed_x, s.fixed_y))
            } else {
                None
            },
            humanize: s.humanize,
            jitter_pct: s.jitter_pct,
            hold_min_ms: s.hold_min_ms,
            hold_max_ms: s.hold_max_ms,
            position_jitter_px: s.pos_jitter_px,
            corner_failsafe: true,
        }
    }

    fn start_clicking(&mut self) {
        if self.engine.is_active() || self.recording_hotkey || self.picker.is_active() {
            return;
        }
        self.commit_all_fields();
        self.modal = None;
        let settings = self.build_click_settings();
        if self.engine.start(settings) {
            play_cue(true, self.config.sound_cue);
        }
    }

    fn stop_clicking(&mut self, wait: bool) {
        let was_active = self.engine.is_active();
        self.engine.stop(wait);
        if was_active {
            play_cue(false, self.config.sound_cue);
        }
    }

    fn toggle_clicking(&mut self) {
        if self.engine.is_active() {
            self.stop_clicking(false);
        } else {
            self.start_clicking();
        }
    }

    fn process_events(&mut self, ctx: &egui::Context) {
        while let Ok(ev) = self.events_rx.try_recv() {
            match ev {
                AppEvent::EngineStopped(reason) => {
                    self.hotkeys.reset_trigger_state();
                    if matches!(reason, StopReason::Completed | StopReason::Failsafe) {
                        play_cue(false, self.config.sound_cue);
                    }
                    if reason == StopReason::Failsafe {
                        self.banner = Some(BannerState {
                            title: "Emergency Failsafe Triggered".to_string(),
                            detail: "Pointer entered top-left corner (0, 0).".to_string(),
                            tone: BannerTone::Danger,
                            progress: None,
                            actions: vec![("Dismiss", BannerAction::Dismiss, false)],
                            auto_hide_at: Some(Instant::now() + Duration::from_secs(4)),
                        });
                    }
                }
                AppEvent::Hotkey(hk) => match hk {
                    TriggerEvent::Panic => {
                        self.stop_clicking(false);
                    }
                    TriggerEvent::Down => {
                        if self.config.trigger_mode == "hold" {
                            self.start_clicking();
                        } else {
                            self.toggle_clicking();
                        }
                    }
                    TriggerEvent::Up => {
                        if self.config.trigger_mode == "hold" {
                            self.stop_clicking(false);
                        }
                    }
                    TriggerEvent::Recorded(binding) => {
                        self.recording_hotkey = false;
                        self.config.hotkey = binding;
                        if !self.preview_mode {
                            let _ = self.config.save();
                        }
                    }
                    TriggerEvent::RecordCancelled => {
                        self.recording_hotkey = false;
                    }
                },
                AppEvent::UpdateFound(release, manual) => {
                    self.update_busy = false;
                    if let Ok(d) = SystemTime::now().duration_since(UNIX_EPOCH) {
                        self.config.last_update_check = d.as_secs_f64();
                        if !self.preview_mode {
                            let _ = self.config.save();
                        }
                    }
                    if updater::is_newer(release.version, None) {
                        let ver = release.version_text();
                        self.update_release = Some(release.clone());
                        if !manual && ver == self.config.skipped_version {
                            continue;
                        }
                        if self.engine.is_active() {
                            self.banner = Some(BannerState {
                                title: format!("Update available: v{ver}"),
                                detail: format!("You're on v{VERSION} · Update now?"),
                                tone: BannerTone::Accent,
                                progress: None,
                                actions: vec![
                                    ("Skip", BannerAction::SkipUpdate, false),
                                    ("Later", BannerAction::Later, false),
                                    ("Update Now", BannerAction::UpdateNow, true),
                                ],
                                auto_hide_at: None,
                            });
                        } else {
                            self.banner = None;
                            self.modal = Some(ModalDialog::UpdatePrompt { release });
                        }
                    } else {
                        self.update_release = None;
                        if manual {
                            self.banner = Some(BannerState {
                                title: format!("You're up to date (v{VERSION})"),
                                detail: format!("Latest release: v{}", release.version_text()),
                                tone: BannerTone::Neutral,
                                progress: None,
                                actions: vec![("Dismiss", BannerAction::Dismiss, false)],
                                auto_hide_at: Some(Instant::now() + Duration::from_secs(4)),
                            });
                        }
                    }
                }
                AppEvent::UpdateError(msg, manual) => {
                    self.update_busy = false;
                    self.transfer_cancel = None;
                    if msg == "Download cancelled." {
                        self.banner = None;
                    } else if manual {
                        self.banner = Some(BannerState {
                            title: "Update check failed".to_string(),
                            detail: msg,
                            tone: BannerTone::Danger,
                            progress: None,
                            actions: vec![
                                ("Releases", BannerAction::OpenReleases, false),
                                ("Dismiss", BannerAction::Dismiss, false),
                            ],
                            auto_hide_at: None,
                        });
                    }
                }
                AppEvent::UpdateProgress(got, total) => {
                    if let Some(b) = &mut self.banner {
                        if total > 0 {
                            b.progress = Some((got as f32 / total as f32).clamp(0.0, 1.0));
                            b.detail = format!(
                                "{:.1} MB / {:.1} MB",
                                got as f64 / (1024.0 * 1024.0),
                                total as f64 / (1024.0 * 1024.0)
                            );
                        } else {
                            b.detail = format!("{:.1} MB downloaded", got as f64 / (1024.0 * 1024.0));
                        }
                    }
                }
                AppEvent::UpdateReady(staged, workdir, release) => {
                    self.update_busy = false;
                    self.transfer_cancel = None;
                    let ver = release.version_text();
                    self.staged_update = Some((staged, workdir, release));
                    self.banner = Some(BannerState {
                        title: format!("Update v{ver} ready to install"),
                        detail: "Click Restart & Update to apply and relaunch.".to_string(),
                        tone: BannerTone::Accent,
                        progress: Some(1.0),
                        actions: vec![
                            ("Later", BannerAction::Later, false),
                            ("Restart & Update", BannerAction::RestartAndUpdate, true),
                        ],
                        auto_hide_at: None,
                    });
                }
                AppEvent::SetupProgress(got, total) => {
                    if let Some(b) = &mut self.banner {
                        if total > 0 {
                            b.progress = Some((got as f32 / total as f32).clamp(0.0, 1.0));
                            b.detail = format!(
                                "{:.1} MB / {:.1} MB",
                                got as f64 / (1024.0 * 1024.0),
                                total as f64 / (1024.0 * 1024.0)
                            );
                        }
                    }
                }
                AppEvent::SetupDone(kind, path) => {
                    self.setup_busy = false;
                    self.transfer_cancel = None;
                    if kind == "portable" {
                        self.installed_exe_path = Some(path);
                        self.banner = Some(BannerState {
                            title: "ClickForge installed to Windows Programs".to_string(),
                            detail: "Available in Start Menu and Installed Apps.".to_string(),
                            tone: BannerTone::Accent,
                            progress: None,
                            actions: vec![
                                ("Dismiss", BannerAction::Dismiss, false),
                                ("Launch Installed", BannerAction::LaunchInstalled, true),
                            ],
                            auto_hide_at: None,
                        });
                    } else {
                        self.banner = Some(BannerState {
                            title: "Setup wizard started".to_string(),
                            detail: "Follow ClickForge Setup to finish installing.".to_string(),
                            tone: BannerTone::Accent,
                            progress: None,
                            actions: vec![("Dismiss", BannerAction::Dismiss, false)],
                            auto_hide_at: Some(Instant::now() + Duration::from_secs(5)),
                        });
                    }
                }
                AppEvent::SetupError(msg) => {
                    self.setup_busy = false;
                    self.transfer_cancel = None;
                    if msg == "Download cancelled." {
                        self.banner = None;
                    } else {
                        self.banner = Some(BannerState {
                            title: "Setup could not complete".to_string(),
                            detail: msg,
                            tone: BannerTone::Danger,
                            progress: None,
                            actions: vec![
                                ("Releases", BannerAction::OpenReleases, false),
                                ("Dismiss", BannerAction::Dismiss, false),
                            ],
                            auto_hide_at: None,
                        });
                    }
                }
            }
        }

        if let Some(b) = &self.banner {
            if b.auto_hide_at.is_some_and(|t| Instant::now() >= t) {
                self.banner = None;
            }
        }
        if let Some(when) = self.startup_check_at {
            if Instant::now() >= when {
                self.startup_check_at = None;
                self.check_for_updates(false);
            }
        }
        if !self.platform_warnings_checked {
            self.platform_warnings_checked = true;
            if let Some(warn) = startup_warnings().into_iter().next() {
                self.banner = Some(BannerState {
                    title: "Platform Permission Notice".to_string(),
                    detail: warn,
                    tone: BannerTone::Neutral,
                    progress: None,
                    actions: vec![("Dismiss", BannerAction::Dismiss, false)],
                    auto_hide_at: None,
                });
            }
        }
        ctx.input(|_| {});
    }

    fn check_for_updates(&mut self, manual: bool) {
        if self.update_busy {
            return;
        }
        if manual {
            if let Some((staged, _, rel)) = &self.staged_update {
                if staged.is_dir() {
                    let ver = rel.version_text();
                    self.banner = Some(BannerState {
                        title: format!("Update v{ver} ready to install"),
                        detail: "Click Restart & Update to apply and relaunch.".to_string(),
                        tone: BannerTone::Accent,
                        progress: Some(1.0),
                        actions: vec![
                            ("Later", BannerAction::Later, false),
                            ("Restart & Update", BannerAction::RestartAndUpdate, true),
                        ],
                        auto_hide_at: None,
                    });
                    return;
                }
                self.staged_update = None;
            }
        }

        self.update_busy = true;
        if manual {
            self.banner = Some(BannerState {
                title: "Checking for updates...".to_string(),
                detail: format!("Current version: v{VERSION}"),
                tone: BannerTone::Neutral,
                progress: None,
                actions: vec![],
                auto_hide_at: None,
            });
        }

        let tx = self.events_tx.clone();
        thread::Builder::new()
            .name("ClickForge-UpdateCheck".to_string())
            .spawn(move || match updater::fetch_latest() {
                Ok(rel) => {
                    let _ = tx.send(AppEvent::UpdateFound(rel, manual));
                }
                Err(err) => {
                    let _ = tx.send(AppEvent::UpdateError(err, manual));
                }
            })
            .ok();
    }

    fn start_update_install(&mut self) {
        let Some(rel) = self.update_release.clone() else {
            let _ = open::that(updater::releases_page_url());
            return;
        };
        let (can_self, _) = updater::can_self_update();
        if !can_self {
            let _ = open::that(&rel.page_url);
            return;
        }
        let Some(asset) = rel.asset_for_platform().cloned() else {
            let _ = open::that(&rel.page_url);
            return;
        };

        self.update_busy = true;
        let cancel = Arc::new(AtomicBool::new(false));
        self.transfer_cancel = Some(Arc::clone(&cancel));
        self.banner = Some(BannerState {
            title: format!("Downloading ClickForge v{}...", rel.version_text()),
            detail: "Connecting to GitHub Releases...".to_string(),
            tone: BannerTone::Accent,
            progress: Some(0.0),
            actions: vec![("Cancel", BannerAction::CancelTransfer, false)],
            auto_hide_at: None,
        });

        let tx = self.events_tx.clone();
        thread::Builder::new()
            .name("ClickForge-UpdateDownload".to_string())
            .spawn(move || {
                let workdir = std::env::temp_dir().join("clickforge-update");
                let mut last_emit = Instant::now();
                let res = updater::download_asset(
                    &asset,
                    &workdir,
                    |got, total| {
                        if last_emit.elapsed() >= Duration::from_millis(60)
                            || (total > 0 && got >= total)
                        {
                            last_emit = Instant::now();
                            let _ = tx.send(AppEvent::UpdateProgress(got, total));
                        }
                    },
                    &cancel,
                );
                match res {
                    Ok(zip_path) => match updater::stage_update(&zip_path, &workdir) {
                        Ok(staged) => {
                            let _ = tx.send(AppEvent::UpdateReady(staged, workdir, rel));
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::UpdateError(e, true));
                        }
                    },
                    Err(e) => {
                        let _ = tx.send(AppEvent::UpdateError(e, true));
                    }
                }
            })
            .ok();
    }

    fn handle_banner_action(&mut self, action: BannerAction, ctx: &egui::Context) {
        match action {
            BannerAction::UpdateNow => {
                self.banner = None;
                self.start_update_install();
            }
            BannerAction::RestartAndUpdate => {
                if let Some((staged, workdir, _)) = self.staged_update.take() {
                    if let Err(err) = updater::launch_self_updater(&staged, &workdir) {
                        self.banner = Some(BannerState {
                            title: "Could not restart updater".to_string(),
                            detail: err,
                            tone: BannerTone::Danger,
                            progress: None,
                            actions: vec![
                                ("Releases", BannerAction::OpenReleases, false),
                                ("Dismiss", BannerAction::Dismiss, false),
                            ],
                            auto_hide_at: None,
                        });
                    } else {
                        self.stop_clicking(true);
                        self.hotkeys.stop();
                        let _ = self.config.save();
                        ctx.send_viewport_cmd(ViewportCommand::Close);
                    }
                }
            }
            BannerAction::Later | BannerAction::Dismiss => {
                self.banner = None;
            }
            BannerAction::SkipUpdate => {
                if let Some(rel) = &self.update_release {
                    self.config.skipped_version = rel.version_text();
                    let _ = self.config.save();
                }
                self.banner = None;
            }
            BannerAction::CancelTransfer => {
                if let Some(c) = &self.transfer_cancel {
                    c.store(true, std::sync::atomic::Ordering::Release);
                }
                self.banner = None;
            }
            BannerAction::OpenReleases => {
                let url = self
                    .update_release
                    .as_ref()
                    .map(|r| r.page_url.clone())
                    .unwrap_or_else(updater::releases_page_url);
                let _ = open::that(url);
            }
            BannerAction::LaunchInstalled => {
                if let Some(path) = self.installed_exe_path.clone() {
                    if installer::launch_exe(&path).is_ok() {
                        self.stop_clicking(true);
                        self.hotkeys.stop();
                        let _ = self.config.save();
                        ctx.send_viewport_cmd(ViewportCommand::Close);
                    }
                }
            }
        }
    }

    fn run_windows_setup(&mut self, desktop_shortcut: bool, has_local: bool, portable_ok: bool) {
        self.modal = None;
        if self.setup_busy {
            return;
        }
        if has_local {
            if let Some(path) = installer::find_local_installer() {
                match installer::launch_exe(&path) {
                    Ok(()) => {
                        self.banner = Some(BannerState {
                            title: "Launched ClickForge Setup".to_string(),
                            detail: "Follow the Windows installer wizard to complete setup.".to_string(),
                            tone: BannerTone::Accent,
                            progress: None,
                            actions: vec![("Dismiss", BannerAction::Dismiss, false)],
                            auto_hide_at: Some(Instant::now() + Duration::from_secs(5)),
                        });
                    }
                    Err(e) => {
                        let _ = self.events_tx.send(AppEvent::SetupError(e));
                    }
                }
                return;
            }
        }

        self.setup_busy = true;
        let tx = self.events_tx.clone();
        if portable_ok {
            self.banner = Some(BannerState {
                title: "Installing ClickForge to Programs...".to_string(),
                detail: "Creating Start Menu & Installed Apps entries".to_string(),
                tone: BannerTone::Accent,
                progress: Some(0.5),
                actions: vec![],
                auto_hide_at: None,
            });
            thread::Builder::new()
                .name("ClickForge-Setup".to_string())
                .spawn(move || match installer::install_portable_to_programs(desktop_shortcut) {
                    Ok(p) => {
                        let _ = tx.send(AppEvent::SetupDone("portable", p));
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::SetupError(e));
                    }
                })
                .ok();
            return;
        }

        let cancel = Arc::new(AtomicBool::new(false));
        self.transfer_cancel = Some(Arc::clone(&cancel));
        self.banner = Some(BannerState {
            title: "Downloading ClickForge-Setup.exe...".to_string(),
            detail: "Connecting to GitHub Releases...".to_string(),
            tone: BannerTone::Accent,
            progress: Some(0.0),
            actions: vec![("Cancel", BannerAction::CancelTransfer, false)],
            auto_hide_at: None,
        });
        thread::Builder::new()
            .name("ClickForge-SetupDownload".to_string())
            .spawn(move || {
                let mut last_emit = Instant::now();
                let res = installer::download_and_launch_setup(
                    |got, total| {
                        if last_emit.elapsed() >= Duration::from_millis(60)
                            || (total > 0 && got >= total)
                        {
                            last_emit = Instant::now();
                            let _ = tx.send(AppEvent::SetupProgress(got, total));
                        }
                    },
                    &cancel,
                );
                match res {
                    Ok(p) => {
                        let _ = tx.send(AppEvent::SetupDone("wizard", p));
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::SetupError(e));
                    }
                }
            })
            .ok();
    }

    fn render_header(&mut self, ui: &mut Ui, p: &Palette, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            if let Some(tex) = &self.logo_texture {
                ui.image((tex.id(), Vec2::splat(22.0)));
            }
            ui.label(
                RichText::new(APP_NAME)
                    .font(FontId::proportional(15.5))
                    .strong()
                    .color(p.text),
            );
            ui.label(
                RichText::new(format!("v{VERSION}"))
                    .font(FontId::monospace(11.0))
                    .color(p.text_faint),
            )
            .on_hover_text(format!("{APP_NAME} v{VERSION} — {CODENAME}"));

            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let active = self.engine.is_active();
                status_pill(ui, p, active);

                if cfg!(target_os = "windows") && !installer::is_installed_in_programs(None) {
                    let btn = ghost_button(ui, p, "Setup", Vec2::new(50.0, 28.0), !self.setup_busy);
                    if btn
                        .on_hover_text("Install ClickForge to Windows Programs")
                        .clicked()
                    {
                        self.modal = Some(ModalDialog::SetupPrompt {
                            create_desktop_shortcut: true,
                            has_local: installer::find_local_installer().is_some(),
                            portable_ok: installer::can_install_portable(),
                        });
                    }
                }

                let has_newer = self
                    .update_release
                    .as_ref()
                    .is_some_and(|r| updater::is_newer(r.version, None));
                let upd_tip = match &self.update_release {
                    Some(r) if updater::is_newer(r.version, None) => {
                        format!("Update available: v{}", r.version_text())
                    }
                    _ => "Check for updates".to_string(),
                };
                if header_icon_button(
                    ui,
                    p,
                    HeaderIconKind::Update,
                    false,
                    has_newer.then_some(p.accent),
                    &upd_tip,
                )
                .clicked()
                {
                    self.check_for_updates(true);
                }

                let pin_kind = if self.config.always_on_top {
                    HeaderIconKind::PinOn
                } else {
                    HeaderIconKind::PinOff
                };
                let pin_tip = if self.config.always_on_top {
                    "Always on top: On"
                } else {
                    "Always on top: Off"
                };
                if header_icon_button(ui, p, pin_kind, self.config.always_on_top, None, pin_tip)
                    .clicked()
                {
                    self.config.always_on_top = !self.config.always_on_top;
                    ctx.send_viewport_cmd(ViewportCommand::WindowLevel(
                        if self.config.always_on_top {
                            egui::WindowLevel::AlwaysOnTop
                        } else {
                            egui::WindowLevel::Normal
                        },
                    ));
                    if !self.preview_mode {
                        let _ = self.config.save();
                    }
                }

                let theme_kind = match self.config.theme.as_str() {
                    "Light" => HeaderIconKind::ThemeLight,
                    "Dark" => HeaderIconKind::ThemeDark,
                    _ => HeaderIconKind::ThemeSystem,
                };
                let theme_tip = format!("Theme: {} (click to cycle)", self.config.theme);
                if header_icon_button(ui, p, theme_kind, false, None, &theme_tip).clicked() {
                    self.config.theme = match self.config.theme.as_str() {
                        "System" => "Dark",
                        "Dark" => "Light",
                        _ => "System",
                    }
                    .to_string();
                    if !self.preview_mode {
                        let _ = self.config.save();
                    }
                }

                let sound_kind = if self.config.sound_cue {
                    HeaderIconKind::SoundOn
                } else {
                    HeaderIconKind::SoundOff
                };
                let sound_tip = if self.config.sound_cue {
                    "Audio cue: On"
                } else {
                    "Audio cue: Off"
                };
                if header_icon_button(
                    ui,
                    p,
                    sound_kind,
                    self.config.sound_cue,
                    None,
                    sound_tip,
                )
                .clicked()
                {
                    self.config.sound_cue = !self.config.sound_cue;
                    if self.config.sound_cue {
                        play_cue(true, true);
                    }
                    if !self.preview_mode {
                        let _ = self.config.save();
                    }
                }
            });
        });
    }

    fn render_banner(&mut self, ui: &mut Ui, p: &Palette, ctx: &egui::Context) {
        let Some(b) = self.banner.as_ref() else {
            return;
        };
        let border_col = match b.tone {
            BannerTone::Accent => p.accent,
            BannerTone::Danger => p.danger,
            BannerTone::Neutral => p.border_strong,
        };
        let actions = b.actions.clone();
        let title = b.title.clone();
        let detail = b.detail.clone();
        let progress = b.progress;
        let mut clicked_action: Option<BannerAction> = None;

        Frame::new()
            .fill(p.surface)
            .stroke(Stroke::new(1.0_f32, border_col))
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::symmetric(12, 10))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    RichText::new(title)
                        .font(FontId::proportional(12.5))
                        .strong()
                        .color(p.text),
                );
                if !detail.is_empty() {
                    ui.label(
                        RichText::new(detail)
                            .font(FontId::proportional(11.5))
                            .color(p.text_muted),
                    );
                }
                if let Some(frac) = progress {
                    ui.add_space(4.0);
                    let bar_w = ui.available_width();
                    let (bar_rect, _) =
                        ui.allocate_exact_size(Vec2::new(bar_w, 5.0), egui::Sense::hover());
                    ui.painter()
                        .rect_filled(bar_rect, CornerRadius::same(3), p.field);
                    if frac > 0.0 {
                        let mut fill_rect = bar_rect;
                        fill_rect.max.x = bar_rect.min.x + bar_rect.width() * frac;
                        ui.painter()
                            .rect_filled(fill_rect, CornerRadius::same(3), p.accent);
                    }
                }
                if !actions.is_empty() {
                    ui.add_space(6.0);
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        for (label, act, primary) in actions.into_iter().rev() {
                            let w = (label.len() as f32 * 7.2 + 24.0).max(66.0);
                            let r = if primary {
                                accent_button(ui, p, label, Vec2::new(w, 26.0), false, 11.5, 6)
                            } else {
                                ghost_button(ui, p, label, Vec2::new(w, 26.0), true)
                            };
                            if r.clicked() {
                                clicked_action = Some(act);
                            }
                        }
                    });
                }
            });

        if let Some(act) = clicked_action {
            self.handle_banner_action(act, ctx);
        }
        ui.add_space(8.0);
    }

    fn render_telemetry_card(&mut self, ui: &mut Ui, p: &Palette) {
        let snap = self.engine.snapshot();
        let active = snap.active;

        card_frame(p, active).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(fmt_with_commas(snap.cps as u64))
                        .font(FontId::monospace(27.0))
                        .strong()
                        .color(if active { p.accent } else { p.text }),
                );
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 1.0;
                    section_title(ui, p, "LIVE CPS");
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        let total_text = if snap.target > 0 {
                            format!(
                                "Total: {} / {}",
                                fmt_with_commas(snap.total),
                                fmt_with_commas(snap.target)
                            )
                        } else {
                            format!("Total: {}", fmt_with_commas(snap.total))
                        };
                        ui.label(
                            RichText::new(total_text)
                                .font(FontId::monospace(11.0))
                                .color(p.text_muted),
                        );

                        let d = snap.elapsed_secs;
                        let mins = (d as u64) / 60;
                        let secs = d - (mins * 60) as f64;
                        ui.label(
                            RichText::new(format!("· {mins:02}:{secs:04.1}"))
                                .font(FontId::monospace(11.0))
                                .color(p.text_faint),
                        )
                        .on_hover_text("Elapsed run duration");
                    });
                });

                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    if ghost_button(ui, p, "Reset", Vec2::new(60.0, 28.0), true)
                        .on_hover_text("Reset click counter and timer")
                        .clicked()
                    {
                        self.engine.reset_stats();
                    }
                });
            });

            // 1px hairline divider
            ui.add_space(7.0);
            let div_rect =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), egui::Sense::hover())
                    .0;
            ui.painter().rect_filled(div_rect, 0, p.border);
            ui.add_space(7.0);

            // Preset selector row
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(
                    Vec2::new(50.0, CONTROL_H),
                    egui::Layout::left_to_right(Align::Center),
                    |ui| {
                        ui.label(
                            RichText::new("Preset")
                                .font(FontId::proportional(11.5))
                                .color(p.text_muted),
                        );
                    },
                );
                let mut choice = ProfileChoice::from_str(&self.config.profile);
                let seg_w = ui.available_width();
                if segmented_control(
                    ui,
                    p,
                    "preset_seg",
                    &mut choice,
                    &[
                        (ProfileChoice::Default, "Default"),
                        (ProfileChoice::Fast, "Fast"),
                        (ProfileChoice::Humanized, "Humanized"),
                        (ProfileChoice::Custom, "Custom"),
                    ],
                    !active,
                    seg_w,
                ) {
                    self.config.apply_preset(choice.as_str());
                    self.sync_buffers_from_config();
                    if !self.preview_mode {
                        let _ = self.config.save();
                    }
                }
            });
        });
    }

    fn render_speed_card(&mut self, ui: &mut Ui, p: &Palette) {
        let locked = self.engine.is_active();
        let editable = !locked;
        card_frame(p, false).show(ui, |ui| {
            ui.set_width(ui.available_width());
            section_title(ui, p, "Speed & Repeat");
            ui.add_space(6.0);

            // Speed row
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let row_lbl = if self.config.settings.speed_unit == "cps" {
                    "Target CPS"
                } else {
                    "Interval"
                };
                row_label(ui, p, row_lbl, editable);
                let (resp, committed) = number_field(
                    ui,
                    p,
                    "speed_val",
                    &mut self.speed_buf,
                    74.0,
                    allow_float,
                    editable,
                    self.speed_err,
                );
                if resp.has_focus() {
                    self.any_field_focused = true;
                }
                if committed {
                    self.commit_speed();
                }

                let mut unit = SpeedUnit::from_cfg(&self.config.settings.speed_unit);
                let seg_w = ui.available_width();
                if segmented_control(
                    ui,
                    p,
                    "speed_unit_seg",
                    &mut unit,
                    &[
                        (SpeedUnit::Ms, "ms"),
                        (SpeedUnit::Sec, "sec"),
                        (SpeedUnit::Min, "min"),
                        (SpeedUnit::Cps, "CPS"),
                    ],
                    editable,
                    seg_w,
                ) {
                    let old_ms = to_interval_ms(
                        sanitize_speed(&self.speed_buf, &self.config.settings.speed_unit)
                            .unwrap_or(self.config.settings.speed_value),
                        &self.config.settings.speed_unit,
                    );
                    let new_unit_str = unit.as_str();
                    let new_val = (crate::core::validation::from_interval_ms(old_ms, new_unit_str)
                        * 1000.0)
                        .round()
                        / 1000.0;
                    self.config.settings.speed_unit = new_unit_str.to_string();
                    self.config.settings.speed_value = new_val.max(0.001);
                    self.speed_buf = fmt_number(self.config.settings.speed_value);
                    self.mark_custom();
                }
            });

            // Live rate hint
            let preview_val = sanitize_speed(&self.speed_buf, &self.config.settings.speed_unit)
                .unwrap_or(self.config.settings.speed_value);
            let hint = effective_rate_hint(preview_val, &self.config.settings.speed_unit);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.add_space(row_label_w_pad());
                ui.label(
                    RichText::new(hint)
                        .font(FontId::monospace(11.0))
                        .color(p.text_faint),
                );
            });
            ui.add_space(6.0);

            // Repeat row
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                row_label(ui, p, "Repeat", editable);
                let mut mode = if self.config.settings.repeat_mode == "fixed" {
                    RepeatMode::Fixed
                } else {
                    RepeatMode::Infinite
                };
                let reserve = if mode == RepeatMode::Fixed { 116.0 } else { 0.0 };
                let seg_w = (ui.available_width() - reserve).max(120.0);
                if segmented_control(
                    ui,
                    p,
                    "repeat_mode_seg",
                    &mut mode,
                    &[
                        (RepeatMode::Infinite, "Infinite"),
                        (RepeatMode::Fixed, "Fixed"),
                    ],
                    editable,
                    seg_w,
                ) {
                    self.config.settings.repeat_mode = match mode {
                        RepeatMode::Infinite => "infinite",
                        RepeatMode::Fixed => "fixed",
                    }
                    .to_string();
                    self.mark_custom();
                }

                if mode == RepeatMode::Fixed {
                    let (resp, committed) = number_field(
                        ui,
                        p,
                        "repeat_cnt",
                        &mut self.repeat_buf,
                        68.0,
                        allow_uint,
                        editable,
                        self.repeat_err,
                    );
                    if resp.has_focus() {
                        self.any_field_focused = true;
                    }
                    if committed {
                        self.commit_repeat();
                    }
                    ui.label(
                        RichText::new("clicks")
                            .font(FontId::proportional(11.5))
                            .color(p.text_muted),
                    );
                }
            });
        });
    }

    fn render_target_card(&mut self, ui: &mut Ui, p: &Palette) {
        let editable = !self.engine.is_active();
        card_frame(p, false).show(ui, |ui| {
            ui.set_width(ui.available_width());
            section_title(ui, p, "Action & Target");
            ui.add_space(6.0);

            // Button + 1x/2x row
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                row_label(ui, p, "Button", editable);
                let mut btn = MouseButton::from_str_lossy(&self.config.settings.button);
                let btn_w = (ui.available_width() - 88.0).max(150.0);
                if segmented_control(
                    ui,
                    p,
                    "button_seg",
                    &mut btn,
                    &[
                        (MouseButton::Left, "Left"),
                        (MouseButton::Right, "Right"),
                        (MouseButton::Middle, "Middle"),
                    ],
                    editable,
                    btn_w,
                ) {
                    self.config.settings.button = btn.as_str().to_string();
                    self.mark_custom();
                }

                let mut cadence = if self.config.settings.click_type == "double" {
                    ClickCadence::Double
                } else {
                    ClickCadence::Single
                };
                if segmented_control(
                    ui,
                    p,
                    "cadence_seg",
                    &mut cadence,
                    &[(ClickCadence::Single, "1x"), (ClickCadence::Double, "2x")],
                    editable,
                    82.0,
                ) {
                    self.config.settings.click_type = match cadence {
                        ClickCadence::Single => "single",
                        ClickCadence::Double => "double",
                    }
                    .to_string();
                    self.mark_custom();
                }
            });
            ui.add_space(6.0);

            // Position mode row
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                row_label(ui, p, "Position", editable);
                let mut pos_mode = if self.config.settings.position_mode == "fixed" {
                    PositionMode::Fixed
                } else {
                    PositionMode::Cursor
                };
                let seg_w = ui.available_width();
                if segmented_control(
                    ui,
                    p,
                    "pos_mode_seg",
                    &mut pos_mode,
                    &[
                        (PositionMode::Cursor, "Follow Cursor"),
                        (PositionMode::Fixed, "Fixed XY"),
                    ],
                    editable,
                    seg_w,
                ) {
                    self.config.settings.position_mode = match pos_mode {
                        PositionMode::Cursor => "cursor",
                        PositionMode::Fixed => "fixed",
                    }
                    .to_string();
                    self.mark_custom();
                }
            });

            // Adaptive Coords row (shown only when Fixed XY is active)
            if self.config.settings.position_mode == "fixed" {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    row_label(ui, p, "Coords", editable);
                    ui.label(
                        RichText::new("X")
                            .font(FontId::monospace(11.0))
                            .color(p.text_muted),
                    );
                    let (rx, cx) = number_field(
                        ui,
                        p,
                        "fixed_x",
                        &mut self.x_buf,
                        68.0,
                        allow_int,
                        editable,
                        self.coords_err,
                    );
                    if rx.has_focus() {
                        self.any_field_focused = true;
                    }
                    if cx {
                        self.commit_coords();
                    }

                    ui.label(
                        RichText::new("Y")
                            .font(FontId::monospace(11.0))
                            .color(p.text_muted),
                    );
                    let (ry, cy) = number_field(
                        ui,
                        p,
                        "fixed_y",
                        &mut self.y_buf,
                        68.0,
                        allow_int,
                        editable,
                        self.coords_err,
                    );
                    if ry.has_focus() {
                        self.any_field_focused = true;
                    }
                    if cy {
                        self.commit_coords();
                    }

                    let pick_w = ui.available_width().max(64.0);
                    let pick_resp = if editable {
                        accent_button(ui, p, "Pick", Vec2::new(pick_w, CONTROL_H), false, 12.0, 7)
                    } else {
                        ghost_button(ui, p, "Pick", Vec2::new(pick_w, CONTROL_H), false)
                    };
                    if pick_resp
                        .on_hover_text(
                            "Pick target screen coordinates (Esc or right-click to cancel)",
                        )
                        .clicked()
                        && editable
                    {
                        self.picker.open();
                    }
                });
            }
        });
    }

    fn render_humanizer_card(&mut self, ui: &mut Ui, p: &Palette) {
        let editable = !self.engine.is_active();
        card_frame(p, false).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                section_title(ui, p, "Humanizer");
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    let mut checked = self.config.settings.humanize;
                    if toggle_switch(ui, p, &mut checked, editable) {
                        self.config.settings.humanize = checked;
                        self.mark_custom();
                    }
                    ui.label(
                        RichText::new(if self.config.settings.humanize {
                            "Active"
                        } else {
                            "Off"
                        })
                        .font(FontId::proportional(11.5))
                        .color(if self.config.settings.humanize {
                            p.accent
                        } else {
                            p.text_faint
                        }),
                    );
                });
            });

            if self.config.settings.humanize {
                ui.add_space(7.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 5.0;
                    row_label(ui, p, "Jitter", editable);
                    ui.label(
                        RichText::new("±")
                            .font(FontId::monospace(11.0))
                            .color(p.text_muted),
                    );
                    let (rj, cj) = number_field(
                        ui,
                        p,
                        "jitter_pct",
                        &mut self.jitter_buf,
                        54.0,
                        allow_float,
                        editable,
                        self.human_err,
                    );
                    if rj.has_focus() {
                        self.any_field_focused = true;
                    }
                    if cj {
                        self.commit_humanizer();
                    }
                    ui.label(
                        RichText::new("% σ")
                            .font(FontId::proportional(11.5))
                            .color(p.text_muted),
                    );
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new("Spread ±")
                            .font(FontId::proportional(11.5))
                            .color(p.text_muted),
                    );
                    let (rp, cp) = number_field(
                        ui,
                        p,
                        "pos_jitter",
                        &mut self.pos_jitter_buf,
                        48.0,
                        allow_uint,
                        editable,
                        self.human_err,
                    );
                    if rp.has_focus() {
                        self.any_field_focused = true;
                    }
                    if cp {
                        self.commit_humanizer();
                    }
                    ui.label(
                        RichText::new("px")
                            .font(FontId::proportional(11.5))
                            .color(p.text_muted),
                    );
                });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 5.0;
                    row_label(ui, p, "Hold Duty", editable);
                    let (r1, c1) = number_field(
                        ui,
                        p,
                        "hold_min",
                        &mut self.hold_min_buf,
                        56.0,
                        allow_float,
                        editable,
                        self.human_err,
                    );
                    if r1.has_focus() {
                        self.any_field_focused = true;
                    }
                    if c1 {
                        self.commit_humanizer();
                    }
                    ui.label(
                        RichText::new("to")
                            .font(FontId::proportional(11.5))
                            .color(p.text_muted),
                    );
                    let (r2, c2) = number_field(
                        ui,
                        p,
                        "hold_max",
                        &mut self.hold_max_buf,
                        56.0,
                        allow_float,
                        editable,
                        self.human_err,
                    );
                    if r2.has_focus() {
                        self.any_field_focused = true;
                    }
                    if c2 {
                        self.commit_humanizer();
                    }
                    ui.label(
                        RichText::new("ms (Gaussian timing)")
                            .font(FontId::proportional(11.0))
                            .color(p.text_faint),
                    );
                });
            }
        });
    }

    fn render_footer(&mut self, ui: &mut Ui, p: &Palette) {
        let active = self.engine.is_active();
        let editable = !active;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.allocate_ui_with_layout(
                Vec2::new(52.0, CONTROL_H),
                egui::Layout::left_to_right(Align::Center),
                |ui| {
                    ui.label(
                        RichText::new("Trigger")
                            .font(FontId::proportional(12.0))
                            .color(if editable { p.text_muted } else { p.text_faint }),
                    );
                },
            );
            let mut trig = if self.config.trigger_mode == "hold" {
                TriggerMode::Hold
            } else {
                TriggerMode::Toggle
            };
            if segmented_control(
                ui,
                p,
                "trigger_mode_seg",
                &mut trig,
                &[(TriggerMode::Toggle, "Toggle"), (TriggerMode::Hold, "Hold")],
                editable,
                132.0,
            ) {
                self.config.trigger_mode = match trig {
                    TriggerMode::Toggle => "toggle",
                    TriggerMode::Hold => "hold",
                }
                .to_string();
                if !self.preview_mode {
                    let _ = self.config.save();
                }
            }

            ui.add_space(4.0);
            ui.label(
                RichText::new("Hotkey")
                    .font(FontId::proportional(12.0))
                    .color(if editable { p.text_muted } else { p.text_faint }),
            );

            let hk_label = if self.recording_hotkey {
                "Press key / M4-5...".to_string()
            } else {
                hotkey_display_name(&self.config.hotkey)
            };
            let btn_w = ui.available_width().max(80.0);
            let hk_resp = keycap_button(ui, p, &hk_label, self.recording_hotkey, editable, btn_w);
            if hk_resp
                .on_hover_text("Click to bind a keyboard key or Mouse4 / Mouse5")
                .clicked()
                && editable
            {
                self.recording_hotkey = !self.recording_hotkey;
                self.hotkeys.set_recording(self.recording_hotkey);
            }
        });

        ui.add_space(8.0);
        let hk_name = hotkey_display_name(&self.config.hotkey);
        let cta_text = if active {
            format!("STOP ({hk_name})")
        } else {
            format!("START ({hk_name})")
        };
        if accent_button(
            ui,
            p,
            &cta_text,
            Vec2::new(ui.available_width(), 42.0),
            active,
            14.0,
            10,
        )
        .clicked()
        {
            self.toggle_clicking();
        }

        ui.add_space(4.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(
                    "Panic stop: Esc or move pointer to top-left corner (0, 0)   •   Supports Mouse4 / Mouse5",
                )
                .font(FontId::proportional(10.5))
                .color(p.text_faint),
            );
        });
    }

    fn render_modal(&mut self, ctx: &egui::Context, p: &Palette) {
        let Some(modal_kind) = self.modal.take() else {
            return;
        };

        let mut next_modal: Option<ModalDialog> = None;
        let mut pending_setup_run: Option<(bool, bool, bool)> = None;
        let mut pending_update_start = false;
        let mut pending_skip_ver: Option<String> = None;

        egui::Area::new(egui::Id::new("modal_backdrop"))
            .fixed_pos(egui::Pos2::ZERO)
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                let rect = ctx.screen_rect();
                ui.painter().rect_filled(
                    rect,
                    0.0,
                    Color32::from_black_alpha(if p.is_dark { 165 } else { 90 }),
                );
                ui.allocate_rect(rect, egui::Sense::click());
            });

        let dialog_title = match &modal_kind {
            ModalDialog::UpdatePrompt { .. } => "ClickForge Update",
            ModalDialog::SetupPrompt { .. } => "ClickForge Setup",
        };

        egui::Window::new(dialog_title)
            .order(egui::Order::Foreground)
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .fixed_size(Vec2::new(WINDOW_WIDTH - 48.0, 0.0))
            .frame(
                Frame::new()
                    .fill(p.surface)
                    .stroke(Stroke::new(1.0_f32, p.border_strong))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(Margin::same(18)),
            )
            .show(ctx, |ui| match modal_kind {
                ModalDialog::UpdatePrompt { release } => {
                    let ver = release.version_text();
                    let (can_self, _) = updater::can_self_update();
                    let primary_lbl = if can_self {
                        "Update Now"
                    } else {
                        "Download Update"
                    };
                    let question = if can_self {
                        "Would you like to download and install this update now?"
                    } else {
                        "Would you like to open the download page for this update?"
                    };

                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 12.0;
                        if let Some(tex) = &self.logo_texture {
                            ui.image((tex.id(), Vec2::splat(38.0)));
                        }
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(format!("Update Available: v{ver}"))
                                    .font(FontId::proportional(15.5))
                                    .strong()
                                    .color(p.text),
                            );
                            ui.label(
                                RichText::new(format!("You are currently using v{VERSION}.\n{question}"))
                                    .font(FontId::proportional(12.0))
                                    .color(p.text_muted),
                            );
                        });
                    });

                    ui.add_space(12.0);
                    section_title(ui, p, "What's New");
                    ui.add_space(4.0);
                    Frame::new()
                        .fill(p.field)
                        .stroke(Stroke::new(1.0_f32, p.border))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::same(10))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ScrollArea::vertical().max_height(125.0).show(ui, |ui| {
                                ui.label(
                                    RichText::new(updater::format_release_notes(&release.notes))
                                        .font(FontId::proportional(11.5))
                                        .color(p.text_muted),
                                );
                            });
                        });

                    ui.add_space(14.0);
                    let mut keep = true;
                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Skip Version")
                                        .font(FontId::proportional(11.5))
                                        .color(p.text_faint),
                                )
                                .frame(false),
                            )
                            .clicked()
                        {
                            pending_skip_ver = Some(ver.clone());
                            keep = false;
                        }
                        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            if accent_button(
                                ui,
                                p,
                                primary_lbl,
                                Vec2::new(124.0, 32.0),
                                false,
                                12.0,
                                7,
                            )
                            .clicked()
                            {
                                pending_update_start = true;
                                keep = false;
                            }
                            if ghost_button(ui, p, "Not Now", Vec2::new(82.0, 32.0), true).clicked()
                            {
                                keep = false;
                            }
                        });
                    });
                    if keep {
                        next_modal = Some(ModalDialog::UpdatePrompt { release });
                    }
                }
                ModalDialog::SetupPrompt {
                    mut create_desktop_shortcut,
                    has_local,
                    portable_ok,
                } => {
                    let (subhead, action_lbl, detail) = if has_local {
                        (
                            "Run the included ClickForge Setup wizard to install into Windows Programs.",
                            "Run Setup Wizard",
                            "Installs per-user to %LOCALAPPDATA%\\Programs\\ClickForge (no admin prompt required).",
                        )
                    } else if portable_ok {
                        (
                            "Install this portable ClickForge build into your Windows Programs folder.",
                            "Install to Programs",
                            "Adds ClickForge to Start Menu and Installed Apps with automatic updates enabled.",
                        )
                    } else {
                        (
                            "Download and run the official ClickForge-Setup.exe installer to add ClickForge to Windows Programs.",
                            "Download Setup",
                            "Installs per-user to %LOCALAPPDATA%\\Programs\\ClickForge (no admin prompt required).",
                        )
                    };

                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 12.0;
                        if let Some(tex) = &self.logo_texture {
                            ui.image((tex.id(), Vec2::splat(38.0)));
                        }
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new("Install ClickForge for Windows")
                                    .font(FontId::proportional(15.5))
                                    .strong()
                                    .color(p.text),
                            );
                            ui.label(
                                RichText::new(subhead)
                                    .font(FontId::proportional(12.0))
                                    .color(p.text_muted),
                            );
                        });
                    });

                    ui.add_space(10.0);
                    ui.label(
                        RichText::new(detail)
                            .font(FontId::proportional(11.5))
                            .color(p.text_faint),
                    );

                    if portable_ok && !has_local {
                        ui.add_space(8.0);
                        ui.checkbox(&mut create_desktop_shortcut, "Create Desktop shortcut");
                    }

                    ui.add_space(14.0);
                    let mut keep = true;
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        if accent_button(
                            ui,
                            p,
                            action_lbl,
                            Vec2::new(136.0, 32.0),
                            false,
                            12.0,
                            7,
                        )
                        .clicked()
                        {
                            pending_setup_run =
                                Some((create_desktop_shortcut, has_local, portable_ok));
                            keep = false;
                        }
                        if ghost_button(ui, p, "Cancel", Vec2::new(82.0, 32.0), true).clicked() {
                            keep = false;
                        }
                    });
                    if keep {
                        next_modal = Some(ModalDialog::SetupPrompt {
                            create_desktop_shortcut,
                            has_local,
                            portable_ok,
                        });
                    }
                }
            });

        self.modal = next_modal;
        if let Some(ver) = pending_skip_ver {
            self.config.skipped_version = ver;
            if !self.preview_mode {
                let _ = self.config.save();
            }
        }
        if pending_update_start {
            self.start_update_install();
        }
        if let Some((desk, local, portable)) = pending_setup_run {
            self.run_windows_setup(desk, local, portable);
        }
    }
}

impl eframe::App for ClickForgeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_events(ctx);

        // Coordinate picker fullscreen viewport
        match self.picker.show(ctx, &self.backend) {
            PickerOutcome::Picked(x, y) => {
                self.config.settings.position_mode = "fixed".to_string();
                self.config.settings.fixed_x = x.clamp(-32768, 32767);
                self.config.settings.fixed_y = y.clamp(-32768, 32767);
                self.x_buf = self.config.settings.fixed_x.to_string();
                self.y_buf = self.config.settings.fixed_y.to_string();
                self.mark_custom();
            }
            PickerOutcome::Cancelled | PickerOutcome::Pending => {}
        }

        // Sync palette with Light/Dark/System theme
        let os_dark = system_prefers_dark(ctx);
        let theme_changed = self.config.theme != self.last_theme_mode
            || (self.config.theme == "System" && self.last_os_dark != Some(os_dark));
        let p = Palette::for_mode(&self.config.theme, ctx);
        if theme_changed {
            self.last_theme_mode = self.config.theme.clone();
            self.last_os_dark = Some(os_dark);
            apply_visuals(ctx, p);
            ctx.send_viewport_cmd(ViewportCommand::SetTheme(if p.is_dark {
                SystemTheme::Dark
            } else {
                SystemTheme::Light
            }));
        }

        self.any_field_focused = false;
        let mut measured_h = 0.0_f32;

        egui::CentralPanel::default()
            .frame(
                Frame::new()
                    .fill(p.bg)
                    .inner_margin(Margin::symmetric(14, 11)),
            )
            .show(ctx, |ui| {
                let inner = ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    self.render_header(ui, p, ctx);
                    ui.add_space(9.0);
                    self.render_banner(ui, p, ctx);
                    self.render_telemetry_card(ui, p);
                    ui.add_space(9.0);
                    self.render_speed_card(ui, p);
                    ui.add_space(9.0);
                    self.render_target_card(ui, p);
                    ui.add_space(9.0);
                    self.render_humanizer_card(ui, p);
                    ui.add_space(10.0);
                    self.render_footer(ui, p);
                });
                measured_h = (inner.response.rect.height() + 22.0).ceil();
            });

        // Sync hotkey suspension when typing inside numeric inputs or picking coords
        let conflict = self.any_field_focused && conflicts_with_numeric_entry(&self.config.hotkey);
        self.hotkeys
            .set_suspended(conflict || self.recording_hotkey || self.picker.is_active());

        // Modal dialog overlay if any
        self.render_modal(ctx, p);

        // Adapt window height cleanly to collapsed/expanded sections
        let min_h = if self.modal.is_some() { 380.0 } else { 490.0 };
        let target_h = measured_h.clamp(min_h, 880.0);
        if (target_h - self.last_target_h).abs() >= 1.5 {
            self.last_target_h = target_h;
            ctx.send_viewport_cmd(ViewportCommand::InnerSize(Vec2::new(WINDOW_WIDTH, target_h)));
        }

        // Low idle CPU when settled; smooth updates when running or animating
        let busy = self.engine.is_active()
            || self.recording_hotkey
            || self.picker.is_active()
            || self.update_busy
            || self.setup_busy
            || self.startup_check_at.is_some()
            || self.banner.as_ref().and_then(|b| b.auto_hide_at).is_some()
            || [self.speed_err, self.repeat_err, self.coords_err, self.human_err]
                .into_iter()
                .flatten()
                .any(|t| Instant::now() < t);

        if busy || self.screenshot_out.is_some() {
            ctx.request_repaint_after(Duration::from_millis(33));
        } else {
            ctx.request_repaint_after(Duration::from_millis(350));
        }

        if let Some(out_path) = self.screenshot_out.clone() {
            let captured = ctx.input(|i| {
                i.events.iter().find_map(|ev| match ev {
                    egui::Event::Screenshot { image, .. } => Some(Arc::clone(image)),
                    _ => None,
                })
            });
            if let Some(img) = captured {
                let raw: Vec<u8> = img
                    .pixels
                    .iter()
                    .flat_map(|c| [c.r(), c.g(), c.b(), c.a()])
                    .collect();
                let _ = image::save_buffer(
                    &out_path,
                    &raw,
                    img.size[0] as u32,
                    img.size[1] as u32,
                    image::ColorType::Rgba8,
                );
                self.screenshot_out = None;
                ctx.send_viewport_cmd(ViewportCommand::Close);
            } else {
                self.screenshot_frames += 1;
                if self.screenshot_frames >= 6 && !self.screenshot_requested {
                    self.screenshot_requested = true;
                    ctx.send_viewport_cmd(ViewportCommand::Screenshot(Default::default()));
                }
            }
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.stop_clicking(true);
        self.hotkeys.stop();
        if !self.preview_mode {
            let _ = self.config.save();
        }
    }
}

const fn row_label_w_pad() -> f32 {
    crate::ui::widgets::ROW_LABEL_W + 6.0
}

fn decode_logo_texture(ctx: &egui::Context) -> Option<TextureHandle> {
    let img = image::load_from_memory(ICON_PNG).ok()?.into_rgba8();
    let (w, h) = img.dimensions();
    let color_image = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &img);
    Some(ctx.load_texture("clickforge_logo", color_image, egui::TextureOptions::LINEAR))
}
