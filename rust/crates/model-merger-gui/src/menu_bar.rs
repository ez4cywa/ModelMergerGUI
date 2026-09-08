//! Compact native menus: global commands stay separate from the model workspace.
use crate::theme;
use eframe::egui;
use model_merger_app_core::{AppLanguage, Catalog, TextKey};

#[derive(Debug, PartialEq)]
pub(crate) enum Action {
    NewGroup,
    SaveSettings,
    RestoreDefaults,
    Language(AppLanguage),
    About,
}

pub(crate) fn labels(language: AppLanguage) -> [&'static str; 3] {
    [
        ["文件", "设置", "帮助"],
        ["File", "Settings", "Help"],
        ["Fichier", "Réglages", "Aide"],
        ["Файл", "Настройки", "Справка"],
        ["Archivo", "Ajustes", "Ayuda"],
    ][language as usize]
}

pub(crate) fn show(
    root: &mut egui::Ui,
    language: AppLanguage,
    can_restore: bool,
) -> Option<Action> {
    let palette = theme::palette(root);
    let catalog = Catalog::new(language);
    let [file, settings, help] = labels(language);
    let mut action = None;
    egui::Panel::top("top-command-bar")
        .frame(
            egui::Frame::new()
                .fill(palette.toolbar)
                .inner_margin(egui::Margin::symmetric(12, 3)),
        )
        .show(root, |ui| {
            ui.spacing_mut().interact_size.y = 26.0;
            ui.spacing_mut().button_padding = egui::vec2(10.0, 4.0);
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button(file, |ui| {
                    menu_style(ui);
                    if ui
                        .add(
                            egui::Button::new(catalog.text(TextKey::NewGroup))
                                .shortcut_text(shortcut(ui, egui::Key::N)),
                        )
                        .clicked()
                    {
                        action = Some(Action::NewGroup);
                        ui.close();
                    }
                });
                ui.menu_button(settings, |ui| {
                    menu_style(ui);
                    ui.menu_button(catalog.text(TextKey::Language), |ui| {
                        menu_style(ui);
                        for candidate in AppLanguage::ALL {
                            if ui
                                .selectable_label(
                                    candidate == language,
                                    Catalog::new(candidate).language_name(),
                                )
                                .clicked()
                            {
                                action = Some(Action::Language(candidate));
                                ui.close();
                            }
                        }
                    });
                    ui.separator();
                    if ui
                        .add(
                            egui::Button::new(catalog.text(TextKey::SaveSettings))
                                .shortcut_text(shortcut(ui, egui::Key::S)),
                        )
                        .clicked()
                    {
                        action = Some(Action::SaveSettings);
                        ui.close();
                    }
                    if ui
                        .add_enabled(
                            can_restore,
                            egui::Button::new(catalog.text(TextKey::RestoreDefaults)),
                        )
                        .clicked()
                    {
                        action = Some(Action::RestoreDefaults);
                        ui.close();
                    }
                });
                ui.menu_button(help, |ui| {
                    menu_style(ui);
                    if ui.button(catalog.text(TextKey::About)).clicked() {
                        action = Some(Action::About);
                        ui.close();
                    }
                });
            });
            let rect = ui.max_rect();
            ui.painter().hline(
                rect.x_range(),
                rect.bottom() + 2.5,
                egui::Stroke::new(1.0, palette.border),
            );
        });
    action
}

fn menu_style(ui: &mut egui::Ui) {
    ui.set_min_width(220.0);
    ui.spacing_mut().interact_size.y = 28.0;
}

fn shortcut(ui: &egui::Ui, key: egui::Key) -> String {
    ui.ctx()
        .format_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, key))
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Harness {
        ctx: egui::Context,
        language: AppLanguage,
        restore: bool,
        action: Option<Action>,
    }
    impl Harness {
        fn frame(&mut self, events: Vec<egui::Event>) -> egui::FullOutput {
            self.ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(900.0, 680.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    self.action = show(ui, self.language, self.restore);
                },
            )
        }
        fn click(&mut self, label: &str) {
            for _ in 0..3 {
                self.frame(vec![]).drop_without_applying_deltas();
            }
            let output = self.frame(vec![]);
            let position = theme::review_text(&output.shapes)
                .iter()
                .find(|(rect, text, clip)| text == label && clip.contains_rect(*rect))
                .unwrap_or_else(|| panic!("Missing menu item: {label}"))
                .0
                .center();
            output.drop_without_applying_deltas();
            self.frame(vec![egui::Event::PointerMoved(position)])
                .drop_without_applying_deltas();
            for pressed in [true, false] {
                self.frame(vec![egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                }])
                .drop_without_applying_deltas();
            }
        }
    }
    #[test]
    fn keyboard_can_open_and_dismiss_the_file_menu() {
        let ctx = egui::Context::default();
        theme::configure(&ctx, AppLanguage::English);
        let mut h = Harness {
            ctx,
            language: AppLanguage::English,
            restore: true,
            action: None,
        };
        for _ in 0..3 {
            h.frame(vec![]).drop_without_applying_deltas();
        }
        for key in [egui::Key::Tab, egui::Key::Enter] {
            for pressed in [true, false] {
                h.frame(vec![egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed,
                    repeat: false,
                    modifiers: Default::default(),
                }])
                .drop_without_applying_deltas();
            }
        }
        let output = h.frame(vec![]);
        let has_new = theme::review_text(&output.shapes)
            .iter()
            .any(|(_, text, _)| text == Catalog::new(AppLanguage::English).text(TextKey::NewGroup));
        output.drop_without_applying_deltas();
        assert!(has_new, "keyboard must open the file menu");
        h.frame(vec![egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        }])
        .drop_without_applying_deltas();
        let output = h.frame(vec![]);
        let has_new = theme::review_text(&output.shapes)
            .iter()
            .any(|(_, text, _)| text == Catalog::new(AppLanguage::English).text(TextKey::NewGroup));
        output.drop_without_applying_deltas();
        assert!(!has_new, "Escape must dismiss the menu");
    }

    #[test]
    fn localized_menus_dispatch_commands_and_preserve_disabled_restore() {
        for language in AppLanguage::ALL {
            for dark in [false, true] {
                let ctx = egui::Context::default();
                theme::configure(&ctx, language);
                ctx.set_theme(if dark {
                    egui::ThemePreference::Dark
                } else {
                    egui::ThemePreference::Light
                });
                let mut h = Harness {
                    ctx,
                    language,
                    restore: false,
                    action: None,
                };
                let catalog = Catalog::new(language);
                let [file, settings, help] = labels(language);
                h.click(file);
                h.click(catalog.text(TextKey::NewGroup));
                assert_eq!(h.action, Some(Action::NewGroup));
                h.click(settings);
                h.click(catalog.text(TextKey::RestoreDefaults));
                assert_eq!(h.action, None);
                h.click(settings);
                h.click(catalog.text(TextKey::SaveSettings));
                assert_eq!(h.action, Some(Action::SaveSettings));
                h.click(settings);
                h.click(catalog.text(TextKey::Language));
                h.click(Catalog::new(AppLanguage::English).language_name());
                assert_eq!(h.action, Some(Action::Language(AppLanguage::English)));
                // Dismiss any parent menu left open by the language submenu.
                h.frame(vec![egui::Event::Key {
                    key: egui::Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Default::default(),
                }])
                .drop_without_applying_deltas();
                h.click(help);
                h.click(catalog.text(TextKey::About));
                assert_eq!(h.action, Some(Action::About));
            }
        }
    }
}
