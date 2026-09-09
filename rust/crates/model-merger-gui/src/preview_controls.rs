//! Small, labeled native controls with one consistent monoline icon style.
use crate::theme;
use eframe::egui;

#[derive(Clone, Copy)]
pub(crate) enum Icon {
    Reset,
    Grid,
    Left,
    Right,
    ZoomOut,
    ZoomIn,
}

pub(crate) fn button(ui: &mut egui::Ui, icon: Icon, label: &str, selected: bool) -> egui::Response {
    let response = ui.add(
        egui::Button::new("")
            .min_size(egui::vec2(32.0, 32.0))
            .selected(selected),
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, ui.is_enabled(), selected, label)
    });
    let palette = theme::palette(ui);
    let color = if !ui.is_enabled() {
        palette.secondary
    } else if selected {
        palette.on_primary
    } else {
        palette.foreground
    };
    let stroke = egui::Stroke::new(1.5, color);
    let center = response.rect.center();
    let point = |x, y| center + egui::vec2(x, y);
    let line = |a, b| {
        ui.painter().line_segment([a, b], stroke);
    };
    match icon {
        Icon::Grid => {
            ui.painter().rect_stroke(
                egui::Rect::from_center_size(center, egui::vec2(16.0, 16.0)),
                2.0,
                stroke,
                egui::StrokeKind::Inside,
            );
            for offset in [-2.5, 2.5] {
                line(point(offset, -7.0), point(offset, 7.0));
                line(point(-7.0, offset), point(7.0, offset));
            }
        }
        Icon::ZoomIn | Icon::ZoomOut => {
            ui.painter().circle_stroke(point(-2.0, -2.0), 5.5, stroke);
            line(point(2.0, 2.0), point(7.0, 7.0));
            line(point(-5.0, -2.0), point(1.0, -2.0));
            if matches!(icon, Icon::ZoomIn) {
                line(point(-2.0, -5.0), point(-2.0, 1.0));
            }
        }
        Icon::Left | Icon::Right => {
            let sign = if matches!(icon, Icon::Left) {
                1.0
            } else {
                -1.0
            };
            let points: Vec<_> = (0..=18)
                .map(|step| {
                    let angle = -2.3 + step as f32 * 4.7 / 18.0;
                    point(angle.cos() * 6.5 * sign, angle.sin() * 6.5)
                })
                .collect();
            let end = points[0];
            ui.painter().add(egui::Shape::line(points, stroke));
            line(end, end + egui::vec2(-sign, 4.5));
            line(end, end + egui::vec2(4.0 * sign, 0.0));
        }
        Icon::Reset => {
            for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                line(point(x * 7.0, y * 3.0), point(x * 7.0, y * 7.0));
                line(point(x * 7.0, y * 7.0), point(x * 3.0, y * 7.0));
            }
            ui.painter().circle_stroke(center, 2.0, stroke);
        }
    }
    response.on_hover_text(label)
}
