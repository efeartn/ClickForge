use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{
    self, pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, FontId, Frame, Stroke, StrokeKind,
    ViewportBuilder, ViewportId,
};

use crate::core::input_backend::InputBackend;
use crate::core::system::virtual_screen_bounds;

pub enum PickerOutcome {
    Pending,
    Picked(i32, i32),
    Cancelled,
}

pub struct CoordinatePicker {
    active: bool,
    opened_at: Instant,
    bounds: (i32, i32, i32, i32),
}

impl CoordinatePicker {
    pub fn new() -> Self {
        Self {
            active: false,
            opened_at: Instant::now(),
            bounds: (0, 0, 1920, 1080),
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn open(&mut self) {
        self.bounds = virtual_screen_bounds().unwrap_or((0, 0, 1920, 1080));
        self.opened_at = Instant::now();
        self.active = true;
    }

    pub fn cancel(&mut self) {
        self.active = false;
    }

    pub fn show(&mut self, ctx: &egui::Context, backend: &Arc<dyn InputBackend>) -> PickerOutcome {
        if !self.active {
            return PickerOutcome::Pending;
        }

        let (left, top, width, height) = self.bounds;
        let ppp = ctx.pixels_per_point().max(1.0);
        #[cfg(target_os = "linux")]
        let wayland_fs = crate::core::wayland::linux_rt::is_wayland_session();
        #[cfg(not(target_os = "linux"))]
        let wayland_fs = false;

        let mut builder = ViewportBuilder::default()
            .with_title("Klick Coordinate Picker")
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_taskbar(false)
            .with_position(pos2(left as f32 / ppp, top as f32 / ppp))
            .with_inner_size(vec2(width as f32 / ppp, height as f32 / ppp));
        if wayland_fs {
            builder = builder.with_fullscreen(true);
        }

        let mut outcome = PickerOutcome::Pending;
        let opened_at = self.opened_at;

        ctx.show_viewport_immediate(ViewportId::from_hash_of("klick_picker"), builder, |vctx, _class| {
            vctx.set_cursor_icon(CursorIcon::Crosshair);
            vctx.request_repaint_after(Duration::from_millis(16));

            if vctx.input(|i| i.key_pressed(egui::Key::Escape) || i.viewport().close_requested()) {
                outcome = PickerOutcome::Cancelled;
                return;
            }

            let global_pos = backend.position().unwrap_or_else(|| {
                vctx.input(|i| i.pointer.latest_pos())
                    .map(|p| {
                        (
                            left + (p.x * ppp).round() as i32,
                            top + (p.y * ppp).round() as i32,
                        )
                    })
                    .unwrap_or((0, 0))
            });

            let ready = opened_at.elapsed() >= Duration::from_millis(140);
            if ready {
                if vctx.input(|i| i.pointer.secondary_clicked()) {
                    outcome = PickerOutcome::Cancelled;
                    return;
                }
                if vctx.input(|i| i.pointer.primary_clicked()) {
                    outcome = PickerOutcome::Picked(global_pos.0, global_pos.1);
                    return;
                }
            }

            egui::CentralPanel::default()
                .frame(Frame::NONE.fill(Color32::from_rgba_unmultiplied(10, 11, 13, 128)))
                .show(vctx, |ui| {
                    let screen = ui.max_rect();
                    let painter = ui.painter();
                    let local_cur = vctx.input(|i| i.pointer.hover_pos()).unwrap_or(screen.center());

                    let emerald = Color32::from_rgb(0x10, 0xA3, 0x7F);
                    let line_col = Color32::from_rgba_unmultiplied(0x10, 0xA3, 0x7F, 185);
                    painter.line_segment(
                        [pos2(screen.min.x, local_cur.y), pos2(screen.max.x, local_cur.y)],
                        Stroke::new(1.0_f32, line_col),
                    );
                    painter.line_segment(
                        [pos2(local_cur.x, screen.min.y), pos2(local_cur.x, screen.max.y)],
                        Stroke::new(1.0_f32, line_col),
                    );
                    painter.circle_stroke(local_cur, 12.0, Stroke::new(1.5_f32, emerald));
                    painter.circle_filled(local_cur, 2.5, Color32::WHITE);

                    // Top floating HUD pill
                    let hud_text = format!(
                        "X: {}   Y: {}   ·   Left-click to pick   ·   Esc / Right-click to cancel",
                        global_pos.0, global_pos.1
                    );
                    let pill_w = 500.0;
                    let pill_h = 34.0;
                    let pill_rect = egui::Rect::from_center_size(
                        pos2(screen.center().x, screen.min.y + 42.0),
                        vec2(pill_w, pill_h),
                    );
                    painter.rect(
                        pill_rect,
                        CornerRadius::same(10),
                        Color32::from_rgba_unmultiplied(0x12, 0x14, 0x18, 240),
                        Stroke::new(1.0_f32, Color32::from_rgb(0x30, 0x36, 0x42)),
                        StrokeKind::Inside,
                    );
                    painter.text(
                        pill_rect.center(),
                        Align2::CENTER_CENTER,
                        hud_text,
                        FontId::monospace(12.5),
                        Color32::from_rgb(0xEC, 0xED, 0xEF),
                    );
                });
        });

        if !matches!(outcome, PickerOutcome::Pending) {
            self.active = false;
        }
        outcome
    }
}
