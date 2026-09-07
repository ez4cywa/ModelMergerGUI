use crate::theme;
#[cfg(test)]
#[path = "about_tests.rs"]
mod tests;
use eframe::egui::{self, RichText};
use model_merger_app_core::{AppLanguage, Catalog, TextKey};

pub const PROJECT_URL: &str = "https://github.com/ez4cywa/ModelMergerGUI";
const RELEASES_URL: &str = "https://github.com/ez4cywa/ModelMergerGUI/releases/latest";
const ISSUES_URL: &str = "https://github.com/ez4cywa/ModelMergerGUI/issues";
const UPSTREAM_URL: &str = "https://github.com/echo000/ModelMerger";
const LICENSE_URL: &str = "https://github.com/ez4cywa/ModelMergerGUI/blob/main/LICENSE";
const NOTICES_URL: &str =
    "https://github.com/ez4cywa/ModelMergerGUI/blob/main/THIRD-PARTY-NOTICES.md";

#[derive(Default)]
pub struct AboutDialog {
    open: bool,
    copied: bool,
    icon: Option<egui::TextureHandle>,
}

fn text(language: AppLanguage, variants: [&'static str; 5]) -> &'static str {
    variants[language as usize]
}

impl AboutDialog {
    pub fn open(&mut self) {
        self.open = true;
        self.copied = false;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn show(&mut self, context: &egui::Context, language: AppLanguage) {
        if !self.open {
            return;
        }
        let catalog = Catalog::new(language);
        if self.icon.is_none()
            && let Ok(icon) =
                eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png"))
        {
            self.icon = Some(context.load_texture(
                "about-app-icon",
                egui::ColorImage::from_rgba_unmultiplied(
                    [icon.width as usize, icon.height as usize],
                    &icon.rgba,
                ),
                Default::default(),
            ));
        }
        let height = (context.content_rect().height() - 180.0).clamp(180.0, 600.0);
        let mut close = false;
        let response = egui::Modal::new(egui::Id::new("about-dialog")).frame(egui::Frame::popup(&context.style_of(context.theme())).inner_margin(20)).show(context, |ui| {
            ui.set_width((context.content_rect().width() - 96.0).clamp(280.0, 520.0));
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
            let palette = theme::palette(ui);
            ui.heading(catalog.text(TextKey::About));
            ui.add_space(8.0);
            egui::ScrollArea::vertical().id_salt("about-content").auto_shrink([false, true]).max_height(height).show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    if let Some(icon) = &self.icon { ui.image((icon.id(), egui::vec2(64.0,64.0))); }
                        ui.label(RichText::new(catalog.text(TextKey::AppTitle)).size(22.0));
                        ui.label(RichText::new(format!("{} {} · Windows x64", text(language,["版本","Version","Version","Версия","Versión"]), env!("CARGO_PKG_VERSION"))).size(13.0).color(palette.secondary));
                });
                ui.add_space(8.0);
                ui.label(RichText::new(text(language,["Rust 原生桌面应用 · 模型合并、弹匣装填与 3D 预览", "Native Rust desktop app · model merging, ammunition placement and 3D preview", "Application native Rust · fusion de modèles, placement de munitions et aperçu 3D", "Приложение на Rust · объединение моделей, размещение патронов и 3D-просмотр", "Aplicación nativa Rust · combinación de modelos, colocación de munición y vista 3D"])).size(13.0).color(palette.secondary));
                ui.add_space(8.0);
                ui.separator();
                egui::Frame::new().fill(palette.surface).corner_radius(10).inner_margin(16).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new("GitHub").size(17.0));
                ui.hyperlink_to("ez4cywa / ModelMergerGUI", PROJECT_URL).on_hover_text(PROJECT_URL);
                ui.horizontal_wrapped(|ui| {
                    link_button(ui, text(language,["下载发布版","Download releases","Téléchargements","Скачать выпуск","Descargar versiones"]),RELEASES_URL);
                    link_button(ui, text(language,["问题反馈","Report an issue","Signaler un problème","Сообщить о проблеме","Informar de un problema"]),ISSUES_URL);
                });
                ui.horizontal_wrapped(|ui| {
                if ui.add(egui::Button::new(text(language,["复制项目链接","Copy project link","Copier le lien du projet","Скопировать ссылку","Copiar enlace del proyecto"])).min_size(egui::vec2(0.0,40.0))).clicked() {
                    context.copy_text(PROJECT_URL.to_owned()); self.copied = true;
                }
                if self.copied { ui.label(RichText::new(text(language,["已复制","Copied","Copié","Скопировано","Copiado"])).size(13.0).color(palette.secondary)); }
                });
                });
                ui.add_space(8.0);
                ui.separator();
                egui::CollapsingHeader::new(RichText::new(text(language,["致谢与许可","Credits and licenses","Crédits et licences","Авторы и лицензии","Créditos y licencias"])).size(17.0)).id_salt("about-credits").show(ui, |ui| {
                ui.label(text(language,["GUI 维护：ez4cywa", "GUI maintainer: ez4cywa", "Maintenance de l’interface : ez4cywa", "Поддержка интерфейса: ez4cywa", "Mantenimiento de la interfaz: ez4cywa"]));
                ui.label(text(language,["原始 ModelMerger：Philip / Scobalula；Cast 支持：echo000。", "Original ModelMerger: Philip / Scobalula. Cast support: echo000.", "ModelMerger original : Philip / Scobalula. Prise en charge de Cast : echo000.", "Оригинальный ModelMerger: Philip / Scobalula. Поддержка Cast: echo000.", "ModelMerger original: Philip / Scobalula. Soporte de Cast: echo000."]));
                ui.horizontal_wrapped(|ui| {
                    ui.hyperlink_to(text(language,["上游项目","Upstream project","Projet d’origine","Исходный проект","Proyecto original"]),UPSTREAM_URL);
                    ui.hyperlink_to("MIT License",LICENSE_URL);
                    ui.hyperlink_to(text(language,["第三方声明","Third-party notices","Mentions tierces","Сторонние компоненты","Avisos de terceros"]),NOTICES_URL);
                });
                ui.label(RichText::new(text(language,["中文字体采用小米 MiSans，遵循其独立字体许可。", "Chinese text uses Xiaomi MiSans under its separate font license.", "Le texte chinois utilise Xiaomi MiSans sous sa propre licence de police.", "Для китайского текста используется Xiaomi MiSans по отдельной лицензии шрифта.", "El texto chino utiliza Xiaomi MiSans bajo su propia licencia de fuente."])).size(13.0).color(palette.secondary));
                });
            });
            ui.add_space(8.0);
            ui.separator();
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(egui::Button::new(catalog.text(TextKey::Close)).min_size(egui::vec2(92.0,40.0))).clicked() {close=true;}
            });
        });
        if close || response.should_close() {
            self.open = false;
        }
    }
}

fn link_button(ui: &mut egui::Ui, label: &str, url: &str) {
    if ui
        .add(egui::Button::new(label).min_size(egui::vec2(0.0, 40.0)))
        .on_hover_text(url)
        .clicked()
    {
        ui.ctx().open_url(egui::OpenUrl::new_tab(url));
    }
}
