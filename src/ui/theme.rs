use std::path::Path;
use std::sync::Arc;

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, Stroke, Visuals,
};

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub is_dark: bool,
    pub bg: Color32,
    pub surface: Color32,
    pub field: Color32,
    pub field_disabled: Color32,
    pub border: Color32,
    pub border_strong: Color32,
    pub hover: Color32,
    pub seg_selected: Color32,
    pub seg_selected_hover: Color32,
    pub switch_track: Color32,
    pub switch_thumb: Color32,
    pub text: Color32,
    pub text_muted: Color32,
    pub text_faint: Color32,
    pub text_on_accent: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub accent_soft: Color32,
    pub danger: Color32,
    pub danger_hover: Color32,
    pub success: Color32,
    pub success_soft: Color32,
    pub warning: Color32,
}

impl Palette {
    pub const DARK: Self = Self {
        is_dark: true,
        bg: Color32::from_rgb(0x0A, 0x0B, 0x0D),
        surface: Color32::from_rgb(0x12, 0x14, 0x18),
        field: Color32::from_rgb(0x17, 0x1A, 0x20),
        field_disabled: Color32::from_rgb(0x0F, 0x11, 0x15),
        border: Color32::from_rgb(0x22, 0x26, 0x2E),
        border_strong: Color32::from_rgb(0x30, 0x36, 0x42),
        hover: Color32::from_rgb(0x1D, 0x20, 0x26),
        seg_selected: Color32::from_rgb(0x26, 0x2B, 0x35),
        seg_selected_hover: Color32::from_rgb(0x2E, 0x34, 0x40),
        switch_track: Color32::from_rgb(0x2B, 0x31, 0x3C),
        switch_thumb: Color32::from_rgb(0xF3, 0xF4, 0xF6),
        text: Color32::from_rgb(0xEC, 0xED, 0xEF),
        text_muted: Color32::from_rgb(0x94, 0x9B, 0xA6),
        text_faint: Color32::from_rgb(0x5A, 0x60, 0x6B),
        text_on_accent: Color32::WHITE,
        accent: Color32::from_rgb(0x10, 0xA3, 0x7F),
        accent_hover: Color32::from_rgb(0x0E, 0x8F, 0x6F),
        accent_soft: Color32::from_rgb(0x14, 0x29, 0x23),
        danger: Color32::from_rgb(0xE5, 0x48, 0x4D),
        danger_hover: Color32::from_rgb(0xF0, 0x6A, 0x6E),
        success: Color32::from_rgb(0x10, 0xB9, 0x81),
        success_soft: Color32::from_rgb(0x0D, 0x24, 0x1C),
        warning: Color32::from_rgb(0xF5, 0xA5, 0x24),
    };

    pub const LIGHT: Self = Self {
        is_dark: false,
        bg: Color32::from_rgb(0xF3, 0xF4, 0xF6),
        surface: Color32::from_rgb(0xFF, 0xFF, 0xFF),
        field: Color32::from_rgb(0xEB, 0xED, 0xF0),
        field_disabled: Color32::from_rgb(0xF4, 0xF5, 0xF7),
        border: Color32::from_rgb(0xE2, 0xE5, 0xEA),
        border_strong: Color32::from_rgb(0xC8, 0xCD, 0xD6),
        hover: Color32::from_rgb(0xE4, 0xE7, 0xEC),
        seg_selected: Color32::from_rgb(0xFF, 0xFF, 0xFF),
        seg_selected_hover: Color32::from_rgb(0xF9, 0xFA, 0xFB),
        switch_track: Color32::from_rgb(0xC2, 0xC8, 0xD1),
        switch_thumb: Color32::from_rgb(0xFF, 0xFF, 0xFF),
        text: Color32::from_rgb(0x11, 0x13, 0x18),
        text_muted: Color32::from_rgb(0x52, 0x58, 0x66),
        text_faint: Color32::from_rgb(0x8E, 0x95, 0xA2),
        text_on_accent: Color32::WHITE,
        accent: Color32::from_rgb(0x05, 0x96, 0x69),
        accent_hover: Color32::from_rgb(0x04, 0x78, 0x57),
        accent_soft: Color32::from_rgb(0xD1, 0xFA, 0xE5),
        danger: Color32::from_rgb(0xDC, 0x26, 0x26),
        danger_hover: Color32::from_rgb(0xB9, 0x1C, 0x1C),
        success: Color32::from_rgb(0x05, 0x96, 0x69),
        success_soft: Color32::from_rgb(0xEC, 0xFD, 0xF5),
        warning: Color32::from_rgb(0xD9, 0x77, 0x06),
    };

    pub fn for_mode(mode: &str, ctx: &egui::Context) -> &'static Self {
        match mode {
            "Dark" => &Self::DARK,
            "Light" => &Self::LIGHT,
            _ => {
                if system_prefers_dark(ctx) {
                    &Self::DARK
                } else {
                    &Self::LIGHT
                }
            }
        }
    }
}

pub fn system_prefers_dark(ctx: &egui::Context) -> bool {
    #[cfg(target_os = "windows")]
    {
        if let Some(dark) = windows_prefers_dark() {
            return dark;
        }
    }
    ctx.style().visuals.dark_mode
}

