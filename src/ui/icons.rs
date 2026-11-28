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
            // Crescent moon built from exact circle-circle intersection geometry
            draw_crescent(painter, pt(7.0, 7.0), 5.0, color);
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
                // Symmetric outlined pushpin matching active PinOn geometry
                painter.line_segment([pt(4.5, 2.2), pt(9.5, 2.2)], Stroke::new(1.4_f32, color));
                painter.line_segment([pt(4.9, 2.2), pt(5.3, 7.2)], stroke);
                painter.line_segment([pt(9.1, 2.2), pt(8.7, 7.2)], stroke);
                painter.line_segment([pt(3.2, 7.3), pt(10.8, 7.3)], Stroke::new(1.4_f32, color));
                painter.line_segment([pt(7.0, 7.3), pt(7.0, 12.3)], Stroke::new(1.4_f32, color));
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

fn draw_crescent(painter: &Painter, c1: Pos2, r1: f32, color: Color32) {
    // Exact circle-circle intersection between outer moon disc (c1, r1) and cutout disc (c2, r2)
    let c2 = c1 + vec2(2.6, -2.1);
    let r2 = 4.2_f32;
    let delta = c2 - c1;
    let d = delta.length();
    let a = (r1 * r1 - r2 * r2 + d * d) / (2.0 * d);
    let alpha = (a / r1).clamp(-1.0, 1.0).acos();
    let beta = ((d - a) / r2).clamp(-1.0, 1.0).acos();
    let base1 = delta.y.atan2(delta.x);
    let base2 = base1 + std::f32::consts::PI;

    let mut pts = Vec::with_capacity(34);
    let outer_steps = 20;
    let outer_start = base1 + alpha;
    let outer_sweep = std::f32::consts::TAU - 2.0 * alpha;
    for i in 0..=outer_steps {
        let t = i as f32 / outer_steps as f32;
        let ang = outer_start + outer_sweep * t;
        pts.push(pos2(c1.x + r1 * ang.cos(), c1.y + r1 * ang.sin()));
    }

    let inner_steps = 14;
    let inner_start = base2 + beta;
    let inner_end = base2 - beta;
    for i in 1..inner_steps {
        let t = i as f32 / inner_steps as f32;
        let ang = inner_start + (inner_end - inner_start) * t;
        pts.push(pos2(c2.x + r2 * ang.cos(), c2.y + r2 * ang.sin()));
    }

    painter.add(Shape::closed_line(pts, Stroke::new(1.45_f32, color)));
}
