use eframe::egui::{pos2, vec2, Color32, Painter, Pos2, Rect, Shape, Stroke, StrokeKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderIconKind {
    SoundOn,
    SoundOff,
    ThemeSystem,
    ThemeLight,
    ThemeDark,
    PinOn,
    PinOff,
    Update,
}

pub fn paint_header_icon(painter: &Painter, rect: Rect, kind: HeaderIconKind, color: Color32, badge: Option<Color32>) {
    // 14x14 coordinate box centered inside button rect
    let size = 14.0_f32;
    let ox = rect.center().x - size * 0.5;
    let oy = rect.center().y - size * 0.5;
    let pt = |x: f32, y: f32| pos2(ox + x, oy + y);
    let stroke = Stroke::new(1.4_f32, color);

    match kind {
        HeaderIconKind::SoundOn | HeaderIconKind::SoundOff => {
            // Speaker cone polygon
            let cone = vec![
                pt(1.5, 5.0),
                pt(4.2, 5.0),
                pt(7.5, 2.2),
                pt(7.5, 11.8),
                pt(4.2, 9.0),
                pt(1.5, 9.0),
            ];
            painter.add(Shape::convex_polygon(cone, color, Stroke::NONE));

            if kind == HeaderIconKind::SoundOn {
                // Inner and outer arcs approximated cleanly
                draw_arc(painter, pt(7.5, 7.0), 3.0, -0.75, 0.75, stroke);
                draw_arc(painter, pt(7.5, 7.0), 5.2, -0.82, 0.82, stroke);
            } else {
                painter.line_segment([pt(9.3, 4.7), pt(12.8, 9.3)], Stroke::new(1.5_f32, color));
                painter.line_segment([pt(12.8, 4.7), pt(9.3, 9.3)], Stroke::new(1.5_f32, color));
            }
        }
        HeaderIconKind::ThemeLight => {
            painter.circle_filled(pt(7.0, 7.0), 2.7, color);
            for i in 0..8 {
                let ang = (i as f32) * std::f32::consts::FRAC_PI_4;
                let p0 = pos2(ox + 7.0 + 4.3 * ang.cos(), oy + 7.0 + 4.3 * ang.sin());
                let p1 = pos2(ox + 7.0 + 6.1 * ang.cos(), oy + 7.0 + 6.1 * ang.sin());
                painter.line_segment([p0, p1], Stroke::new(1.35_f32, color));
            }
        }
        HeaderIconKind::ThemeDark => {
            // Crescent moon built cleanly from a circle arc
            draw_crescent(painter, pt(7.2, 7.0), 4.9, color);
        }
        HeaderIconKind::ThemeSystem => {
            let monitor = Rect::from_min_max(pt(1.5, 2.2), pt(12.5, 9.8));
            painter.rect_stroke(monitor, 1.5, stroke, StrokeKind::Inside);
            painter.line_segment([pt(7.0, 9.8), pt(7.0, 11.8)], stroke);
            painter.line_segment([pt(4.3, 12.0), pt(9.7, 12.0)], Stroke::new(1.5_f32, color));
        }
        HeaderIconKind::PinOn | HeaderIconKind::PinOff => {
            if kind == HeaderIconKind::PinOn {
                // Upright active pushpin
                let head = vec![pt(4.5, 2.0), pt(9.5, 2.0), pt(8.8, 7.2), pt(5.2, 7.2)];
                painter.add(Shape::convex_polygon(head, color, Stroke::NONE));
                painter.line_segment([pt(3.0, 7.5), pt(11.0, 7.5)], Stroke::new(1.6_f32, color));
                painter.line_segment([pt(7.0, 7.5), pt(7.0, 12.4)], Stroke::new(1.5_f32, color));
            } else {
                // Tilted neutral pushpin
                painter.line_segment([pt(5.5, 2.5), pt(10.5, 2.5)], Stroke::new(1.4_f32, color));
                painter.line_segment([pt(5.8, 2.5), pt(5.0, 6.8)], stroke);
                painter.line_segment([pt(10.2, 2.5), pt(9.0, 6.8)], stroke);
                painter.line_segment([pt(3.5, 7.0), pt(10.5, 7.0)], Stroke::new(1.4_f32, color));
                painter.line_segment([pt(7.0, 7.0), pt(7.0, 12.0)], Stroke::new(1.4_f32, color));
            }
        }
        HeaderIconKind::Update => {
            // Downward arrow
            painter.line_segment([pt(7.0, 1.8), pt(7.0, 8.8)], Stroke::new(1.6_f32, color));
            painter.line_segment([pt(4.1, 6.1), pt(7.0, 9.1)], Stroke::new(1.6_f32, color));
            painter.line_segment([pt(9.9, 6.1), pt(7.0, 9.1)], Stroke::new(1.6_f32, color));
            // Base tray
            let tray = [pt(2.0, 9.6), pt(2.0, 12.0), pt(12.0, 12.0), pt(12.0, 9.6)];
            for w in tray.windows(2) {
                painter.line_segment([w[0], w[1]], Stroke::new(1.45_f32, color));
            }
        }
    }

    if let Some(dot) = badge {
        painter.circle_filled(rect.right_top() + vec2(-4.5, 4.5), 2.8, dot);
    }
}

fn draw_arc(painter: &Painter, center: Pos2, r: f32, a0: f32, a1: f32, stroke: Stroke) {
    let steps = 8;
    let pts: Vec<Pos2> = (0..=steps)
        .map(|i| {
            let t = i as f32 / steps as f32;
            let a = a0 + (a1 - a0) * t;
            pos2(center.x + r * a.cos(), center.y + r * a.sin())
        })
        .collect();
    painter.add(Shape::line(pts, stroke));
}

fn draw_crescent(painter: &Painter, center: Pos2, r: f32, color: Color32) {
    // Draw clean crescent outline/shape
    let mut outer = Vec::with_capacity(24);
    for i in 0..=16 {
        let a = 0.55 + (i as f32 / 16.0) * (std::f32::consts::TAU - 2.1);
        outer.push(pos2(center.x + r * a.cos(), center.y + r * a.sin()));
    }
    let offset = center + vec2(r * 0.42, -r * 0.28);
    let inner_r = r * 0.78;
    for i in (0..=12).rev() {
        let a = 0.9 + (i as f32 / 12.0) * 3.2;
        outer.push(pos2(offset.x + inner_r * a.cos(), offset.y + inner_r * a.sin()));
    }
    painter.add(Shape::closed_line(outer, Stroke::new(1.5_f32, color)));
}
