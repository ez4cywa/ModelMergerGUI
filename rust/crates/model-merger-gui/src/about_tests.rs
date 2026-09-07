use super::*;

#[test]
fn about_dialog_fits_all_languages_and_themes() {
    for language in AppLanguage::ALL {
        for dark in [false, true] {
            for size in [egui::vec2(900.0, 680.0), egui::vec2(1180.0, 860.0)] {
                let context = egui::Context::default();
                theme::configure(&context, language);
                context.set_theme(if dark {
                    egui::ThemePreference::Dark
                } else {
                    egui::ThemePreference::Light
                });
                let mut dialog = AboutDialog::default();
                dialog.open();
                for frame in 0..3 {
                    let mut output = context.run_ui(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                            ..Default::default()
                        },
                        |ui| dialog.show(ui.ctx(), language),
                    );
                    let labels = theme::review_text(&output.shapes);
                    output.textures_delta.clear();
                    if frame < 2 {
                        continue;
                    }
                    for (rect, text, _) in &labels {
                        assert!(
                            rect.left() >= 0.0 && rect.right() <= size.x,
                            "{language:?}: overflow {text}"
                        );
                    }
                    assert!(
                        labels
                            .iter()
                            .any(|(_, text, _)| text.contains(env!("CARGO_PKG_VERSION")))
                    );
                    assert!(
                        labels
                            .iter()
                            .any(|(_, text, _)| text == "ez4cywa / ModelMergerGUI")
                    );
                    assert!(labels.iter().any(|(rect, text, clip)| text
                        == Catalog::new(language).text(TextKey::Close)
                        && rect.bottom() <= size.y
                        && clip.contains_rect(*rect)));
                    assert!(
                        !output
                            .platform_output
                            .commands
                            .iter()
                            .any(|c| matches!(c, egui::OutputCommand::OpenUrl(_))),
                        "opening About must not open a browser"
                    );
                    output.drop_without_applying_deltas();
                }
            }
        }
    }
}
