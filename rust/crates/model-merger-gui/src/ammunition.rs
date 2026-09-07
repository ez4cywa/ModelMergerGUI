use crate::theme;
use eframe::egui;
use model_merger_app_core::AppLanguage;
use model_merger_engine::{
    MergeObserver, MergeStage,
    ammunition::{self, Analysis, FillRequest, FillResult},
};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
    },
};

#[derive(Default)]
pub struct AmmoTool {
    pub open: bool,
    weapon: Option<PathBuf>,
    ammo: Option<PathBuf>,
    output: Option<PathBuf>,
    analysis: Option<Analysis>,
    selected: Vec<String>,
    selected_extra: Vec<String>,
    job: Option<Receiver<Result<JobResult, String>>>,
    observer: Arc<Progress>,
    result: Option<FillResult>,
    error: Option<String>,
}

enum JobResult {
    Analysis(Analysis),
    Filled(FillResult),
}

#[derive(Default)]
struct Progress {
    cancelled: AtomicBool,
    fraction: Mutex<f32>,
}
impl MergeObserver for Progress {
    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
    fn on_progress(&self, stage: MergeStage, current: usize, total: usize, _: Option<&str>) {
        let value = match stage {
            MergeStage::Loading => 0.05 + 0.15 * current as f32 / total.max(1) as f32,
            MergeStage::Merging => 0.2 + 0.6 * current as f32 / total.max(1) as f32,
            MergeStage::Saving => 0.85,
            MergeStage::Verifying => 0.95,
            MergeStage::Completed => 1.0,
            _ => 0.0,
        };
        *self.fraction.lock().unwrap() = value;
    }
}

