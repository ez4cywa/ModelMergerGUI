use crate::preview::PreviewSession;
use crate::{GroupLog, NativeAppState, slot_columns, theme};
use eframe::egui::{self, Color32, RichText, Stroke};
use model_merger_app_core::{
    AddPartResult, AddPartStatus, AppLanguage, Catalog, GroupId, RootMode, SettingsStore,
    TaskError, TaskProgress, TaskScheduler, TaskState, TextKey, WindowBounds,
};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::Duration;

const SLOT_COUNT: usize = 15;
const SETTINGS_PANE_WIDTH: f32 = 300.0;
const MINIMUM_SLOT_CARD_WIDTH: f32 = 136.0;

enum UiNotice {
    Key(TextKey),
    AddPart(AddPartStatus),
    SettingsSaveFailed(String),
}

impl UiNotice {
    fn text(&self, catalog: Catalog) -> String {
        match self {
            Self::Key(key) => catalog.text(*key).to_owned(),
            Self::AddPart(status) => add_part_error(catalog, *status).to_owned(),
            Self::SettingsSaveFailed(detail) => {
                format!("{}: {detail}", catalog.text(TextKey::SettingsSaveFailed))
            }
        }
    }
}

pub struct NativeApp {
    state: NativeAppState,
    store: SettingsStore,
    scheduler: TaskScheduler,
    notice: Option<UiNotice>,
    configured_language: AppLanguage,
    previews: Vec<PreviewSession>,
    next_preview_id: u64,
    drop_target: Option<usize>,
    pending_group_delete: Option<usize>,
    pending_overwrites: VecDeque<GroupId>,
}

