use std::collections::VecDeque;
use std::time::Instant;

use eframe::egui::{
    self, Align, Color32, CornerRadius, CursorIcon, FontId, Frame, Margin, Pos2, RichText, Sense,
    Shape, Stroke, StrokeKind, TextEdit, Ui, Vec2,
};

use crate::ui::icons::{paint_header_icon, HeaderIconKind};
use crate::ui::theme::Palette;

pub const WINDOW_WIDTH: f32 = 476.0;
pub const CONTROL_H: f32 = 30.0;
pub const ROW_LABEL_W: f32 = 72.0;

pub fn card_frame(p: &Palette, highlight: bool) -> Frame {
    Frame::new()
        .fill(p.surface)
        .stroke(Stroke::new(
            1.0_f32,
            if highlight { p.accent } else { p.border },
        ))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(14, 11))
}

pub fn section_title(ui: &mut Ui, p: &Palette, text: &str) {
    ui.label(
        RichText::new(text.to_ascii_uppercase())
            .font(FontId::proportional(10.5))
            .strong()
            .color(p.text_faint),
    );
}

pub fn row_label(ui: &mut Ui, p: &Palette, text: &str, enabled: bool) {
    ui.allocate_ui_with_layout(
        Vec2::new(ROW_LABEL_W, CONTROL_H),
        egui::Layout::left_to_right(Align::Center),
        |ui| {
            ui.label(
                RichText::new(text)
                    .font(FontId::proportional(12.0))
                    .color(if enabled { p.text_muted } else { p.text_faint }),
            );
        },
    );
}

