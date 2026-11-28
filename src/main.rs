#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod core;
mod meta;
mod ui;

use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui::{self, Vec2, ViewportBuilder};

use crate::core::clicker::{ClickEngine, ClickSettings};
use crate::core::config::AppConfig;
use crate::core::hotkeys::normalize_binding;
use crate::core::input_backend::DummyBackend;
use crate::core::system::cleanup_legacy_migration_marker;
use crate::core::updater::parse_version;
use crate::meta::{APP_NAME, CODENAME, ICON_PNG, VERSION};
use crate::ui::app::{KlickApp, StartupOptions};
use crate::ui::widgets::WINDOW_WIDTH;

fn run_smoke_test() -> i32 {
    let backend = Arc::new(DummyBackend::new((240, 240)));
    let engine = Arc::new(ClickEngine::with_backend(backend, None));
    let ok = engine.start(ClickSettings {
        interval_ms: 2.0,
        repeat_count: 3,
        corner_failsafe: false,
        ..Default::default()
    });
    if !ok {
        return 1;
    }
    std::thread::sleep(std::time::Duration::from_millis(40));
    engine.stop(true);
    let snap = engine.snapshot();
    if snap.total == 0 {
        return 1;
    }
    if normalize_binding("VK117", "f6") != "f6" {
        return 1;
    }
    if parse_version(&format!("v{VERSION}")).is_none() {
        return 1;
    }
    if image::load_from_memory(ICON_PNG).is_err() {
        return 1;
    }
    let _cfg = AppConfig::default();
    let _ = writeln!(
        std::io::stdout(),
        "{APP_NAME} v{VERSION} ({CODENAME}) smoke-test OK"
    );
    0
}

fn load_window_icon() -> Option<egui::IconData> {
    let img = image::load_from_memory(ICON_PNG).ok()?.into_rgba8();
    let (width, height) = img.dimensions();
    Some(egui::IconData {
        rgba: img.into_raw(),
        width,
        height,
    })
}

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--version" || a == "-V") {
        let _ = writeln!(std::io::stdout(), "{APP_NAME} v{VERSION} ({CODENAME})");
        return Ok(());
    }
    if args.iter().any(|a| a == "--smoke-test") {
        let code = run_smoke_test();
        std::process::exit(code);
    }

    cleanup_legacy_migration_marker();

    let mut preview_theme: Option<String> = None;
    let mut preview_dialog: Option<String> = None;
    let mut screenshot_out: Option<PathBuf> = None;
    let mut idx = 1;
    while idx < args.len() {
        match args[idx].as_str() {
            "--preview-theme" if idx + 1 < args.len() => {
                preview_theme = Some(args[idx + 1].clone());
                idx += 2;
            }
            "--preview-dialog" if idx + 1 < args.len() => {
                preview_dialog = Some(args[idx + 1].clone());
                idx += 2;
            }
            "--screenshot-out" if idx + 1 < args.len() => {
                screenshot_out = Some(PathBuf::from(&args[idx + 1]));
                idx += 2;
            }
            _ => idx += 1,
        }
    }

    let mut viewport = ViewportBuilder::default()
        .with_title(APP_NAME)
        .with_inner_size(Vec2::new(WINDOW_WIDTH, 560.0))
        .with_min_inner_size(Vec2::new(WINDOW_WIDTH, 440.0))
        .with_resizable(true);

    if let Some(icon) = load_window_icon() {
        viewport = viewport.with_icon(Arc::new(icon));
    }

    let options = eframe::NativeOptions {
        viewport,
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        APP_NAME,
        options,
        Box::new(move |cc| {
            Ok(Box::new(KlickApp::new(
                cc,
                StartupOptions {
                    preview_theme,
                    preview_dialog,
                    screenshot_out,
                },
            )))
        }),
    )
}