impl NativeApp {
    pub fn new(context: &egui::Context) -> Self {
        let store = default_settings_store();
        let state = NativeAppState::new(store.load());
        let configured_language = state.language();
        theme::configure(context, configured_language);
        context.send_viewport_cmd(egui::ViewportCommand::Title(
            Catalog::new(configured_language)
                .text(TextKey::AppTitle)
                .to_owned(),
        ));
        let mut app = Self {
            state,
            store,
            scheduler: TaskScheduler::native(2).expect("two native merge workers are valid"),
            notice: None,
            configured_language,
            previews: Vec::new(),
            next_preview_id: 1,
            drop_target: None,
            pending_group_delete: None,
            pending_overwrites: VecDeque::new(),
        };
        for path in std::env::args_os().skip(1).map(PathBuf::from).take(5) {
            if path.is_file()
                && path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("cast"))
            {
                app.open_preview(&path);
            }
        }
        app
    }

    fn catalog(&self) -> Catalog {
        Catalog::new(self.state.language())
    }

    fn save_settings(&mut self) {
        self.notice = Some(match self.store.save(self.state.settings()) {
            Ok(()) => UiNotice::Key(TextKey::SettingsSaved),
            Err(error) => UiNotice::SettingsSaveFailed(error.to_string()),
        });
    }

    fn restore_defaults(&mut self) {
        self.state.reset_defaults();
        self.notice = Some(UiNotice::Key(TextKey::DefaultsRestored));
    }

    fn add_part_dialog(&mut self, group_index: usize, replace_index: Option<usize>) {
        let catalog = self.catalog();
        let recent = self.state.groups()[group_index]
            .plan
            .state()
            .recent_input_directory;
        let mut dialog = rfd::FileDialog::new()
            .set_title(catalog.text(TextKey::SelectCast))
            .add_filter("Cast", &["cast"]);
        if let Some(directory) = recent {
            dialog = dialog.set_directory(directory);
        }
        let results = if let Some(part_index) = replace_index {
            let Some(path) = dialog.pick_file() else {
                return;
            };
            vec![self.state.replace_part(group_index, part_index, path)]
        } else {
            let Some(paths) = dialog.pick_files() else {
                return;
            };
            self.state.add_parts(group_index, paths)
        };
        if let Some(status) = latest_add_part_error(&results) {
            self.notice = Some(UiNotice::AddPart(status));
        }
    }

    fn choose_output_directory(&mut self, group_index: usize) {
        let catalog = self.catalog();
        let current = self.state.groups()[group_index]
            .plan
            .state()
            .output_directory;
        let mut dialog = rfd::FileDialog::new().set_title(catalog.text(TextKey::SelectOutput));
        if current.is_dir() {
            dialog = dialog.set_directory(current);
        }
        let Some(directory) = dialog.pick_folder() else {
            return;
        };
        self.state.groups_mut()[group_index]
            .plan
            .choose_output_directory(&directory);
        self.state.remember_output_directory(directory);
    }

    fn open_preview(&mut self, path: &Path) {
        self.previews
            .push(PreviewSession::load(self.next_preview_id, path));
        self.next_preview_id += 1;
    }

    fn schedule_group(&mut self, group_index: usize, overwrite: bool) {
        let Some(request) = self.state.groups()[group_index]
            .plan
            .create_request(overwrite)
        else {
            self.notice = Some(UiNotice::Key(TextKey::NeedTwoToFifteen));
            return;
        };
        match self.scheduler.schedule(request) {
            Ok(task_id) => {
                let task_snapshot = self.scheduler.snapshot(task_id);
                self.state.start_task(group_index, task_id, task_snapshot);
            }
            Err(_error) => {
                self.notice = Some(UiNotice::Key(TextKey::MergeFailed));
            }
        }
    }

    fn refresh_tasks(&mut self) {
        let mut overwrite_candidates = Vec::new();
        for group_index in 0..self.state.groups().len() {
            let Some(task_id) = self.state.groups()[group_index].task_id() else {
                continue;
            };
            let Some(snapshot) = self.scheduler.snapshot(task_id) else {
                continue;
            };
            if let Some(completion) = self.state.apply_task_snapshot(group_index, snapshot)
                && completion.requires_overwrite_confirmation
            {
                overwrite_candidates.push(completion.group_id);
            }
        }
        for group_id in overwrite_candidates {
            if !self.pending_overwrites.contains(&group_id) {
                self.pending_overwrites.push_back(group_id);
            }
        }
    }

    fn top_bar(&mut self, root: &mut egui::Ui) {
        let catalog = self.catalog();
        let can_restore_defaults = self
            .state
            .groups()
            .iter()
            .all(|group| group.task_id().is_none());
        egui::Panel::top("top-command-bar")
            .frame(panel_frame().inner_margin(egui::Margin::symmetric(28, 16)))
            .show(root, |ui| {
                if ui.available_width() < 1050.0 {
                    ui.vertical(|ui| {
                        top_title(ui, catalog);
                        ui.horizontal_wrapped(|ui| {
                            ui.label(catalog.text(TextKey::Language));
                            let mut language = self.state.language();
                            language_selector(ui, &mut language);
                            if sized_button(ui, catalog.text(TextKey::NewGroup)).clicked() {
                                self.state.add_group();
                            }
                            if sized_button(ui, catalog.text(TextKey::SaveSettings)).clicked() {
                                self.save_settings();
                            }
                            if ui
                                .add_enabled(
                                    can_restore_defaults,
                                    egui::Button::new(catalog.text(TextKey::RestoreDefaults))
                                        .min_size(egui::vec2(80.0, 44.0)),
                                )
                                .clicked()
                            {
                                self.restore_defaults();
                            }
                            if language != self.state.language() {
                                self.state.set_language(language);
                            }
                        });
                    });
                } else {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| top_title(ui, catalog));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add_enabled(
                                    can_restore_defaults,
                                    egui::Button::new(catalog.text(TextKey::RestoreDefaults))
                                        .min_size(egui::vec2(80.0, 44.0)),
                                )
                                .clicked()
                            {
                                self.restore_defaults();
                            }
                            if sized_button(ui, catalog.text(TextKey::SaveSettings)).clicked() {
                                self.save_settings();
                            }
                            if sized_button(ui, catalog.text(TextKey::NewGroup)).clicked() {
                                self.state.add_group();
                            }
                            let mut language = self.state.language();
                            language_selector(ui, &mut language);
                            ui.label(catalog.text(TextKey::Language));
                            if language != self.state.language() {
                                self.state.set_language(language);
                            }
                        });
                    });
                }
            });
    }

    fn bottom_bar(&mut self, root: &mut egui::Ui) {
        let catalog = self.catalog();
        let ready = self
            .state
            .groups()
            .iter()
            .filter(|group| group.plan.state().is_ready && group.task_id().is_none())
            .count();
        egui::Panel::bottom("bottom-action-bar")
            .frame(panel_frame().inner_margin(egui::Margin::symmetric(28, 12)))
            .show(root, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.vertical(|ui| {
                        ui.label(format!(
                            "{} · {}",
                            catalog.text(TextKey::Concurrency),
                            ready
                        ));
                        ui.label(
                            RichText::new(catalog.text(TextKey::Attribution))
                                .size(13.0)
                                .color(theme::SECONDARY),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let merge_text = RichText::new(catalog.text(TextKey::MergeAllReady)).color(
                            if ready > 0 {
                                theme::ON_PRIMARY
                            } else {
                                theme::FOREGROUND
                            },
                        );
                        let mut merge_button =
                            egui::Button::new(merge_text).min_size(egui::vec2(200.0, 44.0));
                        if ready > 0 {
                            merge_button = merge_button.fill(theme::PRIMARY);
                        }
                        let merge = ui.add_enabled(ready > 0, merge_button);
                        if merge.clicked() {
                            let indices: Vec<_> = self
                                .state
                                .groups()
                                .iter()
                                .enumerate()
                                .filter_map(|(index, group)| {
                                    (group.plan.state().is_ready && group.task_id().is_none())
                                        .then_some(index)
                                })
                                .collect();
                            for index in indices {
                                self.schedule_group(index, false);
                            }
                        }
                        if sized_button(ui, catalog.text(TextKey::CancelAll)).clicked() {
                            for group in self.state.groups() {
                                if let Some(task_id) = group.task_id() {
                                    self.scheduler.cancel(task_id);
                                }
                            }
                        }
                        let mut remember = self.state.settings().remember_output_directory;
                        if ui
                            .checkbox(&mut remember, catalog.text(TextKey::RememberOutput))
                            .changed()
                        {
                            self.state.set_remember_output(remember);
                        }
                    });
                });
            });
    }

    fn central_workspace(&mut self, root: &mut egui::Ui) {
        self.drop_target = None;
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(theme::BACKGROUND)
                    .inner_margin(egui::Margin::same(28)),
            )
            .show(root, |ui| {
                if let Some(notice) = &self.notice {
                    let notice = notice.text(self.catalog());
                    let close_label = self.catalog().text(TextKey::Close);
                    egui::Frame::new()
                        .fill(Color32::from_rgb(239, 246, 255))
                        .stroke(Stroke::new(1.0, theme::PRIMARY))
                        .corner_radius(6.0)
                        .inner_margin(egui::Margin::same(12))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(notice);
                                if ui.button(close_label).clicked() {
                                    self.notice = None;
                                }
                            });
                        });
                    ui.add_space(8.0);
                }
                let workspace_width = visible_available_width(ui);
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_width(workspace_width);
                        let count = self.state.groups().len();
                        for group_index in 0..count {
                            self.group_card(ui, group_index);
                            ui.add_space(16.0);
                        }
                    });
                if let Some(index) = self.pending_group_delete.take() {
                    self.state.delete_group(index);
                }
            });
    }

    fn group_card(&mut self, ui: &mut egui::Ui, group_index: usize) {
        let catalog = self.catalog();
        let group_snapshot = self.state.groups()[group_index].plan.state();
        let collapsed = self.state.groups()[group_index].collapsed;
        let task_id = self.state.groups()[group_index].task_id();
        let group_inner_width = (visible_available_width(ui) - 32.0).max(280.0);
        let response = panel_frame()
            .inner_margin(egui::Margin::same(16))
            .show(ui, |ui| {
                ui.set_width(group_inner_width);
                ui.horizontal_wrapped(|ui| {
                    if sized_button(
                        ui,
                        &format!("{} {}", catalog.text(TextKey::Group), group_index + 1),
                    )
                    .clicked()
                    {
                        self.state.set_collapsed(group_index, !collapsed);
                    }
                    ui.label(
                        RichText::new(format!(
                            "{} / {SLOT_COUNT}",
                            group_snapshot.part_files.len()
                        ))
                        .color(theme::PRIMARY),
                    );
                    ui.label(if group_snapshot.is_ready {
                        catalog.text(TextKey::Ready)
                    } else {
                        catalog.text(TextKey::NeedTwoToFifteen)
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let can_delete = self.state.groups().len() > 1 && task_id.is_none();
                        if ui
                            .add_enabled(
                                can_delete,
                                egui::Button::new(
                                    RichText::new(catalog.text(TextKey::DeleteGroup)).color(
                                        if can_delete {
                                            theme::DESTRUCTIVE
                                        } else {
                                            theme::FOREGROUND
                                        },
                                    ),
                                )
                                .min_size(egui::vec2(96.0, 44.0)),
                            )
                            .clicked()
                        {
                            self.pending_group_delete = Some(group_index);
                        }
                        if let Some(task_id) = task_id {
                            if sized_button(ui, catalog.text(TextKey::Cancel)).clicked() {
                                self.scheduler.cancel(task_id);
                            }
                        } else if ui
                            .add_enabled(
                                group_snapshot.is_ready,
                                egui::Button::new(catalog.text(TextKey::StartGroup))
                                    .min_size(egui::vec2(112.0, 44.0)),
                            )
                            .clicked()
                        {
                            self.schedule_group(group_index, false);
                        }
                    });
                });
                if collapsed {
                    return;
                }
                ui.separator();
                ui.horizontal(|ui| {
                    let width = ui.available_width();
                    let settings_width = SETTINGS_PANE_WIDTH;
                    let slots_width = (width - settings_width - 8.0).max(280.0);
                    let columns = slot_columns(slots_width);
                    let rows = SLOT_COUNT.div_ceil(columns);
                    let panel_height = 128.0 + rows as f32 * 116.0;
                    ui.allocate_ui_with_layout(
                        egui::vec2(slots_width, panel_height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.set_width(slots_width);
                            self.parts_panel(ui, group_index, task_id.is_some(), slots_width);
                        },
                    );
                    ui.allocate_ui_with_layout(
                        egui::vec2(settings_width, panel_height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.set_width(settings_width);
                            self.settings_panel(ui, group_index, task_id.is_some());
                        },
                    );
                });
            })
            .response;
        if task_id.is_none()
            && ui
                .ctx()
                .input(|input| input.pointer.hover_pos())
                .is_some_and(|position| response.rect.contains(position))
        {
            self.drop_target = Some(group_index);
        }
    }

    fn parts_panel(
        &mut self,
        ui: &mut egui::Ui,
        group_index: usize,
        locked: bool,
        panel_width: f32,
    ) {
        let catalog = self.catalog();
        let snapshot = self.state.groups()[group_index].plan.state();
        ui.add_enabled_ui(!locked, |ui| {
            ui.label(RichText::new(catalog.text(TextKey::ModelParts)).size(18.0));
            ui.label(
                RichText::new(catalog.text(TextKey::ModelPartsHint))
                    .size(13.0)
                    .color(theme::SECONDARY),
            );
            ui.horizontal(|ui| {
                if sized_button(ui, catalog.text(TextKey::AddNext)).clicked() {
                    self.add_part_dialog(group_index, None);
                }
                if ui
                    .add_enabled(
                        !snapshot.part_files.is_empty(),
                        egui::Button::new(catalog.text(TextKey::Clear))
                            .min_size(egui::vec2(80.0, 44.0)),
                    )
                    .clicked()
                {
                    self.state.clear_parts(group_index);
                }
            });
            let columns = slot_columns(panel_width);
            let card_width = slot_card_width(panel_width, columns);
            egui::Grid::new(("parts-grid", group_index))
                .num_columns(columns)
                .spacing(egui::vec2(8.0, 8.0))
                .show(ui, |ui| {
                    for slot in 0..SLOT_COUNT {
                        self.slot_card(ui, group_index, slot, &snapshot.part_files, card_width);
                        if (slot + 1) % columns == 0 {
                            ui.end_row();
                        }
                    }
                });
        });
    }

    fn slot_card(
        &mut self,
        ui: &mut egui::Ui,
        group_index: usize,
        slot: usize,
        parts: &[PathBuf],
        card_width: f32,
    ) {
        let catalog = self.catalog();
        let content_width = (card_width - 16.0).max(96.0);
        egui::Frame::new()
            .fill(theme::PANEL)
            .stroke(Stroke::new(1.0, theme::BORDER))
            .corner_radius(6.0)
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                ui.set_width(content_width);
                ui.set_min_height(108.0);
                ui.vertical(|ui| {
                    ui.set_width(content_width);
                    ui.label(RichText::new(format!("{:02}", slot + 1)).size(13.0));
                    if let Some(path) = parts.get(slot) {
                        let file_name = path.file_name().unwrap_or_default().to_string_lossy();
                        ui.add_sized(
                            [content_width, 22.0],
                            truncated_file_name_label(file_name.as_ref()),
                        )
                        .on_hover_text(path.display().to_string());
                        if !path.is_file() {
                            ui.colored_label(
                                theme::DESTRUCTIVE,
                                catalog.text(TextKey::FileMissing),
                            );
                        }
                        ui.horizontal_wrapped(|ui| {
                            if ui
                                .add(
                                    egui::Button::new(catalog.text(TextKey::Preview))
                                        .min_size(egui::vec2(64.0, 44.0)),
                                )
                                .clicked()
                            {
                                self.open_preview(path);
                            }
                            ui.menu_button(catalog.text(TextKey::Actions), |ui| {
                                if ui.button(catalog.text(TextKey::Replace)).clicked() {
                                    self.add_part_dialog(group_index, Some(slot));
                                    ui.close();
                                }
                                if ui.button(catalog.text(TextKey::SetAsRoot)).clicked() {
                                    self.state.set_group_manual_root(group_index, slot);
                                    ui.close();
                                }
                                if ui
                                    .button(
                                        RichText::new(catalog.text(TextKey::Remove))
                                            .color(theme::DESTRUCTIVE),
                                    )
                                    .clicked()
                                {
                                    self.state.remove_part(group_index, slot);
                                    ui.close();
                                }
                            });
                        });
                    } else if ui
                        .add_sized(
                            [content_width, 72.0],
                            egui::Button::new(catalog.text(TextKey::AddPart)),
                        )
                        .clicked()
                    {
                        self.add_part_dialog(group_index, None);
                    }
                });
            });
    }

    fn settings_panel(&mut self, ui: &mut egui::Ui, group_index: usize, locked: bool) {
        let catalog = self.catalog();
        let snapshot = self.state.groups()[group_index].plan.state();
        ui.add_enabled_ui(!locked, |ui| {
            ui.label(RichText::new(catalog.text(TextKey::RootModel)).size(18.0));
            let mut root_mode = snapshot.root_mode;
            ui.horizontal_wrapped(|ui| {
                ui.radio_value(
                    &mut root_mode,
                    RootMode::Automatic,
                    catalog.text(TextKey::Automatic),
                );
                ui.radio_value(
                    &mut root_mode,
                    RootMode::Manual,
                    catalog.text(TextKey::Manual),
                );
            });
            if root_mode != snapshot.root_mode {
                self.state.set_group_root_mode(group_index, root_mode);
            }
            if root_mode == RootMode::Manual && !snapshot.part_files.is_empty() {
                let current = snapshot
                    .manual_root_file
                    .as_ref()
                    .and_then(|root| snapshot.part_files.iter().position(|part| part == root))
                    .unwrap_or(0);
                let mut selected = current;
                egui::ComboBox::from_id_salt(("manual-root", group_index))
                    .selected_text(short_name(&snapshot.part_files[current]))
                    .width(ui.available_width())
                    .truncate()
                    .show_ui(ui, |ui| {
                        for (index, path) in snapshot.part_files.iter().enumerate() {
                            ui.selectable_value(&mut selected, index, short_name(path));
                        }
                    });
                if selected != current {
                    self.state.set_group_manual_root(group_index, selected);
                }
            }
            ui.add_space(8.0);
            ui.label(catalog.text(TextKey::OutputFolder));
            let mut output_directory = snapshot.output_directory.display().to_string();
            ui.horizontal(|ui| {
                let field_width = (ui.available_width() - 88.0).max(96.0);
                ui.add_sized(
                    [field_width, 44.0],
                    egui::TextEdit::singleline(&mut output_directory).interactive(false),
                );
                if sized_button(ui, catalog.text(TextKey::Browse)).clicked() {
                    self.choose_output_directory(group_index);
                }
            });
            ui.label(catalog.text(TextKey::OutputFileName));
            let mut output_name = snapshot.output_file_name;
            let output_name_response = egui::Frame::new()
                .fill(theme::PANEL)
                .stroke(Stroke::new(1.0, theme::BORDER))
                .corner_radius(2.0)
                .inner_margin(egui::Margin::same(1))
                .show(ui, |ui| {
                    ui.add_sized(
                        [ui.available_width(), 42.0],
                        egui::TextEdit::singleline(&mut output_name).frame(egui::Frame::NONE),
                    )
                })
                .inner;
            if output_name_response.changed() {
                self.state.groups_mut()[group_index]
                    .plan
                    .set_output_file_name(output_name);
            }
        });
        ui.add_space(12.0);
        let progress = self.state.groups()[group_index]
            .task_id()
            .and_then(|id| self.scheduler.snapshot(id))
            .or_else(|| self.state.groups()[group_index].task_snapshot().cloned());
        let progress_value = progress
            .as_ref()
            .and_then(|snapshot| snapshot.progress.as_ref())
            .map_or(0.0, |progress| {
                normalized_progress(progress.current, progress.total)
            });
        ui.horizontal(|ui| {
            ui.label(RichText::new(catalog.text(TextKey::GroupStatus)).size(18.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("{:.0}%", progress_value * 100.0)).color(theme::PRIMARY),
                );
            });
        });
        ui.scope(|ui| {
            ui.style_mut().visuals.extreme_bg_color = theme::PROGRESS_TRACK;
            ui.add(
                egui::ProgressBar::new(progress_value)
                    .desired_width(ui.available_width())
                    .desired_height(28.0)
                    .corner_radius(6.0)
                    .fill(theme::PRIMARY),
            );
        });
        let status = progress
            .as_ref()
            .and_then(|snapshot| snapshot.progress.as_ref())
            .map_or_else(
                || task_label(catalog, progress.as_ref().map(|value| value.state)).to_owned(),
                |task_progress| task_progress_message(catalog, task_progress),
            );
        ui.label(status);
        ui.label(RichText::new(catalog.text(TextKey::RunLog)).size(17.0));
        egui::ScrollArea::vertical()
            .max_height(120.0)
            .show(ui, |ui| {
                for entry in self.state.groups()[group_index].log() {
                    let message = match entry {
                        GroupLog::Status(key) => catalog.text(*key).to_owned(),
                        GroupLog::Warning(warning) => merge_warning_message(catalog, warning),
                        GroupLog::Output(path) => catalog
                            .text(TextKey::ProgressCompleted)
                            .replace("{0}", &path.display().to_string()),
                        GroupLog::Error(error) => task_error_message(catalog, error),
                    };
                    ui.label(RichText::new(message).size(13.0).color(theme::SECONDARY));
                }
            });
        if let Some(path) = self.state.groups()[group_index]
            .last_output()
            .map(Path::to_path_buf)
            && path.is_file()
            && ui
                .add_sized(
                    [ui.available_width(), 44.0],
                    egui::Button::new(catalog.text(TextKey::PreviewMerged)),
                )
                .clicked()
        {
            self.open_preview(&path);
        }
    }

    fn preview_windows(&mut self, context: &egui::Context) {
        let catalog = self.catalog();
        for preview in &mut self.previews {
            let viewport_id = egui::ViewportId::from_hash_of(("model-preview", preview.id));
            let title = PreviewSession::path_title(
                Path::new(&preview.title),
                catalog.text(TextKey::Preview),
            );
            context.show_viewport_immediate(
                viewport_id,
                egui::ViewportBuilder::default()
                    .with_title(title)
                    .with_inner_size([900.0, 700.0])
                    .with_min_inner_size([640.0, 480.0]),
                |ui, _class| preview.show(ui, catalog),
            );
        }
        self.previews.retain(|preview| preview.open);
    }

    fn handle_dropped_files(&mut self, context: &egui::Context) {
        let paths: Vec<_> = context.input(|input| {
            input
                .raw
                .dropped_files
                .iter()
                .map(|file| file.path().to_path_buf())
                .collect()
        });
        if paths.is_empty() {
            return;
        }
        let Some(group_index) = self
            .drop_target
            .filter(|index| {
                self.state
                    .groups()
                    .get(*index)
                    .is_some_and(|group| group.task_id().is_none())
            })
            .or_else(|| {
                self.state
                    .groups()
                    .iter()
                    .position(|group| !group.collapsed && group.task_id().is_none())
            })
        else {
            return;
        };
        let results = self.state.add_parts(group_index, paths);
        if let Some(status) = latest_add_part_error(&results) {
            self.notice = Some(UiNotice::AddPart(status));
        }
    }

    fn overwrite_dialog(&mut self, context: &egui::Context) {
        let Some(group_id) = self.pending_overwrites.front().copied() else {
            return;
        };
        let Some(group_index) = self.state.group_index(group_id) else {
            self.pending_overwrites.pop_front();
            context.request_repaint();
            return;
        };
        let catalog = self.catalog();
        egui::Modal::new(egui::Id::new("overwrite-confirmation")).show(context, |ui| {
            ui.set_min_width(360.0);
            ui.label(RichText::new(catalog.text(TextKey::Overwrite)).size(18.0));
            ui.label(catalog.text(TextKey::OverwriteQuestion));
            ui.horizontal(|ui| {
                if sized_button(ui, catalog.text(TextKey::No)).clicked() {
                    self.pending_overwrites.pop_front();
                }
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(catalog.text(TextKey::Yes)).color(theme::ON_PRIMARY),
                        )
                        .min_size(egui::vec2(80.0, 44.0))
                        .fill(theme::PRIMARY),
                    )
                    .clicked()
                {
                    self.pending_overwrites.pop_front();
                    self.schedule_group(group_index, true);
                }
            });
        });
    }
}

