use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, FontId, TextStyle};
use model_merger_app_core::AppLanguage;
use std::path::PathBuf;
use std::sync::Arc;

const MISANS_MEDIUM: &[u8] = include_bytes!("../assets/fonts/MiSans-Medium.ttf");

pub const BACKGROUND: Color32 = Color32::from_rgb(248, 250, 252);
pub const PANEL: Color32 = Color32::WHITE;
pub const FOREGROUND: Color32 = Color32::from_rgb(15, 23, 42);
pub const SECONDARY: Color32 = Color32::from_rgb(71, 85, 105);
pub const BORDER: Color32 = Color32::from_rgb(203, 213, 225);
pub const PROGRESS_TRACK: Color32 = BORDER;
pub const PRIMARY: Color32 = Color32::from_rgb(37, 99, 235);
pub const ON_PRIMARY: Color32 = Color32::WHITE;
pub const DESTRUCTIVE: Color32 = Color32::from_rgb(220, 38, 38);
pub const DISABLED_ALPHA: f32 = 0.85;

pub fn configure(context: &egui::Context, language: AppLanguage) {
    context.set_theme(egui::ThemePreference::Light);
    install_fonts(context, language);
    let mut style = (*context.style_of(egui::Theme::Light)).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(14.0, 10.0);
    style.spacing.interact_size.y = 44.0;
    style.visuals = egui::Visuals::light();
    style.visuals.panel_fill = BACKGROUND;
    style.visuals.window_fill = PANEL;
    style.visuals.extreme_bg_color = PANEL;
    style.visuals.override_text_color = Some(FOREGROUND);
    style.visuals.selection.bg_fill = PRIMARY;
    style.visuals.disabled_alpha = DISABLED_ALPHA;
    style.visuals.widgets.noninteractive.bg_stroke.color = BORDER;
    style.visuals.widgets.inactive.bg_stroke.color = BORDER;
    style.visuals.widgets.hovered.bg_stroke.color = PRIMARY;
    style.visuals.widgets.active.bg_stroke.color = PRIMARY;
    style.text_styles = [
        (TextStyle::Heading, FontId::proportional(24.0)),
        (TextStyle::Body, FontId::proportional(15.0)),
        (TextStyle::Button, FontId::proportional(15.0)),
        (TextStyle::Small, FontId::proportional(13.0)),
        (TextStyle::Monospace, FontId::monospace(14.0)),
    ]
    .into();
    context.set_style_of(egui::Theme::Light, style);
}

fn install_fonts(context: &egui::Context, language: AppLanguage) {
    let mut definitions = FontDefinitions::default();
    let mut installed = Vec::new();
    if let Some(bytes) = load_font(&segoe_candidates()) {
        definitions
            .font_data
            .insert("Segoe UI".into(), Arc::new(FontData::from_owned(bytes)));
        installed.push("Segoe UI".to_owned());
    }
    definitions.font_data.insert(
        "MiSans Medium".into(),
        Arc::new(FontData::from_static(MISANS_MEDIUM)),
    );
    if language == AppLanguage::ChineseSimplified {
        installed.insert(0, "MiSans Medium".to_owned());
    } else {
        installed.push("MiSans Medium".to_owned());
    }
    if let Some(family) = definitions.families.get_mut(&FontFamily::Proportional) {
        for font in installed.into_iter().rev() {
            family.insert(0, font);
        }
    }
    context.set_fonts(definitions);
}

fn load_font(candidates: &[PathBuf]) -> Option<Vec<u8>> {
    candidates.iter().find_map(|path| std::fs::read(path).ok())
}

fn segoe_candidates() -> Vec<PathBuf> {
    std::env::var_os("WINDIR")
        .map(PathBuf::from)
        .into_iter()
        .map(|path| path.join("Fonts").join("segoeui.ttf"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn luminance(color: Color32) -> f32 {
        let channel = |value: u8| {
            let value = f32::from(value) / 255.0;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(color.r()) + 0.7152 * channel(color.g()) + 0.0722 * channel(color.b())
    }

    #[test]
    fn primary_button_text_meets_wcag_aa_contrast() {
        let lighter = luminance(ON_PRIMARY);
        let darker = luminance(PRIMARY);
        let ratio = (lighter + 0.05) / (darker + 0.05);

        assert!(ratio >= 4.5, "primary contrast was {ratio:.2}:1");
    }

    #[test]
    fn disabled_body_text_remains_legible_on_the_background() {
        let blend = |foreground: u8, background: u8| {
            (f32::from(foreground) * DISABLED_ALPHA
                + f32::from(background) * (1.0 - DISABLED_ALPHA))
                .round() as u8
        };
        let disabled = Color32::from_rgb(
            blend(FOREGROUND.r(), BACKGROUND.r()),
            blend(FOREGROUND.g(), BACKGROUND.g()),
            blend(FOREGROUND.b(), BACKGROUND.b()),
        );
        let ratio = (luminance(BACKGROUND) + 0.05) / (luminance(disabled) + 0.05);

        assert!(ratio >= 4.5, "disabled contrast was {ratio:.2}:1");
    }
}