pub fn title(language: AppLanguage) -> &'static str {
    localized(
        language,
        [
            "弹匣装填",
            "Fill magazine",
            "Remplir le chargeur",
            "Заполнить магазин",
            "Llenar cargador",
        ],
    )
}
fn localized(language: AppLanguage, text: [&'static str; 5]) -> &'static str {
    text[language as usize]
}

impl AmmoTool {
    pub fn start(&mut self, weapon: Option<PathBuf>) {
        self.open = true;
        if self.job.is_none()
            && let Some(path) = weapon
        {
            self.set_weapon(path);
        }
    }
    fn set_weapon(&mut self, path: PathBuf) {
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        self.output = Some(path.with_file_name(format!("{stem}_filled.cast")));
        self.weapon = Some(path.clone());
        self.analysis = None;
        self.selected.clear();
        self.selected_extra.clear();
        self.result = None;
        self.error = None;
        self.observer = Arc::new(Progress::default());
        let observer = Arc::clone(&self.observer);
        let (tx, rx) = mpsc::channel();
        self.job = Some(rx);
        std::thread::spawn(move || {
            let _ = tx.send(
                ammunition::inspect(&path, observer.as_ref())
                    .map(JobResult::Analysis)
                    .map_err(|e| e.to_string()),
            );
        });
    }
    fn fill(&mut self) {
        let (Some(weapon), Some(ammunition), Some(output)) =
            (&self.weapon, &self.ammo, &self.output)
        else {
            return;
        };
        let request = FillRequest {
            weapon: weapon.clone(),
            ammunition: ammunition.clone(),
            output: output.clone(),
            magazines: self.selected.clone(),
            extra_slots: self.selected_extra.clone(),
        };
        self.result = None;
        self.error = None;
        self.observer = Arc::new(Progress::default());
        let observer = Arc::clone(&self.observer);
        let (tx, rx) = mpsc::channel();
        self.job = Some(rx);
        std::thread::spawn(move || {
            let _ = tx.send(
                ammunition::fill(request, observer.as_ref())
                    .map(JobResult::Filled)
                    .map_err(|e| e.to_string()),
            );
        });
    }
    pub fn show(&mut self, context: &egui::Context, language: AppLanguage) -> Option<PathBuf> {
        if let Some(rx) = &self.job {
            match rx.try_recv() {
                Ok(result) => {
                    self.job = None;
                    match result {
                        Ok(JobResult::Analysis(analysis)) => {
                            self.selected = analysis
                                .magazines
                                .first()
                                .map(|m| vec![m.name.clone()])
                                .unwrap_or_default();
                            self.analysis = Some(analysis);
                        }
                        Ok(JobResult::Filled(result)) => self.result = Some(result),
                        Err(error) => self.error = Some(error),
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.job = None;
                    self.error = Some("Worker stopped".into());
                }
                Err(mpsc::TryRecvError::Empty) => {
                    context.request_repaint_after(std::time::Duration::from_millis(50))
                }
            }
        }
        if !self.open {
            return None;
        }
        let mut open = self.open;
        let mut preview = None;
        let height = (context.content_rect().height() - 112.0).clamp(280.0, 650.0);
        egui::Window::new(title(language)).open(&mut open).default_width(560.0).max_width((context.content_rect().width() - 48.0).max(280.0)).default_height(height).max_height(height).default_pos(context.content_rect().center() - egui::vec2(280.0, height / 2.0 + 20.0)).resizable(true).show(context, |ui| {
            let p = theme::palette(ui);
            egui::ScrollArea::vertical().id_salt("ammo-content").scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible).max_height((ui.available_height() - 92.0).max(160.0)).auto_shrink([false, true]).show(ui, |ui| {
            ui.label(localized(language, ["按弹匣骨骼放置子弹模型，并另存 CAST。", "Place ammunition at magazine bones and save a new CAST.", "Placer les munitions sur les os du chargeur et enregistrer un nouveau CAST.", "Разместить патроны по костям магазина и сохранить новый CAST.", "Colocar munición en los huesos del cargador y guardar un CAST nuevo."]));
            ui.add_enabled_ui(self.job.is_none(), |ui| {
                let weapon_label = localized(language, ["武器 / 弹匣模型", "Weapon / magazine model", "Modèle d’arme / chargeur", "Модель оружия / магазина", "Modelo de arma / cargador"]);
                if path_row(ui, weapon_label, &self.weapon) && let Some(path) = picker(&self.weapon).pick_file() { self.set_weapon(path); }
                let ammo_label = localized(language, ["子弹模型（tag_ammo）", "Ammunition model (tag_ammo)", "Modèle de munition (tag_ammo)", "Модель патрона (tag_ammo)", "Modelo de munición (tag_ammo)"]);
                if path_row(ui, ammo_label, &self.ammo) && let Some(path) = picker(&self.ammo).pick_file() { self.ammo = Some(path); self.result = None; }
                let output_label = localized(language, ["另存文件", "Save as", "Enregistrer sous", "Сохранить как", "Guardar como"]);
                if path_row(ui, output_label, &self.output) {
                    let mut dialog = picker(&self.output);
                    if let Some(name) = self.output.as_ref().and_then(|p| p.file_name()) { dialog = dialog.set_file_name(name.to_string_lossy()); }
                    if let Some(path) = dialog.save_file() { self.output = Some(path); self.result = None; }
                }
                ui.separator();
                if let Some(analysis) = &self.analysis {
                    if analysis.magazines.is_empty() { ui.label(localized(language, ["未找到受支持的弹匣子弹骨骼。", "No supported magazine ammunition bones found.", "Aucun os de munition de chargeur pris en charge.", "Поддерживаемые кости патронов магазина не найдены.", "No se encontraron huesos de munición compatibles."])); }
                    for group in &analysis.magazines {
                        let mut selected = self.selected.contains(&group.name);
                        let suffix = localized(language, ["骨骼 / 已占用", "bones / occupied", "os / occupés", "костей / занято", "huesos / ocupados"]);
                        if ui.checkbox(&mut selected, format!("{} · {} / {} {suffix}", group.name, group.slots.len(), group.occupied.len())).changed() {
                            if selected { self.selected.push(group.name.clone()); } else { self.selected.retain(|n| n != &group.name); }
                            self.result = None;
                        }
                        ui.label(egui::RichText::new(group.slots.join(", ")).size(13.0).color(p.secondary));
                    }
                    if !analysis.excluded_slots.is_empty() {
                        ui.separator();
                        ui.label(localized(language,["其他子弹骨骼（按需勾选）","Other ammunition bones (optional)","Autres os de munition (facultatif)","Другие кости патронов (по выбору)","Otros huesos de munición (opcionales)"]));
                        for name in &analysis.excluded_slots {
                            let mut selected = self.selected_extra.contains(name);
                            if ui.checkbox(&mut selected, name).changed() {
                                if selected { self.selected_extra.push(name.clone()); } else { self.selected_extra.retain(|n| n != name); }
                                self.result = None;
                            }
                        }
                    }
                    ui.label(egui::RichText::new(localized(language,["默认首个弹匣；动画备用弹匣可能与其重叠。已绑定网格的骨骼会跳过。", "First magazine selected by default; animation variants may overlap. Occupied bones are skipped.", "Premier chargeur par défaut ; les variantes d’animation peuvent se superposer. Os occupés ignorés.", "По умолчанию выбран первый магазин; варианты анимации могут совпадать. Занятые кости пропускаются.", "Se selecciona el primer cargador; variantes animadas pueden solaparse. Se omiten huesos ocupados."])).size(13.0).color(p.secondary));
                }
            });
            if let Some(error) = &self.error { ui.colored_label(p.destructive, format!("{}: {error}",localized(language,["操作失败","Operation failed","Échec","Ошибка","Error"]))); }
            if let Some(result) = &self.result {
                ui.label(format!("{}: {} · {}: {}", localized(language,["已装填","Inserted","Insérés","Добавлено","Insertados"]),result.inserted, localized(language,["已跳过","Skipped","Ignorés","Пропущено","Omitidos"]),result.skipped));
                ui.add(egui::Label::new(result.output.display().to_string()).truncate()).on_hover_text(result.output.display().to_string());
                if ui.button(localized(language,["预览装填模型","Preview filled model","Aperçu du modèle rempli","Просмотр результата","Vista previa del resultado"])).clicked() { preview = Some(result.output.clone()); }
            }
            });
            ui.separator();
            let ready = self.job.is_none() && self.ammo.is_some() && self.output.is_some() && (!self.selected.is_empty() || !self.selected_extra.is_empty()) && self.analysis.is_some();
            ui.horizontal_wrapped(|ui| {
                if ui.add_enabled(ready, egui::Button::new(title(language)).min_size(egui::vec2(140.0,36.0))).clicked() { self.fill(); }
                if self.job.is_some() && ui.button(localized(language,["取消","Cancel","Annuler","Отмена","Cancelar"])).clicked() { self.observer.cancelled.store(true,Ordering::Relaxed); }
            });
            if self.job.is_some() { ui.add(egui::ProgressBar::new(*self.observer.fraction.lock().unwrap()).desired_width(ui.available_width())); }
        });
        self.open = open;
        if !open {
            self.observer.cancelled.store(true, Ordering::Relaxed);
        }
        preview
    }
}

impl Drop for AmmoTool {
    fn drop(&mut self) {
        self.observer.cancelled.store(true, Ordering::Relaxed);
    }
}
fn picker(current: &Option<PathBuf>) -> rfd::FileDialog {
    let mut dialog = rfd::FileDialog::new().add_filter("CAST", &["cast"]);
    if let Some(parent) = current.as_ref().and_then(|p| p.parent()) {
        dialog = dialog.set_directory(parent);
    }
    dialog
}

fn path_row(ui: &mut egui::Ui, label: &str, path: &Option<PathBuf>) -> bool {
    ui.label(label);
    let text = path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "…".into());
    ui.add_sized(
        [ui.available_width(), 36.0],
        egui::Button::new(
            path.as_ref()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| text.clone()),
        )
        .wrap_mode(egui::TextWrapMode::Truncate),
    )
    .on_hover_text(text)
    .clicked()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fill_action_stays_visible_with_long_localized_content() {
        for language in AppLanguage::ALL {
            let context = egui::Context::default();
            theme::configure(&context, language);
            let mut tool = AmmoTool::default();
            tool.start(None);
            tool.analysis = Some(Analysis {
                magazines: vec![ammunition::Magazine {
                    name: "j_mag1".into(),
                    slots: (0..128).map(|n| format!("j_ammo_{n:03}")).collect(),
                    occupied: vec![],
                }],
                excluded_slots: vec!["j_ammo_999".into()],
            });
            for frame in 0..3 {
                let mut output = context.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(900.0, 680.0),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        tool.show(ui.ctx(), language);
                    },
                );
                let labels = theme::review_text(&output.shapes);
                output.textures_delta.clear();
                if frame < 2 {
                    continue;
                }
                let visible_titles = labels
                    .iter()
                    .filter(|(rect, text, clip)| {
                        text == title(language)
                            && rect.bottom() <= 680.0
                            && clip.contains_rect(*rect)
                    })
                    .count();
                assert!(
                    visible_titles >= 2,
                    "{language:?}: window title or fill action hidden"
                );
                output.drop_without_applying_deltas();
            }
        }
    }
}