impl eframe::App for NativeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let context = ui.ctx().clone();
        if let Some((outer_rect, inner_rect)) =
            context.input(|input| input.viewport().outer_rect.zip(input.viewport().inner_rect))
        {
            self.state
                .update_window_bounds(model_merger_app_core::WindowBounds::new(
                    f64::from(outer_rect.left()),
                    f64::from(outer_rect.top()),
                    f64::from(inner_rect.width()),
                    f64::from(inner_rect.height()),
                ));
        }
        if self.configured_language != self.state.language() {
            self.configured_language = self.state.language();
            theme::configure(&context, self.configured_language);
            context.send_viewport_cmd(egui::ViewportCommand::Title(
                self.catalog().text(TextKey::AppTitle).to_owned(),
            ));
        }
        self.refresh_tasks();
        self.top_bar(ui);
        self.bottom_bar(ui);
        self.central_workspace(ui);
        self.handle_dropped_files(&context);
        self.overwrite_dialog(&context);
        self.preview_windows(&context);
        if self
            .state
            .groups()
            .iter()
            .any(|group| group.task_id().is_some())
        {
            context.request_repaint_after(Duration::from_millis(100));
        }
    }
}

fn panel_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(theme::PANEL)
        .stroke(Stroke::new(1.0, theme::BORDER))
        .corner_radius(8.0)
}

