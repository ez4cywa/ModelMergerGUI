use eframe::egui::{
    self, Color32, FontData, FontDefinitions, FontFamily, FontId, Stroke, TextStyle,
};
use model_merger_app_core::AppLanguage;
use std::path::PathBuf;
use std::sync::Arc;

const MISANS_MEDIUM: &[u8] = include_bytes!("../assets/fonts/MiSans-Medium.ttf");

pub const DISABLED_ALPHA: f32 = 0.78;

#[derive(Clone, Copy)]
pub struct Palette {
    pub background: Color32,
    pub panel: Color32,
    pub toolbar: Color32,
    pub sidebar: Color32,
    pub surface: Color32,
    pub foreground: Color32,
    pub secondary: Color32,
    pub border: Color32,
    pub border_strong: Color32,
    pub control: Color32,
    pub hover: Color32,
    pub progress_track: Color32,
    pub primary: Color32,
    pub on_primary: Color32,
    pub destructive: Color32,
    pub notice_info: Color32,
    pub notice_error: Color32,
    pub preview_canvas: Color32,
}

pub const LIGHT: Palette = Palette {
    background: Color32::from_rgb(245, 245, 247),
    panel: Color32::WHITE,
    toolbar: Color32::from_rgb(247, 247, 249),
    sidebar: Color32::from_rgb(246, 246, 248),
    surface: Color32::from_rgb(250, 250, 251),
    foreground: Color32::from_rgb(29, 29, 31),
    secondary: Color32::from_rgb(84, 84, 88),
    border: Color32::from_rgb(209, 209, 214),
    border_strong: Color32::from_rgb(174, 174, 178),
    control: Color32::from_rgb(233, 233, 236),
    hover: Color32::from_rgb(223, 224, 228),
    progress_track: Color32::from_rgb(217, 217, 222),
    primary: Color32::from_rgb(0, 102, 204),
    on_primary: Color32::WHITE,
    destructive: Color32::from_rgb(198, 40, 40),
    notice_info: Color32::from_rgb(235, 245, 255),
    notice_error: Color32::from_rgb(255, 240, 240),
    preview_canvas: Color32::from_rgb(235, 237, 240),
};

pub const DARK: Palette = Palette {
    background: Color32::from_rgb(28, 28, 30),
    panel: Color32::from_rgb(36, 36, 38),
    toolbar: Color32::from_rgb(42, 42, 44),
    sidebar: Color32::from_rgb(44, 44, 46),
    surface: Color32::from_rgb(50, 50, 52),
    foreground: Color32::from_rgb(245, 245, 247),
    secondary: Color32::from_rgb(190, 190, 196),
    border: Color32::from_rgb(72, 72, 74),
    border_strong: Color32::from_rgb(99, 99, 102),
    control: Color32::from_rgb(58, 58, 60),
    hover: Color32::from_rgb(72, 72, 74),
    progress_track: Color32::from_rgb(76, 76, 78),
    primary: Color32::from_rgb(10, 114, 232),
    on_primary: Color32::WHITE,
    destructive: Color32::from_rgb(255, 105, 97),
    notice_info: Color32::from_rgb(24, 58, 91),
    notice_error: Color32::from_rgb(74, 32, 32),
    preview_canvas: Color32::from_rgb(32, 33, 36),
};

pub fn palette(ui: &egui::Ui) -> Palette {
    if ui.visuals().dark_mode { DARK } else { LIGHT }
}

pub fn configure(context: &egui::Context, language: AppLanguage) {
    context.set_theme(egui::ThemePreference::System);
    install_fonts(context, language);
    configure_style(context, egui::Theme::Light, LIGHT, egui::Visuals::light());
    configure_style(context, egui::Theme::Dark, DARK, egui::Visuals::dark());
}

fn configure_style(
    context: &egui::Context,
    theme: egui::Theme,
    palette: Palette,
    mut visuals: egui::Visuals,
) {
    let mut style = (*context.style_of(theme)).clone();
    style.animation_time = 0.18;
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(14.0, 8.0);
    style.spacing.interact_size.y = 36.0;

    visuals.panel_fill = palette.background;
    visuals.window_fill = palette.panel;
    visuals.extreme_bg_color = palette.surface;
    visuals.override_text_color = Some(palette.foreground);
    visuals.selection.bg_fill = palette.primary;
    visuals.selection.stroke = Stroke::new(1.0, palette.on_primary);
    visuals.disabled_alpha = DISABLED_ALPHA;
    visuals.widgets.noninteractive.bg_fill = palette.panel;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, palette.border);
    visuals.widgets.noninteractive.corner_radius = 7.0.into();
    visuals.widgets.inactive.bg_fill = palette.control;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, palette.border);
    visuals.widgets.inactive.corner_radius = 7.0.into();
    visuals.widgets.hovered.bg_fill = palette.hover;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, palette.border_strong);
    visuals.widgets.hovered.corner_radius = 7.0.into();
    visuals.widgets.active.bg_fill = palette.primary;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, palette.primary);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, palette.on_primary);
    visuals.widgets.active.corner_radius = 7.0.into();
    visuals.widgets.open.bg_fill = palette.hover;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, palette.border_strong);
    visuals.widgets.open.corner_radius = 7.0.into();
    visuals.window_corner_radius = 12.0.into();
    visuals.menu_corner_radius = 9.0.into();
    visuals.window_shadow = egui::epaint::Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: Color32::from_black_alpha(if visuals.dark_mode { 96 } else { 38 }),
    };
    visuals.window_stroke = Stroke::new(1.0, palette.border);
    visuals.popup_shadow = egui::epaint::Shadow {
        offset: [0, 6],
        blur: 18,
        spread: 0,
        color: Color32::from_black_alpha(if visuals.dark_mode { 104 } else { 34 }),
    };

    style.visuals = visuals;
    style.text_styles = [
        (TextStyle::Heading, FontId::proportional(24.0)),
        (TextStyle::Body, FontId::proportional(15.0)),
        (TextStyle::Button, FontId::proportional(15.0)),
        (TextStyle::Small, FontId::proportional(13.0)),
        (TextStyle::Monospace, FontId::monospace(14.0)),
    ]
    .into();
    context.set_style_of(theme, style);
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

    fn contrast(left: Color32, right: Color32) -> f32 {
        let left = luminance(left);
        let right = luminance(right);
        (left.max(right) + 0.05) / (left.min(right) + 0.05)
    }

    fn blended(foreground: Color32, background: Color32, alpha: f32) -> Color32 {
        let channel = |foreground: u8, background: u8| {
            (f32::from(foreground) * alpha + f32::from(background) * (1.0 - alpha)).round() as u8
        };
        Color32::from_rgb(
            channel(foreground.r(), background.r()),
            channel(foreground.g(), background.g()),
            channel(foreground.b(), background.b()),
        )
    }

    #[test]
    fn semantic_text_colors_meet_wcag_aa_in_both_themes() {
        for palette in [LIGHT, DARK] {
            assert!(contrast(palette.foreground, palette.background) >= 4.5);
            assert!(contrast(palette.secondary, palette.background) >= 4.5);
            assert!(contrast(palette.on_primary, palette.primary) >= 4.5);
        }
    }

    #[test]
    fn disabled_body_text_remains_legible_in_both_themes() {
        for palette in [LIGHT, DARK] {
            let disabled = blended(palette.foreground, palette.background, DISABLED_ALPHA);
            assert!(
                contrast(disabled, palette.background) >= 4.5,
                "disabled contrast was {:.2}:1",
                contrast(disabled, palette.background)
            );
        }
    }
}
