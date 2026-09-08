use super::*;

struct InteractionHarness {
    context: egui::Context,
    dialog: AboutDialog,
}

impl InteractionHarness {
    fn new() -> Self {
        let context = egui::Context::default();
        theme::configure(&context, AppLanguage::English);
        context.global_style_mut(|style| style.animation_time = 0.0);
        let mut dialog = AboutDialog::default();
        dialog.open();
        let mut harness = Self { context, dialog };
        for _ in 0..3 {
            harness.frame(vec![]).drop_without_applying_deltas();
        }
        harness
    }

    fn frame(&mut self, events: Vec<egui::Event>) -> egui::FullOutput {
        self.context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1180.0, 1000.0),
                )),
                events,
                ..Default::default()
            },
            |ui| self.dialog.show(ui.ctx(), AppLanguage::English),
        )
    }

    fn click(&mut self, label: &str) -> Vec<egui::OutputCommand> {
        let mut output = self.frame(vec![]);
        for _ in 0..12 {
            if theme::review_text(&output.shapes)
                .iter()
                .any(|(rect, text, clip)| text == label && clip.contains_rect(*rect))
            {
                break;
            }
            output.drop_without_applying_deltas();
            output = self.frame(vec![
                egui::Event::PointerMoved(egui::pos2(600.0, 600.0)),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, -100.0),
                    phase: egui::TouchPhase::Move,
                    modifiers: Default::default(),
                },
            ]);
            output.drop_without_applying_deltas();
            for _ in 0..60 {
                self.frame(vec![]).drop_without_applying_deltas();
            }
            output = self.frame(vec![]);
        }
        let position = theme::review_text(&output.shapes)
            .iter()
            .find(|(rect, text, clip)| text == label && clip.contains_rect(*rect))
            .unwrap_or_else(|| panic!("Control not visible: {label}"))
            .0
            .center();
        output.drop_without_applying_deltas();
        self.frame(vec![egui::Event::PointerMoved(position)])
            .drop_without_applying_deltas();
        self.frame(vec![egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Default::default(),
        }])
        .drop_without_applying_deltas();
        let mut output = self.frame(vec![egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Default::default(),
        }]);
        let commands = std::mem::take(&mut output.platform_output.commands);
        output.drop_without_applying_deltas();
        commands
    }
}

#[test]
fn about_clicks_dispatch_browser_clipboard_and_close_actions() {
    let mut harness = InteractionHarness::new();
    for (label, expected) in [
        ("ez4cywa / ModelMergerGUI", PROJECT_URL),
        ("Download releases", RELEASES_URL),
        ("Report an issue", ISSUES_URL),
    ] {
        let commands = harness.click(label);
        assert!(commands.iter().any(|command| matches!(command, egui::OutputCommand::OpenUrl(url) if url.url == expected)), "{label}: missing browser command");
        assert!(harness.dialog.is_open());
    }
    let commands = harness.click("Copy project link");
    assert!(commands.iter().any(
        |command| matches!(command, egui::OutputCommand::CopyText(text) if text == PROJECT_URL)
    ));
    assert!(harness.dialog.copied);
    harness.click("Credits and licenses");
    for _ in 0..3 {
        harness.frame(vec![]).drop_without_applying_deltas();
    }
    for (label, expected) in [
        ("Upstream project", UPSTREAM_URL),
        ("MIT License", LICENSE_URL),
        ("Third-party notices", NOTICES_URL),
    ] {
        assert!(harness.click(label).iter().any(|command| matches!(command, egui::OutputCommand::OpenUrl(url) if url.url == expected)), "{label}: missing browser command");
    }
    harness.click("Close");
    assert!(!harness.dialog.is_open());
}

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