fn sized_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(egui::Button::new(label).min_size(egui::vec2(80.0, 44.0)))
}

fn visible_available_width(ui: &egui::Ui) -> f32 {
    let viewport_right = ui
        .ctx()
        .input(|input| input.viewport().inner_rect.map(|rect| rect.right()))
        .unwrap_or_else(|| ui.ctx().content_rect().right());
    let visible = viewport_right - ui.next_widget_position().x;
    ui.available_width().min(visible).max(0.0)
}

fn slot_card_width(available_width: f32, columns: usize) -> f32 {
    let columns = columns.max(1);
    let spacing = 8.0 * columns.saturating_sub(1) as f32;
    (((available_width - spacing) / columns as f32) / 8.0)
        .floor()
        .mul_add(8.0, 0.0)
        .max(MINIMUM_SLOT_CARD_WIDTH)
}

fn latest_add_part_error(results: &[AddPartResult]) -> Option<AddPartStatus> {
    results
        .iter()
        .rev()
        .find_map(|result| (result.status != AddPartStatus::Added).then_some(result.status))
}

fn normalized_progress(current: usize, total: usize) -> f32 {
    if total == 0 {
        0.0
    } else {
        (current as f32 / total as f32).clamp(0.0, 1.0)
    }
}

fn truncated_file_name_label(file_name: &str) -> egui::Label {
    egui::Label::new(RichText::new(file_name.to_owned()).size(15.0)).truncate()
}

