//! User-requested macOS-style window chrome, with native Windows window commands.
use crate::theme;
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke};
use model_merger_app_core::{AppLanguage, Catalog, TextKey};

fn text(language: AppLanguage, choices: [&'static str; 5]) -> &'static str {
    choices[language as usize]
}

pub(crate) fn show(ui: &mut egui::Ui, language: AppLanguage) {
    let palette = theme::palette(ui);
    let catalog = Catalog::new(language);
    let (maximized, focused) = ui.ctx().input(|input| {
        (
            input.viewport().maximized.unwrap_or(false),
            input.viewport().focused.unwrap_or(true),
        )
    });
    egui::Panel::top("window-chrome")
        .frame(egui::Frame::new().fill(palette.toolbar))
        .exact_size(36.0)
        .show(ui, |ui| {
            let rect = ui.max_rect();
            let controls = [
                (
                    Color32::from_rgb(255, 95, 87),
                    catalog.text(TextKey::Close),
                    egui::ViewportCommand::Close,
                ),
                (
                    Color32::from_rgb(255, 189, 46),
                    text(
                        language,
                        ["最小化", "Minimize", "Réduire", "Свернуть", "Minimizar"],
                    ),
                    egui::ViewportCommand::Minimized(true),
                ),
                (
                    Color32::from_rgb(40, 201, 64),
                    if maximized {
                        text(
                            language,
                            [
                                "还原窗口",
                                "Restore window",
                                "Restaurer",
                                "Восстановить",
                                "Restaurar ventana",
                            ],
                        )
                    } else {
                        text(
                            language,
                            ["最大化", "Maximize", "Agrandir", "Развернуть", "Maximizar"],
                        )
                    },
                    egui::ViewportCommand::Maximized(!maximized),
                ),
            ];
            for (index, (color, label, command)) in controls.into_iter().enumerate() {
                let center = Pos2::new(rect.left() + 22.0 + index as f32 * 22.0, rect.center().y);
                let response = ui
                    .put(
                        Rect::from_center_size(center, egui::vec2(20.0, 24.0)),
                        egui::Button::new("").frame(false),
                    )
                    .on_hover_text(label);
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label)
                });
                ui.painter().circle_filled(
                    center,
                    6.0,
                    if focused || response.hovered() {
                        color
                    } else {
                        palette.border
                    },
                );
                ui.painter().circle_stroke(
                    center,
                    6.0,
                    Stroke::new(0.5, Color32::from_black_alpha(45)),
                );
                if response.has_focus() {
                    ui.painter()
                        .circle_stroke(center, 8.5, Stroke::new(1.5, palette.primary));
                }
                if response.hovered() || response.has_focus() {
                    let stroke = Stroke::new(1.0, Color32::from_black_alpha(160));
                    match index {
                        0 => {
                            ui.painter().line_segment(
                                [
                                    center + egui::vec2(-2.5, -2.5),
                                    center + egui::vec2(2.5, 2.5),
                                ],
                                stroke,
                            );
                            ui.painter().line_segment(
                                [
                                    center + egui::vec2(-2.5, 2.5),
                                    center + egui::vec2(2.5, -2.5),
                                ],
                                stroke,
                            );
                        }
                        1 => {
                            ui.painter().line_segment(
                                [center - egui::vec2(3.0, 0.0), center + egui::vec2(3.0, 0.0)],
                                stroke,
                            );
                        }
                        _ => {
                            ui.painter().line_segment(
                                [center - egui::vec2(3.0, 0.0), center + egui::vec2(3.0, 0.0)],
                                stroke,
                            );
                            if !maximized {
                                ui.painter().line_segment(
                                    [center - egui::vec2(0.0, 3.0), center + egui::vec2(0.0, 3.0)],
                                    stroke,
                                );
                            }
                        }
                    }
                }
                if response.clicked() {
                    ui.ctx().send_viewport_cmd(command);
                }
            }
            let drag_rect = Rect::from_min_max(
                Pos2::new(rect.left() + 88.0, rect.top() + 4.0),
                Pos2::new(rect.right() - 5.0, rect.bottom()),
            );
            let response = ui.interact(
                drag_rect,
                ui.id().with("window-drag"),
                Sense::click_and_drag(),
            );
            if response.double_clicked() {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
            } else if response.drag_started() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                catalog.text(TextKey::AppTitle),
                FontId::proportional(13.0),
                if focused {
                    palette.foreground
                } else {
                    palette.secondary
                },
            );
            ui.painter().hline(
                rect.x_range(),
                rect.bottom() - 0.5,
                Stroke::new(1.0, palette.border),
            );
        });
}