pub fn segmented_control<T: Copy + PartialEq>(
    ui: &mut Ui,
    p: &Palette,
    id_salt: &str,
    current: &mut T,
    options: &[(T, &str)],
    enabled: bool,
    total_width: f32,
) -> bool {
    let mut changed = false;
    let n = options.len().max(1);
    let desired = Vec2::new(total_width.max(40.0), CONTROL_H);
    let (outer_rect, _) = ui.allocate_exact_size(desired, Sense::hover());

    if ui.is_rect_visible(outer_rect) {
        let painter = ui.painter();
        painter.rect(
            outer_rect,
            CornerRadius::same(7),
            if enabled { p.field } else { p.field_disabled },
            Stroke::new(1.0_f32, p.border),
            StrokeKind::Inside,
        );

        let pad = 2.0;
        let inner = outer_rect.shrink(pad);
        let seg_w = inner.width() / n as f32;

        for (i, (val, label)) in options.iter().enumerate() {
            let x0 = inner.min.x + seg_w * i as f32;
            let seg_rect = egui::Rect::from_min_max(
                egui::pos2(x0, inner.min.y),
                egui::pos2(x0 + seg_w, inner.max.y),
            );
            let resp = ui.interact(
                seg_rect,
                ui.make_persistent_id((id_salt, i)),
                if enabled { Sense::click() } else { Sense::hover() },
            );
            let selected = *val == *current;
            let fill = if selected {
                if resp.hovered() && enabled {
                    p.seg_selected_hover
                } else {
                    p.seg_selected
                }
            } else if resp.hovered() && enabled {
                p.hover
            } else {
                Color32::TRANSPARENT
            };

            if fill != Color32::TRANSPARENT {
                let seg_stroke = if selected && !p.is_dark {
                    Stroke::new(1.0_f32, p.border)
                } else {
                    Stroke::NONE
                };
                painter.rect(
                    seg_rect,
                    CornerRadius::same(5),
                    fill,
                    seg_stroke,
                    StrokeKind::Inside,
                );
            }

            let text_color = if !enabled {
                p.text_faint
            } else if selected {
                p.text
            } else {
                p.text_muted
            };

            painter.text(
                seg_rect.center(),
                egui::Align2::CENTER_CENTER,
                label,
                FontId::proportional(12.0),
                text_color,
            );

            if resp.hovered() && enabled {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            if resp.clicked() && enabled && !selected {
                *current = *val;
                changed = true;
            }
        }
    }

    changed
}

pub fn toggle_switch(ui: &mut Ui, p: &Palette, checked: &mut bool, enabled: bool) -> bool {
    let desired = Vec2::new(36.0, 19.0);
    let (rect, resp) = ui.allocate_exact_size(
        desired,
        if enabled { Sense::click() } else { Sense::hover() },
    );
    if resp.hovered() && enabled {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    let mut toggled = false;
    if resp.clicked() && enabled {
        *checked = !*checked;
        toggled = true;
    }
    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool_responsive(resp.id, *checked);
        let track_color = if !enabled {
            p.field_disabled
        } else if *checked {
            p.accent
        } else {
            p.switch_track
        };
        let painter = ui.painter();
        painter.rect_filled(rect, CornerRadius::same(10), track_color);
        let radius = rect.height() * 0.5 - 2.5;
        let thumb_x = egui::lerp((rect.min.x + radius + 2.5)..=(rect.max.x - radius - 2.5), how_on);
        let thumb_color = if enabled { p.switch_thumb } else { p.text_faint };
        painter.circle_filled(egui::pos2(thumb_x, rect.center().y), radius, thumb_color);
    }
    toggled
}

#[allow(clippy::too_many_arguments)]
pub fn number_field(
    ui: &mut Ui,
    p: &Palette,
    id_salt: &str,
    text: &mut String,
    width: f32,
    allow_fn: fn(&str) -> bool,
    enabled: bool,
    error_until: Option<Instant>,
) -> (egui::Response, bool) {
    let mut prev = text.clone();
    let is_err = error_until.is_some_and(|t| Instant::now() < t);

    let frame_bg = if enabled { p.field } else { p.field_disabled };
    let border_col = if is_err {
        p.danger
    } else if enabled {
        p.border_strong
    } else {
        p.border
    };

    let mut committed = false;
    let resp = ui
        .allocate_ui_with_layout(
            Vec2::new(width, CONTROL_H),
            egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
            |ui| {
                let mut inner_resp = None;
                Frame::new()
                    .fill(frame_bg)
                    .stroke(Stroke::new(1.0_f32, border_col))
                    .corner_radius(CornerRadius::same(7))
                    .inner_margin(Margin::symmetric(6, 5))
                    .show(ui, |ui| {
                        let edit = TextEdit::singleline(text)
                            .id(ui.make_persistent_id(id_salt))
                            .font(FontId::monospace(12.5))
                            .text_color(if enabled { p.text } else { p.text_faint })
                            .horizontal_align(Align::Center)
                            .frame(false)
                            .interactive(enabled)
                            .desired_width(width - 14.0);
                        let r = ui.add(edit);
                        if r.has_focus() && !is_err && enabled {
                            ui.painter().rect_stroke(
                                r.rect.expand2(Vec2::new(6.0, 5.0)),
                                CornerRadius::same(7),
                                Stroke::new(1.2_f32, p.accent),
                                StrokeKind::Inside,
                            );
                        }
                        if r.changed() && !allow_fn(text) {
                            *text = prev.split_off(0);
                        }
                        if r.lost_focus()
                            || (r.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                        {
                            committed = true;
                        }
                        inner_resp = Some(r);
                    });
                inner_resp.unwrap()
            },
        )
        .inner;

    (resp, committed)
}

pub fn ghost_button(
    ui: &mut Ui,
    p: &Palette,
    text: &str,
    size: Vec2,
    enabled: bool,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(
        size,
        if enabled { Sense::click() } else { Sense::hover() },
    );
    if resp.hovered() && enabled {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    if ui.is_rect_visible(rect) {
        let fill = if !enabled {
            p.field_disabled
        } else if resp.is_pointer_button_down_on() {
            p.seg_selected
        } else if resp.hovered() {
            p.hover
        } else {
            p.field
        };
        ui.painter().rect(
            rect,
            CornerRadius::same(7),
            fill,
            Stroke::new(1.0_f32, if enabled { p.border_strong } else { p.border }),
            StrokeKind::Inside,
        );
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            FontId::proportional(11.5),
            if enabled { p.text } else { p.text_faint },
        );
    }
    resp
}

pub fn header_icon_button(
    ui: &mut Ui,
    p: &Palette,
    kind: HeaderIconKind,
    active: bool,
    badge: Option<Color32>,
    tooltip: &str,
) -> egui::Response {
    let size = Vec2::splat(28.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    if ui.is_rect_visible(rect) {
        let fill = if active {
            p.accent_soft
        } else if resp.hovered() {
            p.hover
        } else {
            p.field
        };
        let stroke = Stroke::new(1.0_f32, if active { p.accent } else { p.border });
        ui.painter()
            .rect(rect, CornerRadius::same(7), fill, stroke, StrokeKind::Inside);
        let icon_color = if active { p.accent } else { p.text_muted };
        paint_header_icon(ui.painter(), rect, kind, icon_color, badge);
    }
    resp.on_hover_text(tooltip)
}

pub fn accent_button(
    ui: &mut Ui,
    p: &Palette,
    text: &str,
    size: Vec2,
    danger: bool,
    font_size: f32,
    radius: u8,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    if ui.is_rect_visible(rect) {
        let base = if danger { p.danger } else { p.accent };
        let hov = if danger { p.danger_hover } else { p.accent_hover };
        let fill = if resp.hovered() { hov } else { base };
        ui.painter()
            .rect_filled(rect, CornerRadius::same(radius), fill);
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            FontId::proportional(font_size),
            p.text_on_accent,
        );
    }
    resp
}

pub fn warning_button(
    ui: &mut Ui,
    p: &Palette,
    text: &str,
    size: Vec2,
    font_size: f32,
    radius: u8,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    if ui.is_rect_visible(rect) {
        let hov = Color32::from_rgb(
            p.warning.r().saturating_add(14),
            p.warning.g().saturating_add(14),
            p.warning.b().saturating_add(14),
        );
        let fill = if resp.hovered() { hov } else { p.warning };
        ui.painter()
            .rect_filled(rect, CornerRadius::same(radius), fill);
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            FontId::proportional(font_size),
            Color32::from_rgb(0x11, 0x13, 0x18),
        );
    }
    resp
}

pub fn keycap_button(
    ui: &mut Ui,
    p: &Palette,
    label: &str,
    recording: bool,
    enabled: bool,
    width: f32,
) -> egui::Response {
    let size = Vec2::new(width.max(70.0), CONTROL_H);
    let (rect, resp) = ui.allocate_exact_size(
        size,
        if enabled { Sense::click() } else { Sense::hover() },
    );
    if resp.hovered() && enabled {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    if ui.is_rect_visible(rect) {
        let fill = if recording {
            p.accent_soft
        } else if !enabled {
            p.field_disabled
        } else if resp.hovered() {
            p.seg_selected
        } else {
            p.field
        };
        let border = if recording {
            p.warning
        } else if enabled {
            p.border_strong
        } else {
            p.border
        };
        ui.painter().rect(
            rect,
            CornerRadius::same(7),
            fill,
            Stroke::new(1.0_f32, border),
            StrokeKind::Inside,
        );
        // Subtle tactile bottom keycap edge
        if !recording && enabled {
            let y = rect.max.y - 1.5;
            ui.painter().line_segment(
                [egui::pos2(rect.min.x + 5.0, y), egui::pos2(rect.max.x - 5.0, y)],
                Stroke::new(1.0_f32, p.border),
            );
        }
        let text_col = if recording {
            p.warning
        } else if enabled {
            p.text
        } else {
            p.text_faint
        };
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            FontId::monospace(12.0),
            text_col,
        );
    }
    resp
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusPillState {
    Ready,
    Arming,
    Running,
}

pub fn status_pill(ui: &mut Ui, p: &Palette, state: StatusPillState) {
    let (text, width) = match state {
        StatusPillState::Ready => ("Ready", 68.0),
        StatusPillState::Arming => ("Arming", 76.0),
        StatusPillState::Running => ("Running", 78.0),
    };
    let desired = Vec2::new(width, 26.0);
    let (rect, _) = ui.allocate_exact_size(desired, Sense::hover());
    if ui.is_rect_visible(rect) {
        let (fill, border_col, dot_col, text_col) = match state {
            StatusPillState::Ready => (p.field, p.border, p.text_faint, p.text_muted),
            StatusPillState::Arming => (p.field, p.warning, p.warning, p.warning),
            StatusPillState::Running => (p.success_soft, p.accent, p.success, p.success),
        };
        ui.painter().rect(
            rect,
            CornerRadius::same(13),
            fill,
            Stroke::new(1.0_f32, border_col),
            StrokeKind::Inside,
        );
        let dot_center = egui::pos2(rect.min.x + 12.0, rect.center().y);
        if matches!(state, StatusPillState::Running | StatusPillState::Arming) {
            let phase = (ui.input(|i| i.time) * 3.8).sin() as f32 * 0.5 + 0.5;
            let alpha = (95.0 * (1.0 - phase * 0.65)).round() as u8;
            let halo = Color32::from_rgba_unmultiplied(dot_col.r(), dot_col.g(), dot_col.b(), alpha);
            ui.painter()
                .circle_filled(dot_center, 3.5 + phase * 3.2, halo);
        }
        ui.painter().circle_filled(dot_center, 3.5, dot_col);
        ui.painter().text(
            egui::pos2(rect.min.x + 21.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            text,
            FontId::proportional(11.5),
            text_col,
        );
    }
}

pub fn cps_sparkline(
    ui: &mut Ui,
    p: &Palette,
    samples: &VecDeque<f32>,
    active: bool,
    size: Vec2,
) {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.rect(
            rect,
            CornerRadius::same(6),
            p.field,
            Stroke::new(1.0_f32, if active { p.border_strong } else { p.border }),
            StrokeKind::Inside,
        );

        let plot = rect.shrink2(Vec2::new(5.0, 4.0));
        let max_val = samples.iter().copied().fold(0.0_f32, f32::max);
        if samples.len() >= 2 && max_val > 0.0 && plot.width() > 2.0 && plot.height() > 2.0 {
            let scale_max = max_val.max(10.0) * 1.15;
            let n = samples.len();
            let pts: Vec<Pos2> = samples
                .iter()
                .enumerate()
                .map(|(i, &v)| {
                    let tx = i as f32 / (n - 1) as f32;
                    let ty = (v.max(0.0) / scale_max).clamp(0.0, 1.0);
                    Pos2::new(
                        plot.min.x + tx * plot.width(),
                        plot.max.y - ty * plot.height(),
                    )
                })
                .collect();

            let line_col = if active { p.accent } else { p.text_muted };
            let fill_col = Color32::from_rgba_unmultiplied(
                line_col.r(),
                line_col.g(),
                line_col.b(),
                if active { 38 } else { 18 },
            );

            let mut mesh = egui::epaint::Mesh::default();
            for pair in pts.windows(2) {
                let p0 = pair[0];
                let p1 = pair[1];
                let b0 = Pos2::new(p0.x, plot.max.y);
                let b1 = Pos2::new(p1.x, plot.max.y);
                let base = mesh.vertices.len() as u32;
                mesh.colored_vertex(p0, fill_col);
                mesh.colored_vertex(p1, fill_col);
                mesh.colored_vertex(b1, fill_col);
                mesh.colored_vertex(b0, fill_col);
                mesh.add_triangle(base, base + 1, base + 2);
                mesh.add_triangle(base, base + 2, base + 3);
            }
            painter.add(Shape::mesh(mesh));
            painter.add(Shape::line(pts.clone(), Stroke::new(1.5_f32, line_col)));
            if active {
                if let Some(&last) = pts.last() {
                    painter.circle_filled(last, 2.2, p.success);
                }
            }
        } else {
            let y = plot.max.y - 2.0;
            painter.line_segment(
                [Pos2::new(plot.min.x, y), Pos2::new(plot.max.x, y)],
                Stroke::new(1.0_f32, p.border_strong),
            );
        }
    }
    resp.on_hover_text("Live CPS trend");
}