fn top_title(ui: &mut egui::Ui, catalog: Catalog) {
    ui.label(
        RichText::new(catalog.text(TextKey::AppTitle))
            .size(24.0)
            .color(theme::FOREGROUND),
    );
    ui.label(
        RichText::new(catalog.text(TextKey::AppSubtitle))
            .size(15.0)
            .color(theme::SECONDARY),
    );
}

fn language_selector(ui: &mut egui::Ui, language: &mut AppLanguage) {
    egui::ComboBox::from_id_salt("language")
        .selected_text(Catalog::new(*language).language_name())
        .show_ui(ui, |ui| {
            for candidate in AppLanguage::ALL {
                ui.selectable_value(language, candidate, Catalog::new(candidate).language_name());
            }
        });
}

fn add_part_error(catalog: Catalog, status: AddPartStatus) -> &'static str {
    catalog.text(match status {
        AddPartStatus::InvalidPath | AddPartStatus::InvalidIndex => TextKey::AddPartInvalidPath,
        AddPartStatus::Missing => TextKey::AddPartMissing,
        AddPartStatus::UnsupportedFormat => TextKey::AddPartNotCast,
        AddPartStatus::Duplicate => TextKey::AddPartDuplicate,
        AddPartStatus::Full => TextKey::AddPartFull,
        AddPartStatus::Added => TextKey::Completed,
    })
}