#[cfg(target_os = "windows")]
fn windows_prefers_dark() -> Option<bool> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::sync::{Mutex, OnceLock};
    use std::time::{Duration, Instant};
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CURRENT_USER, KEY_READ, REG_DWORD,
    };

    type ThemeCache = Mutex<Option<(Instant, Option<bool>)>>;
    static CACHE: OnceLock<ThemeCache> = OnceLock::new();
    let mutex = CACHE.get_or_init(|| Mutex::new(None));
    if let Ok(guard) = mutex.lock() {
        if let Some((at, cached)) = *guard {
            if at.elapsed() < Duration::from_millis(1500) {
                return cached;
            }
        }
    }

    let subkey: Vec<u16> =
        OsStr::new(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
    let val_name: Vec<u16> = OsStr::new("AppsUseLightTheme")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let result = unsafe {
        let mut hkey: HKEY = std::ptr::null_mut();
        if RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_READ, &mut hkey) != 0 {
            None
        } else {
            let mut ty: u32 = 0;
            let mut data: u32 = 1;
            let mut size: u32 = 4;
            let rc = RegQueryValueExW(
                hkey,
                val_name.as_ptr(),
                std::ptr::null(),
                &mut ty,
                (&mut data as *mut u32) as *mut u8,
                &mut size,
            );
            RegCloseKey(hkey);
            if rc == 0 && ty == REG_DWORD {
                Some(data == 0)
            } else {
                None
            }
        }
    };

    if let Ok(mut guard) = mutex.lock() {
        *guard = Some((Instant::now(), result));
    }
    result
}

pub fn install_native_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    let ui_candidates: &[&str] = if cfg!(target_os = "windows") {
        &[
            r"C:\Windows\Fonts\segoeui.ttf",
            r"C:\Windows\Fonts\tahoma.ttf",
        ]
    } else if cfg!(target_os = "macos") {
        &[
            "/Library/Fonts/SF-Pro-Text-Regular.otf",
            "/System/Library/Fonts/SFNS.ttf",
            "/System/Library/Fonts/Supplemental/Arial.ttf",
        ]
    } else {
        &[
            "/usr/share/fonts/truetype/inter/Inter-Regular.ttf",
            "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        ]
    };

    let mono_candidates: &[&str] = if cfg!(target_os = "windows") {
        &[
            r"C:\Windows\Fonts\CascadiaMono.ttf",
            r"C:\Windows\Fonts\consola.ttf",
        ]
    } else if cfg!(target_os = "macos") {
        &[
            "/System/Library/Fonts/SFNSMono.ttf",
            "/Library/Fonts/SF-Mono-Regular.otf",
            "/System/Library/Fonts/Menlo.ttc",
            "/System/Library/Fonts/Monaco.ttf",
        ]
    } else {
        &[
            "/usr/share/fonts/truetype/jetbrains-mono/JetBrainsMono-Regular.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        ]
    };

    if let Some(bytes) = first_existing_font(ui_candidates) {
        fonts
            .font_data
            .insert("klick_ui".to_owned(), Arc::new(FontData::from_owned(bytes)));
        if let Some(fam) = fonts.families.get_mut(&FontFamily::Proportional) {
            fam.insert(0, "klick_ui".to_owned());
        }
    }

    if let Some(bytes) = first_existing_font(mono_candidates) {
        fonts
            .font_data
            .insert("klick_mono".to_owned(), Arc::new(FontData::from_owned(bytes)));
        if let Some(fam) = fonts.families.get_mut(&FontFamily::Monospace) {
            fam.insert(0, "klick_mono".to_owned());
        }
    }

    ctx.set_fonts(fonts);
}

fn first_existing_font(paths: &[&str]) -> Option<Vec<u8>> {
    for p in paths {
        if Path::new(p).is_file() {
            if let Ok(bytes) = std::fs::read(p) {
                return Some(bytes);
            }
        }
    }
    None
}

pub fn apply_visuals(ctx: &egui::Context, p: &Palette) {
    let mut visuals = if p.is_dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };

    visuals.panel_fill = p.bg;
    visuals.window_fill = p.surface;
    visuals.extreme_bg_color = p.field;
    visuals.faint_bg_color = p.field;
    visuals.override_text_color = Some(p.text);
    visuals.window_stroke = Stroke::new(1.0_f32, p.border_strong);
    visuals.window_corner_radius = CornerRadius::same(10);
    visuals.popup_shadow = egui::epaint::Shadow {
        offset: [0, 4],
        blur: 10,
        spread: 0,
        color: Color32::from_black_alpha(if p.is_dark { 110 } else { 36 }),
    };
    visuals.window_shadow = visuals.popup_shadow;

    visuals.selection.bg_fill = p.accent_soft;
    visuals.selection.stroke = Stroke::new(1.0_f32, p.accent);

    visuals.widgets.noninteractive.bg_fill = p.surface;
    visuals.widgets.noninteractive.weak_bg_fill = p.field;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.border);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, p.text_muted);

    visuals.widgets.inactive.bg_fill = p.field;
    visuals.widgets.inactive.weak_bg_fill = p.field;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, p.border_strong);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, p.text);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(7);

    visuals.widgets.hovered.bg_fill = p.hover;
    visuals.widgets.hovered.weak_bg_fill = p.hover;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, p.border_strong);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, p.text);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(7);

    visuals.widgets.active.bg_fill = p.seg_selected;
    visuals.widgets.active.weak_bg_fill = p.seg_selected;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, p.accent);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, p.text);
    visuals.widgets.active.corner_radius = CornerRadius::same(7);

    ctx.set_visuals(visuals);
}
