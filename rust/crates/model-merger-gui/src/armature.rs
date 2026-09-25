//! Opt-in arm + weapon assembly tool: splices the weapon skeleton onto the
//! arms' `tag_weapon` bone and saves a new CAST.
use crate::theme;
use eframe::egui;
use model_merger_app_core::AppLanguage;
use model_merger_engine::{
    MergeObserver, MergeStage,
    armature::{self, AssembleRequest, AssembleResult},
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
pub struct ArmatureTool {
    pub open: bool,
    arms: Option<PathBuf>,
    weapon: Option<PathBuf>,
    output: Option<PathBuf>,
    bones: Option<Vec<String>>,
    target: Option<String>,
    job: Option<Receiver<Result<JobResult, String>>>,
    observer: Arc<Progress>,
    result: Option<AssembleResult>,
    error: Option<String>,
}

enum JobResult {
    Bones(Vec<String>),
    Assembled(AssembleResult),
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
            "手臂武器拼接",
            "Attach weapon to arms",
            "Fixer l’arme aux bras",
            "Прикрепить оружие к рукам",
            "Acoplar arma a los brazos",
        ],
    )
}
fn localized(language: AppLanguage, text: [&'static str; 5]) -> &'static str {
    text[language as usize]
}

impl ArmatureTool {
    pub fn start(&mut self, weapon: Option<PathBuf>) {
        self.open = true;
        if self.weapon.is_none()
            && let Some(path) = weapon
        {
            self.set_weapon(path);
        }
    }
    fn set_weapon(&mut self, path: PathBuf) {
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let code = model_merger_engine::armature::weapon_code(&stem);
        self.output = Some(path.with_file_name(format!("{code}_viewhands.cast")));
        self.weapon = Some(path);
        self.result = None;
        self.error = None;
    }
    fn set_arms(&mut self, path: PathBuf) {
        self.observer = Arc::new(Progress::default());
        let observer = Arc::clone(&self.observer);
        let job_path = path.clone();
        self.arms = Some(path);
        self.bones = None;
        self.target = None;
        self.result = None;
        self.error = None;
        let (tx, rx) = mpsc::channel();
        self.job = Some(rx);
        std::thread::spawn(move || {
            let _ = tx.send(
                armature::inspect_arms(&job_path, observer.as_ref())
                    .map(JobResult::Bones)
                    .map_err(|e| e.to_string()),
            );
        });
    }
    fn assemble(&mut self) {
        let (Some(arms), Some(weapon), Some(output)) = (&self.arms, &self.weapon, &self.output)
        else {
            return;
        };
        let request = AssembleRequest {
            arms: arms.clone(),
            weapon: weapon.clone(),
            output: output.clone(),
            target_bone: self.target.clone(),
        };
        self.result = None;
        self.error = None;
        self.observer = Arc::new(Progress::default());
        let observer = Arc::clone(&self.observer);
        let (tx, rx) = mpsc::channel();
        self.job = Some(rx);
        std::thread::spawn(move || {
            let _ = tx.send(
                armature::assemble(request, observer.as_ref())
                    .map(JobResult::Assembled)
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
                        Ok(JobResult::Bones(bones)) => self.bones = Some(bones),
                        Ok(JobResult::Assembled(result)) => self.result = Some(result),
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
        let height = (context.content_rect().height() - 112.0).clamp(280.0, 560.0);
        egui::Window::new(title(language)).open(&mut open).default_width(560.0).max_width((context.content_rect().width() - 48.0).max(280.0)).default_height(height).max_height(height).default_pos(context.content_rect().center() - egui::vec2(280.0, height / 2.0 + 20.0)).resizable(true).show(context, |ui| {
            let p = theme::palette(ui);
            egui::ScrollArea::vertical().id_salt("armature-content").max_height((ui.available_height() - 92.0).max(160.0)).auto_shrink([false, true]).show(ui, |ui| {
            ui.label(localized(language, ["把手臂与武器拼接成完整的第一人称模型：武器根骨骼归零挂接到手臂的 tag_weapon，另存新 CAST。", "Assemble the arms and weapon into a first-person model: the weapon root bone is zeroed onto the arms' tag_weapon and saved as a new CAST.", "Assemblez les bras et l’arme en un modèle à la première personne : l’os racine de l’arme est aligné sur le tag_weapon des bras et enregistré dans un nouveau CAST.", "Соберите руки и оружие в модель от первого лица: корневая кость оружия обнуляется на tag_weapon рук и сохраняется в новый CAST.", "Ensambla brazos y arma en un modelo en primera persona: el hueso raíz del arma se alinea con el tag_weapon de los brazos y se guarda en un CAST nuevo."]));
            ui.add_enabled_ui(self.job.is_none(), |ui| {
                let arms_label = localized(language, ["手臂模型（viewhands）", "Arms model (viewhands)", "Modèle de bras (viewhands)", "Модель рук (viewhands)", "Modelo de brazos (viewhands)"]);
                if path_row(ui, arms_label, &self.arms) && let Some(path) = picker(&self.arms).pick_file() { self.set_arms(path); }
                let weapon_label = localized(language, ["武器模型", "Weapon model", "Modèle d’arme", "Модель оружия", "Modelo de arma"]);
                if path_row(ui, weapon_label, &self.weapon) && let Some(path) = picker(&self.weapon).pick_file() { self.set_weapon(path); }
                let output_label = localized(language, ["另存文件", "Save as", "Enregistrer sous", "Сохранить как", "Guardar como"]);
                if path_row(ui, output_label, &self.output) {
                    let mut dialog = picker(&self.output);
                    if let Some(name) = self.output.as_ref().and_then(|p| p.file_name()) { dialog = dialog.set_file_name(name.to_string_lossy()); }
                    if let Some(path) = dialog.save_file() { self.output = Some(path); self.result = None; }
                }
                let auto_label = localized(language, ["自动（tag_weapon）", "Auto (tag_weapon)", "Auto (tag_weapon)", "Авто (tag_weapon)", "Automático (tag_weapon)"]);
                ui.label(localized(language, ["目标骨骼", "Target bone", "Os cible", "Целевая кость", "Hueso de destino"]));
                let selected_display = match (&self.target, &self.bones) {
                    (Some(name), _) => name.clone(),
                    (None, _) => auto_label.to_owned(),
                };
                egui::ComboBox::from_id_salt("armature-target")
                    .selected_text(selected_display)
                    .wrap_mode(egui::TextWrapMode::Truncate)
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(self.target.is_none(), auto_label)
                            .clicked()
                        {
                            self.target = None;
                            self.result = None;
                        }
                        if let Some(bones) = &self.bones {
                            for name in bones {
                                if ui.selectable_label(self.target.as_deref() == Some(name.as_str()), name).clicked() {
                                    self.target = Some(name.clone());
                                    self.result = None;
                                }
                            }
                        }
                    });
                if let Some(error) = &self.error { ui.colored_label(p.destructive, format!("{}: {error}", localized(language, ["操作失败", "Operation failed", "Échec", "Ошибка", "Error"]))); }
                if let Some(result) = &self.result {
                    ui.label(format!("{}: {} · {}: {}", localized(language, ["目标骨骼", "Target bone", "Os cible", "Целевая кость", "Hueso de destino"]), result.target_bone, localized(language, ["已拼接网格", "Attached meshes", "Maillages assemblés", "Присоединено сеток", "Mallas acopladas"]), result.attached_meshes));
                    ui.add(egui::Label::new(result.output.display().to_string()).truncate()).on_hover_text(result.output.display().to_string());
                    if ui.button(localized(language, ["预览拼接模型", "Preview assembled model", "Aperçu du modèle assemblé", "Просмотр результата", "Vista previa del modelo"])).clicked() { preview = Some(result.output.clone()); }
                }
            });
            });
            ui.separator();
            let ready = self.job.is_none() && self.arms.is_some() && self.weapon.is_some() && self.output.is_some();
            ui.horizontal_wrapped(|ui| {
                if ui.add_enabled(ready, egui::Button::new(title(language)).min_size(egui::vec2(140.0, 36.0))).clicked() { self.assemble(); }
                if self.job.is_some() && ui.button(localized(language, ["取消", "Cancel", "Annuler", "Отмена", "Cancelar"])).clicked() { self.observer.cancelled.store(true, Ordering::Relaxed); }
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

impl Drop for ArmatureTool {
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