fn task_label(catalog: Catalog, state: Option<TaskState>) -> &'static str {
    catalog.text(match state {
        None => TextKey::NeedTwoToFifteen,
        Some(TaskState::Queued) => TextKey::Queued,
        Some(TaskState::Running) => TextKey::Running,
        Some(TaskState::Succeeded) => TextKey::Succeeded,
        Some(TaskState::Failed) => TextKey::Failed,
        Some(TaskState::Cancelled) => TextKey::Cancelled,
    })
}

fn task_progress_message(catalog: Catalog, progress: &TaskProgress) -> String {
    let key = match progress.stage {
        model_merger_engine::MergeStage::Validating => TextKey::ProgressValidating,
        model_merger_engine::MergeStage::Loading => TextKey::ProgressLoading,
        model_merger_engine::MergeStage::SelectingRoot => TextKey::ProgressSelectingRoot,
        model_merger_engine::MergeStage::Merging => TextKey::ProgressMerging,
        model_merger_engine::MergeStage::Saving => TextKey::ProgressSaving,
        model_merger_engine::MergeStage::Verifying => TextKey::ProgressVerifying,
        model_merger_engine::MergeStage::Completed => TextKey::Completed,
    };
    let item = progress.item.as_deref().unwrap_or_default();
    catalog.text(key).replace("{0}", item)
}

