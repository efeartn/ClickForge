use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::core::hotkeys::normalize_binding;
use crate::core::validation::{clamp_f64, to_interval_ms};
use crate::meta::{APP_NAME, LEGACY_APP_NAME};

pub const SCHEMA_VERSION: u32 = 5;
pub const PROFILE_NAMES: [&str; 4] = ["Default", "Fast", "Humanized", "Custom"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProfileSettings {
    pub speed_value: f64,
    pub speed_unit: String,   // ms | sec | min | cps
    pub repeat_mode: String,  // infinite | fixed
    pub repeat_count: u64,
    pub button: String,       // left | right | middle
    pub click_type: String,   // single | double
    pub position_mode: String,// cursor | fixed
    pub fixed_x: i32,
    pub fixed_y: i32,
    pub humanize: bool,
    pub jitter_pct: f64,
    pub hold_min_ms: f64,
    pub hold_max_ms: f64,
    pub pos_jitter_px: i32,
}

impl Default for ProfileSettings {
    fn default() -> Self {
        Self {
            speed_value: 10.0,
            speed_unit: "ms".to_string(),
            repeat_mode: "infinite".to_string(),
            repeat_count: 10,
            button: "left".to_string(),
            click_type: "single".to_string(),
            position_mode: "cursor".to_string(),
            fixed_x: 0,
            fixed_y: 0,
            humanize: false,
            jitter_pct: 15.0,
            hold_min_ms: 18.0,
            hold_max_ms: 42.0,
            pos_jitter_px: 2,
        }
    }
}

impl ProfileSettings {
    pub fn sanitize(&mut self) {
        if !self.speed_value.is_finite() || self.speed_value <= 0.0 {
            self.speed_value = 10.0;
        }
        if !matches!(self.speed_unit.as_str(), "ms" | "sec" | "min" | "cps") {
            self.speed_unit = "ms".to_string();
        }
        if !matches!(self.repeat_mode.as_str(), "infinite" | "fixed") {
            self.repeat_mode = "infinite".to_string();
        }
        self.repeat_count = self.repeat_count.clamp(1, 10_000_000);
        if !matches!(self.button.as_str(), "left" | "right" | "middle") {
            self.button = "left".to_string();
        }
        if !matches!(self.click_type.as_str(), "single" | "double") {
            self.click_type = "single".to_string();
        }
        if !matches!(self.position_mode.as_str(), "cursor" | "fixed") {
            self.position_mode = "cursor".to_string();
        }
        self.fixed_x = self.fixed_x.clamp(-32768, 32767);
        self.fixed_y = self.fixed_y.clamp(-32768, 32767);
        self.jitter_pct = if self.jitter_pct.is_finite() {
            clamp_f64(self.jitter_pct, 0.0, 80.0)
        } else {
            15.0
        };
        self.hold_min_ms = if self.hold_min_ms.is_finite() {
            clamp_f64(self.hold_min_ms, 1.0, 500.0)
        } else {
            18.0
        };
        self.hold_max_ms = if self.hold_max_ms.is_finite() {
            clamp_f64(self.hold_max_ms, 1.0, 500.0)
        } else {
            42.0
        };
        if self.hold_max_ms < self.hold_min_ms {
            self.hold_max_ms = self.hold_min_ms;
        }
        self.pos_jitter_px = self.pos_jitter_px.clamp(0, 50);
    }

    pub fn from_json_lenient(val: &Value) -> Self {
        let mut out = Self::default();
        let Some(obj) = val.as_object() else {
            return out;
        };
        if let Some(v) = obj.get("speed_value").and_then(Value::as_f64) {
            out.speed_value = v;
        }
        if let Some(v) = obj.get("speed_unit").and_then(Value::as_str) {
            out.speed_unit = v.to_string();
        }
        if let Some(v) = obj.get("repeat_mode").and_then(Value::as_str) {
            out.repeat_mode = v.to_string();
        }
        if let Some(v) = obj.get("repeat_count").and_then(Value::as_u64) {
            out.repeat_count = v;
        } else if let Some(v) = obj.get("repeat_count").and_then(Value::as_i64) {
            out.repeat_count = v.max(1) as u64;
        }
        if let Some(v) = obj.get("button").and_then(Value::as_str) {
            out.button = v.to_string();
        }
        if let Some(v) = obj.get("click_type").and_then(Value::as_str) {
            out.click_type = v.to_string();
        }
        if let Some(v) = obj.get("position_mode").and_then(Value::as_str) {
            out.position_mode = v.to_string();
        }
        if let Some(v) = obj.get("fixed_x").and_then(Value::as_i64) {
            out.fixed_x = v.clamp(-32768, 32767) as i32;
        }
        if let Some(v) = obj.get("fixed_y").and_then(Value::as_i64) {
            out.fixed_y = v.clamp(-32768, 32767) as i32;
        }
        if let Some(v) = obj.get("humanize").and_then(Value::as_bool) {
            out.humanize = v;
        }
        if let Some(v) = obj.get("jitter_pct").and_then(Value::as_f64) {
            out.jitter_pct = v;
        }
        if let Some(v) = obj.get("hold_min_ms").and_then(Value::as_f64) {
            out.hold_min_ms = v;
        }
        if let Some(v) = obj.get("hold_max_ms").and_then(Value::as_f64) {
            out.hold_max_ms = v;
        }
        if let Some(v) = obj.get("pos_jitter_px").and_then(Value::as_i64) {
            out.pos_jitter_px = v.clamp(0, 50) as i32;
        }
        out.sanitize();
        out
    }

    pub fn interval_ms(&self) -> f64 {
        to_interval_ms(self.speed_value, &self.speed_unit)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub schema: u32,
    pub profile: String,
    pub theme: String, // System | Light | Dark
    pub always_on_top: bool,
    pub sound_cue: bool,
    pub hotkey: String,
    pub trigger_mode: String, // toggle | hold
    pub auto_update_check: bool,
    pub last_update_check: f64,
    pub skipped_version: String,
    pub settings: ProfileSettings,
    pub custom: ProfileSettings,
    #[serde(skip)]
    pub path: Option<PathBuf>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            schema: SCHEMA_VERSION,
            profile: "Default".to_string(),
            theme: "System".to_string(),
            always_on_top: false,
            sound_cue: false,
            hotkey: "f6".to_string(),
            trigger_mode: "toggle".to_string(),
            auto_update_check: true,
            last_update_check: 0.0,
            skipped_version: String::new(),
            settings: ProfileSettings::default(),
            custom: ProfileSettings::default(),
            path: None,
        }
    }
}