pub(crate) fn resize_edges(ui: &mut egui::Ui) {
    if ui.ctx().input(|input| {
        input.viewport().maximized.unwrap_or(false) || input.viewport().fullscreen.unwrap_or(false)
    }) {
        return;
    }
    let rect = ui.ctx().content_rect();
    let xs = [
        rect.left(),
        rect.left() + 5.0,
        rect.right() - 5.0,
        rect.right(),
    ];
    let ys = [
        rect.top(),
        rect.top() + 5.0,
        rect.bottom() - 5.0,
        rect.bottom(),
    ];
    for (index, (x, y)) in [
        (0, 0),
        (1, 0),
        (2, 0),
        (0, 1),
        (2, 1),
        (0, 2),
        (1, 2),
        (2, 2),
    ]
    .into_iter()
    .enumerate()
    {
        let zone = Rect::from_min_max(egui::pos2(xs[x], ys[y]), egui::pos2(xs[x + 1], ys[y + 1]));
        let direction = resize_direction(rect, zone.center()).unwrap();
        let cursor = match direction {
            egui::ResizeDirection::North | egui::ResizeDirection::South => {
                egui::CursorIcon::ResizeVertical
            }
            egui::ResizeDirection::East | egui::ResizeDirection::West => {
                egui::CursorIcon::ResizeHorizontal
            }
            egui::ResizeDirection::NorthWest | egui::ResizeDirection::SouthEast => {
                egui::CursorIcon::ResizeNwSe
            }
            _ => egui::CursorIcon::ResizeNeSw,
        };
        let response = ui
            .interact(zone, ui.id().with(("resize-edge", index)), Sense::drag())
            .on_hover_cursor(cursor);
        if response.is_pointer_button_down_on()
            && ui.ctx().input(|input| input.pointer.primary_pressed())
        {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::BeginResize(direction));
        }
    }
}

fn resize_direction(rect: Rect, pointer: Pos2) -> Option<egui::ResizeDirection> {
    if !rect.contains(pointer) {
        return None;
    }
    let left = pointer.x <= rect.left() + 5.0;
    let right = pointer.x >= rect.right() - 5.0;
    let top = pointer.y <= rect.top() + 5.0;
    let bottom = pointer.y >= rect.bottom() - 5.0;
    use egui::ResizeDirection::*;
    match (left, right, top, bottom) {
        (true, _, true, _) => Some(NorthWest),
        (_, true, true, _) => Some(NorthEast),
        (true, _, _, true) => Some(SouthWest),
        (_, true, _, true) => Some(SouthEast),
        (true, _, _, _) => Some(West),
        (_, true, _, _) => Some(East),
        (_, _, true, _) => Some(North),
        (_, _, _, true) => Some(South),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn traffic_lights_dispatch_native_window_commands() {
        for (index, expected) in [(0, "Close"), (1, "Minimized(true)"), (2, "Maximized(true)")] {
            let context = egui::Context::default();
            theme::configure(&context, AppLanguage::English);
            let position = egui::pos2(22.0 + index as f32 * 22.0, 18.0);
            for step in 0..5 {
                let mut input = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(900.0, 680.0))),
                    ..Default::default()
                };
                if step >= 2 {
                    input.events.push(egui::Event::PointerMoved(position));
                }
                if step >= 3 {
                    input.events.push(egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed: step == 3,
                        modifiers: Default::default(),
                    });
                }
                let output = context.run_ui(input, |ui| show(ui, AppLanguage::English));
                if step == 4 {
                    let commands: Vec<_> = output
                        .viewport_output
                        .values()
                        .flat_map(|viewport| &viewport.commands)
                        .collect();
                    assert!(
                        commands
                            .iter()
                            .any(|command| format!("{command:?}") == expected),
                        "{expected}: {commands:?}"
                    );
                }
                output.drop_without_applying_deltas();
            }
        }
    }
    #[test]
    fn window_resize_edges_do_not_capture_content() {
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(900.0, 680.0));
        assert_eq!(
            resize_direction(rect, egui::pos2(1.0, 1.0)),
            Some(egui::ResizeDirection::NorthWest)
        );
        assert_eq!(
            resize_direction(rect, egui::pos2(899.0, 679.0)),
            Some(egui::ResizeDirection::SouthEast)
        );
        assert_eq!(
            resize_direction(rect, egui::pos2(450.0, 679.0)),
            Some(egui::ResizeDirection::South)
        );
        assert_eq!(resize_direction(rect, egui::pos2(22.0, 18.0)), None);
        assert_eq!(resize_direction(rect, rect.center()), None);
    }
}