fn task_error_message(catalog: Catalog, error: &TaskError) -> String {
    use model_merger_engine::MergeValidationCode;
    match error {
        TaskError::ModelRead { path, message } => {
            return catalog
                .text(TextKey::ModelPartReadError)
                .replace("{0}", &path.display().to_string())
                .replace("{1}", message);
        }
        TaskError::Codec(message) | TaskError::InvalidModel(message) => {
            return catalog
                .text(TextKey::ModelPartReadError)
                .replace("{0}", "Cast")
                .replace("{1}", message);
        }
        TaskError::Io { path, message } => {
            return format!(
                "{}: {}\n{message}",
                catalog.text(TextKey::MergeFailed),
                path.display()
            );
        }
        TaskError::Backend(message) => {
            return format!("{}: {message}", catalog.text(TextKey::MergeFailed));
        }
        TaskError::SchedulerStopped | TaskError::InvalidConcurrency => {
            return format!("{}: {error}", catalog.text(TextKey::MergeFailed));
        }
        _ => {}
    }
    let key = match error {
        TaskError::Cancelled => TextKey::Cancelled,
        TaskError::OutputConflict(_) => TextKey::OutputConflict,
        TaskError::Validation { code, .. } => match code {
            MergeValidationCode::InvalidPartCount => TextKey::ValidationInvalidPartCount,
            MergeValidationCode::InvalidPath => TextKey::ValidationInvalidPath,
            MergeValidationCode::MissingFile => TextKey::ValidationMissingFile,
            MergeValidationCode::UnsupportedExtension => TextKey::ValidationUnsupportedExtension,
            MergeValidationCode::DuplicateFile => TextKey::ValidationDuplicateFile,
            MergeValidationCode::InvalidOutputDirectory => {
                TextKey::ValidationInvalidOutputDirectory
            }
            MergeValidationCode::InvalidOutputFileName => TextKey::ValidationInvalidOutputFileName,
            MergeValidationCode::OutputAlreadyExists => TextKey::ValidationOutputAlreadyExists,
            MergeValidationCode::ManualRootNotSelected => TextKey::ValidationManualRootNotSelected,
        },
        TaskError::Io { .. }
        | TaskError::Codec(_)
        | TaskError::ModelRead { .. }
        | TaskError::InvalidModel(_)
        | TaskError::Backend(_)
        | TaskError::SchedulerStopped
        | TaskError::InvalidConcurrency => unreachable!("handled above"),
    };
    let mut message = catalog.text(key).to_owned();
    let path = match error {
        TaskError::OutputConflict(path) => Some(path),
        TaskError::Validation { path, .. } => path.as_ref(),
        TaskError::Io { path, .. } | TaskError::ModelRead { path, .. } => Some(path),
        _ => None,
    };
    if let Some(path) = path {
        message = message.replace("{0}", &path.display().to_string());
    }
    message
}

fn merge_warning_message(catalog: Catalog, warning: &model_merger_engine::MergeWarning) -> String {
    let key = match warning.code {
        model_merger_engine::MergeWarningCode::NoAttachmentBone => TextKey::WarningNoAttachmentBone,
        model_merger_engine::MergeWarningCode::UnconnectedHierarchy => {
            TextKey::WarningUnconnectedHierarchy
        }
    };
    catalog
        .text(key)
        .replace("{0}", &warning.model_name)
        .replace("{1}", &warning.root_model_name)
}

fn short_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

fn default_settings_store() -> SettingsStore {
    if let Some(path) = std::env::var_os("CAST_MODEL_MERGER_SETTINGS") {
        return SettingsStore::new(path);
    }
    let local = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_default();
    SettingsStore::default_for_local_app_data(&local)
}

pub fn startup_viewport() -> egui::ViewportBuilder {
    let settings = default_settings_store().load();
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1180.0, 860.0])
        .with_min_inner_size([900.0, 680.0]);
    if let Some(bounds) = settings.window_bounds {
        viewport = viewport.with_inner_size([bounds.width as f32, bounds.height as f32]);
        if saved_position_is_visible(bounds) {
            viewport = viewport.with_position([bounds.left as f32, bounds.top as f32]);
        }
    }
    viewport
}

fn bounds_have_visible_area(bounds: WindowBounds, display: WindowBounds) -> bool {
    let left = bounds.left.max(display.left);
    let top = bounds.top.max(display.top);
    let right = (bounds.left + bounds.width).min(display.left + display.width);
    let bottom = (bounds.top + bounds.height).min(display.top + display.height);
    right - left >= 80.0 && bottom - top >= 80.0
}

#[cfg(windows)]
fn saved_position_is_visible(bounds: WindowBounds) -> bool {
    #[link(name = "user32")]
    unsafe extern "system" {
        #[link_name = "GetSystemMetrics"]
        fn get_system_metrics(index: i32) -> i32;
    }
    // SAFETY: GetSystemMetrics has no pointer parameters and is safe for these documented indexes.
    let display = unsafe {
        WindowBounds::new(
            f64::from(get_system_metrics(76)),
            f64::from(get_system_metrics(77)),
            f64::from(get_system_metrics(78)),
            f64::from(get_system_metrics(79)),
        )
    };
    display.width > 0.0 && display.height > 0.0 && bounds_have_visible_area(bounds, display)
}