impl AppConfig {
    fn config_base_dir() -> PathBuf {
        #[cfg(target_os = "windows")]
        let base = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("USERPROFILE").map(|p| PathBuf::from(p).join("AppData").join("Local"))
            })
            .unwrap_or_else(|| PathBuf::from("."));

        #[cfg(target_os = "macos")]
        let base = std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join("Library").join("Application Support"))
            .unwrap_or_else(|| PathBuf::from("."));

        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .unwrap_or_else(|| PathBuf::from("."));

        base
    }

    pub fn default_config_path() -> PathBuf {
        Self::config_base_dir().join(APP_NAME).join("config.json")
    }

    pub fn legacy_config_path() -> PathBuf {
        Self::config_base_dir()
            .join(LEGACY_APP_NAME)
            .join("config.json")
    }

    pub fn load_default() -> Self {
        Self::load_default_from_paths(Self::default_config_path(), Self::legacy_config_path())
    }

    pub fn load_default_from_paths(new_path: PathBuf, legacy_path: PathBuf) -> Self {
        if new_path.is_file() || !legacy_path.is_file() {
            return Self::load_from_path(new_path);
        }
        let mut cfg = Self::load_from_path(legacy_path);
        cfg.path = Some(new_path);
        let _ = cfg.save();
        cfg
    }

    pub fn load_from_path(path: PathBuf) -> Self {
        let mut cfg = match fs::read_to_string(&path) {
            Ok(raw) => match serde_json::from_str::<Value>(&raw) {
                Ok(val) => Self::from_json_lenient(&val),
                Err(_) => Self::default(),
            },
            Err(_) => Self::default(),
        };
        cfg.path = Some(path);
        cfg
    }

    pub fn from_json_lenient(val: &Value) -> Self {
        let mut cfg = Self::default();
        let Some(obj) = val.as_object() else {
            return cfg;
        };

        if let Some(v) = obj.get("profile").and_then(Value::as_str) {
            if PROFILE_NAMES.contains(&v) {
                cfg.profile = v.to_string();
            }
        }
        if let Some(v) = obj.get("theme").and_then(Value::as_str) {
            if matches!(v, "System" | "Light" | "Dark") {
                cfg.theme = v.to_string();
            }
        }
        if let Some(v) = obj.get("always_on_top").and_then(Value::as_bool) {
            cfg.always_on_top = v;
        }
        if let Some(v) = obj.get("sound_cue").and_then(Value::as_bool) {
            cfg.sound_cue = v;
        }
        if let Some(v) = obj.get("hotkey").and_then(Value::as_str) {
            cfg.hotkey = normalize_binding(v, "f6");
        }
        if let Some(v) = obj.get("trigger_mode").and_then(Value::as_str) {
            if matches!(v, "toggle" | "hold") {
                cfg.trigger_mode = v.to_string();
            }
        }
        if let Some(v) = obj.get("auto_update_check").and_then(Value::as_bool) {
            cfg.auto_update_check = v;
        }
        if let Some(v) = obj.get("last_update_check").and_then(Value::as_f64) {
            if v.is_finite() {
                cfg.last_update_check = v;
            }
        }
        if let Some(v) = obj.get("skipped_version").and_then(Value::as_str) {
            cfg.skipped_version = v.to_string();
        }
        if let Some(settings_val) = obj.get("settings") {
            cfg.settings = ProfileSettings::from_json_lenient(settings_val);
        }
        if let Some(custom_val) = obj.get("custom") {
            cfg.custom = ProfileSettings::from_json_lenient(custom_val);
        }

        // Legacy schema migration (Python v3.2.0 Schema 5)
        let saved_schema = obj.get("schema").and_then(Value::as_u64).unwrap_or(0) as u32;
        if saved_schema < SCHEMA_VERSION {
            if cfg.profile == "Default"
                && cfg.settings.speed_unit == "ms"
                && (cfg.settings.speed_value - 100.0).abs() < 1e-6
            {
                cfg.settings.speed_value = 10.0;
            }
            if cfg.settings.position_mode == "fixed"
                && cfg.settings.fixed_x == 0
                && cfg.settings.fixed_y == 0
            {
                cfg.settings.position_mode = "cursor".to_string();
            }
            if cfg.custom.position_mode == "fixed"
                && cfg.custom.fixed_x == 0
                && cfg.custom.fixed_y == 0
            {
                cfg.custom.position_mode = "cursor".to_string();
            }
        }

        cfg.schema = SCHEMA_VERSION;
        cfg
    }

    pub fn save(&self) -> Result<(), String> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent).map_err(|e| format!("Failed creating config dir: {e}"))?;
        let json =
            serde_json::to_string_pretty(self).map_err(|e| format!("Serialize error: {e}"))?;
        let tmp = path.with_extension("json.tmp");
        if let Err(e) = fs::write(&tmp, json) {
            let _ = fs::remove_file(&tmp);
            return Err(format!("Failed writing temp config: {e}"));
        }
        if let Err(e) = fs::rename(&tmp, path) {
            let _ = fs::remove_file(&tmp);
            // Windows rename can fail if target exists; fall back to copy/overwrite
            fs::write(path, serde_json::to_string_pretty(self).unwrap_or_default())
                .map_err(|e2| format!("Failed saving config ({e}; {e2})"))?;
        }
        Ok(())
    }

    pub fn apply_preset(&mut self, name: &str) {
        if !PROFILE_NAMES.contains(&name) {
            return;
        }
        self.profile = name.to_string();
        match name {
            "Default" => {
                self.settings.speed_value = 10.0;
                self.settings.speed_unit = "ms".to_string();
                self.settings.repeat_mode = "infinite".to_string();
                self.settings.button = "left".to_string();
                self.settings.click_type = "single".to_string();
                self.settings.position_mode = "cursor".to_string();
                self.settings.humanize = false;
            }
            "Fast" => {
                self.settings.speed_value = 500.0;
                self.settings.speed_unit = "cps".to_string();
                self.settings.repeat_mode = "infinite".to_string();
                self.settings.button = "left".to_string();
                self.settings.click_type = "single".to_string();
                self.settings.humanize = false;
            }
            "Humanized" => {
                self.settings.speed_value = 12.0;
                self.settings.speed_unit = "cps".to_string();
                self.settings.repeat_mode = "infinite".to_string();
                self.settings.button = "left".to_string();
                self.settings.click_type = "single".to_string();
                self.settings.humanize = true;
                self.settings.jitter_pct = 15.0;
                self.settings.hold_min_ms = 18.0;
                self.settings.hold_max_ms = 42.0;
                self.settings.pos_jitter_px = 2;
            }
            "Custom" => {
                self.settings = self.custom.clone();
            }
            _ => {}
        }
        self.settings.sanitize();
    }

    #[allow(dead_code)]
    pub fn export_profile_json(&self) -> Result<String, String> {
        let mut root = Map::new();
        root.insert("schema".to_string(), Value::from(SCHEMA_VERSION));
        root.insert("profile".to_string(), Value::from(self.profile.clone()));
        root.insert(
            "settings".to_string(),
            serde_json::to_value(&self.settings).map_err(|e| e.to_string())?,
        );
        serde_json::to_string_pretty(&Value::Object(root)).map_err(|e| e.to_string())
    }

    #[allow(dead_code)]
    pub fn import_profile_json(&mut self, raw: &str) -> Result<(), String> {
        let val: Value = serde_json::from_str(raw).map_err(|e| format!("Invalid JSON: {e}"))?;
        let settings_val = val.get("settings").unwrap_or(&val);
        let imported = ProfileSettings::from_json_lenient(settings_val);
        self.profile = "Custom".to_string();
        self.settings = imported.clone();
        self.custom = imported;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corrupt_and_legacy_config_safely_migrated() {
        let legacy: Value = serde_json::json!({
            "schema": 4,
            "profile": "Default",
            "hotkey": "vk117",
            "settings": {
                "speed_value": 100.0,
                "speed_unit": "ms",
                "position_mode": "fixed",
                "fixed_x": 0,
                "fixed_y": 0,
                "repeat_count": 999999999
            }
        });
        let cfg = AppConfig::from_json_lenient(&legacy);
        assert_eq!(cfg.schema, SCHEMA_VERSION);
        assert_eq!(cfg.hotkey, "f6");
        assert_eq!(cfg.settings.speed_value, 10.0);
        assert_eq!(cfg.settings.position_mode, "cursor");
        assert_eq!(cfg.settings.repeat_count, 10_000_000);
    }

    #[test]
    fn presets_and_roundtrip_json() {
        let mut cfg = AppConfig::default();
        cfg.apply_preset("Fast");
        assert_eq!(cfg.settings.speed_unit, "cps");
        assert_eq!(cfg.settings.speed_value, 500.0);
        cfg.apply_preset("Humanized");
        assert!(cfg.settings.humanize);
        let exported = cfg.export_profile_json().unwrap();
        let mut cfg2 = AppConfig::default();
        cfg2.import_profile_json(&exported).unwrap();
        assert_eq!(cfg2.profile, "Custom");
        assert!(cfg2.settings.humanize);
    }

    #[test]
    fn load_from_legacy_migrates_to_new_path() {
        let root = std::env::temp_dir().join("klick_test_legacy_cfg_migration");
        let _ = fs::remove_dir_all(&root);
        let legacy_path = root.join("ClickForge").join("config.json");
        let new_path = root.join("Klick").join("config.json");

        let legacy = AppConfig {
            hotkey: "f8".to_string(),
            theme: "Light".to_string(),
            path: Some(legacy_path.clone()),
            ..Default::default()
        };
        legacy.save().unwrap();

        let migrated = AppConfig::load_default_from_paths(new_path.clone(), legacy_path);
        assert_eq!(migrated.hotkey, "f8");
        assert_eq!(migrated.theme, "Light");
        assert!(new_path.is_file());

        let _ = fs::remove_dir_all(&root);
    }
}