#[cfg(not(windows))]
fn saved_position_is_visible(_bounds: WindowBounds) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::accesskit::Role;

    #[test]
    fn native_workspace_exposes_buttons_and_text_fields_to_accesskit() {
        for language in AppLanguage::ALL {
            let context = egui::Context::default();
            context.enable_accesskit();
            context.set_theme(egui::ThemePreference::Light);
            let settings = model_merger_app_core::AppSettings {
                language: Some(language),
                ..Default::default()
            };
            let mut app = NativeApp {
                state: NativeAppState::new(settings),
                store: SettingsStore::new(std::env::temp_dir().join("unused-settings.json")),
                scheduler: TaskScheduler::native(2).unwrap(),
                notice: None,
                configured_language: language,
                previews: Vec::new(),
                next_preview_id: 1,
                drop_target: None,
                pending_group_delete: None,
                pending_overwrites: VecDeque::new(),
            };
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(900.0, 680.0),
                )),
                ..Default::default()
            };
            let output = context.run_ui(input, |ui| {
                app.top_bar(ui);
                app.bottom_bar(ui);
                app.central_workspace(ui);
            });
            let update = output
                .platform_output
                .accesskit_update
                .as_ref()
                .expect("AccessKit tree should be emitted");
            let button_count = update
                .nodes
                .iter()
                .filter(|(_, node)| matches!(node.role(), Role::Button | Role::DefaultButton))
                .count();
            let text_input_count = update
                .nodes
                .iter()
                .filter(|(_, node)| node.role() == Role::TextInput)
                .count();
            let progress_count = update
                .nodes
                .iter()
                .filter(|(_, node)| node.role() == Role::ProgressIndicator)
                .count();
            let tree = format!("{:?}", update.nodes);

            assert!(button_count >= 20, "expected slot and command buttons");
            assert!(text_input_count >= 2, "expected labeled output fields");
            assert!(progress_count >= 1, "expected an accessible progress bar");
            assert!(
                tree.contains(Catalog::new(language).text(TextKey::AppTitle)),
                "localized title should be present in the accessibility tree"
            );
            output.drop_without_applying_deltas();
        }
    }

    #[test]
    fn merge_warnings_rerender_from_semantic_data() {
        let warning = model_merger_engine::MergeWarning {
            code: model_merger_engine::MergeWarningCode::NoAttachmentBone,
            model_name: "arm".to_owned(),
            root_model_name: "body".to_owned(),
        };

        let chinese = merge_warning_message(Catalog::new(AppLanguage::ChineseSimplified), &warning);
        let english = merge_warning_message(Catalog::new(AppLanguage::English), &warning);

        assert!(chinese.contains("arm"));
        assert!(chinese.contains("body"));
        assert_ne!(chinese, english);
    }

    #[test]
    fn offscreen_saved_window_position_is_rejected() {
        let display = WindowBounds::new(0.0, 0.0, 1920.0, 1080.0);
        let visible = WindowBounds::new(1800.0, 900.0, 600.0, 500.0);
        let offscreen = WindowBounds::new(2500.0, 1200.0, 1180.0, 860.0);

        assert!(bounds_have_visible_area(visible, display));
        assert!(!bounds_have_visible_area(offscreen, display));
    }

    #[test]
    fn stage_progress_and_model_read_errors_include_actionable_context() {
        let catalog = Catalog::new(AppLanguage::English);
        let progress = TaskProgress {
            stage: model_merger_engine::MergeStage::Loading,
            current: 1,
            total: 2,
            item: Some("arm.cast".to_owned()),
        };
        let error = TaskError::ModelRead {
            path: PathBuf::from("broken.cast"),
            message: "truncated face buffer".to_owned(),
        };

        assert!(task_progress_message(catalog, &progress).contains("arm.cast"));
        let message = task_error_message(catalog, &error);
        assert!(message.contains("broken.cast"));
        assert!(message.contains("truncated face buffer"));
    }

    #[test]
    fn progress_fraction_is_bounded_and_handles_an_empty_total() {
        assert_eq!(0.0, normalized_progress(0, 0));
        assert_eq!(0.5, normalized_progress(1, 2));
        assert_eq!(1.0, normalized_progress(4, 2));
    }

    #[test]
    fn slot_cards_share_the_available_width_without_overflowing() {
        assert_eq!(144.0, slot_card_width(760.0, 5));
        assert_eq!(136.0, slot_card_width(430.0, 3));
        assert_eq!(176.0, slot_card_width(360.0, 2));
    }

    #[test]
    fn compact_workspace_preserves_the_settings_pane_and_minimum_slot_width() {
        let group_content_width = 812.0;
        let slots_width = group_content_width - SETTINGS_PANE_WIDTH - 8.0;
        let columns = slot_columns(slots_width);

        assert_eq!(300.0, SETTINGS_PANE_WIDTH);
        assert_eq!(3, columns);
        assert!(slot_card_width(slots_width, columns) >= MINIMUM_SLOT_CARD_WIDTH);
    }

    #[test]
    fn long_file_names_do_not_expand_the_workspace_past_the_viewport() {
        let directory = std::env::temp_dir().join(format!(
            "model-merger-long-filename-layout-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory
            .join("01_att_t10_vm_p50_dm_erie_rec_LOD0_with_an_intentionally_long_filename.cast");
        std::fs::write(&path, b"cast").unwrap();

        let context = egui::Context::default();
        context.set_theme(egui::ThemePreference::Light);
        let mut state = NativeAppState::new(model_merger_app_core::AppSettings::default());
        assert_eq!(AddPartStatus::Added, state.add_part(0, &path).status);
        let mut app = NativeApp {
            state,
            store: SettingsStore::new(directory.join("settings.json")),
            scheduler: TaskScheduler::native(2).unwrap(),
            notice: None,
            configured_language: AppLanguage::English,
            previews: Vec::new(),
            next_preview_id: 1,
            drop_target: None,
            pending_group_delete: None,
            pending_overwrites: VecDeque::new(),
        };
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1180.0, 860.0),
            )),
            ..Default::default()
        };
        let mut root_rect = egui::Rect::NOTHING;

        let output = context.run_ui(input, |ui| {
            app.central_workspace(ui);
            root_rect = ui.min_rect();
        });
        output.drop_without_applying_deltas();
        let _ = std::fs::remove_dir_all(directory);

        assert!(
            root_rect.right() <= 1182.0,
            "workspace overflowed to x={}",
            root_rect.right()
        );
    }
}
