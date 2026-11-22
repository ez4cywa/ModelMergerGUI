//! The selected group's output controls, part list, and independent preview tabs.

use super::{
    FileDropTarget, NativeApp, SLOT_COUNT, disclosure_button, file_drop_contains_pointer,
    file_drop_pointer, is_cast_path, normalized_progress, short_name, text_width,
};
use crate::{GroupLog, messages, theme};
use eframe::egui::{self, RichText, Stroke};
use model_merger_app_core::{Catalog, GroupPlanState, RootMode, TextKey};
use std::path::Path;

const FIELD_HEIGHT: f32 = 34.0;
const FIELD_LABEL_HEIGHT: f32 = 18.0;
const FIELD_LABEL_GAP: f32 = 4.0;

impl NativeApp {
    pub(super) fn active_group_index(&self) -> usize {
        self.active_group
            .and_then(|id| self.state.group_index(id))
            .unwrap_or(0)
    }

    pub(super) fn select_group(&mut self, index: usize) {
        self.active_group = self.state.groups().get(index).map(|group| group.id());
        self.state.set_collapsed(index, false);
    }

    pub(super) fn output_bar(&mut self, root: &mut egui::Ui) {
        let catalog = self.catalog();
        let palette = theme::palette(root);
        egui::Panel::top("group-output-bar")
            .frame(
                egui::Frame::new()
                    .fill(palette.panel)
                    .stroke(Stroke::new(1.0, palette.border))
                    .inner_margin(egui::Margin::symmetric(16, 9)),
            )
            .show(root, |ui| {
                ui.spacing_mut().interact_size.y = FIELD_HEIGHT;
                ui.spacing_mut().button_padding = egui::vec2(10.0, 5.0);
                self.notices.show_global(ui, catalog);
                let mut index = self.active_group_index();
                let group_label = group_name(catalog, index);
                let group_value_width = text_width(ui, &group_label, 15.0)
                    + 2.0 * ui.spacing().button_padding.x
                    + ui.spacing().icon_width
                    + ui.spacing().icon_spacing
                    + 2.0;
                let group_width = field_width(ui, catalog, &[TextKey::CurrentGroup], 126.0)
                    .max(group_value_width);
                let root_width = field_width(
                    ui,
                    catalog,
                    &[TextKey::RootModel, TextKey::Automatic, TextKey::Manual],
                    130.0,
                );
                let action_width =
                    field_width(ui, catalog, &[TextKey::StartGroup, TextKey::Cancel], 88.0);
                let gap = ui.spacing().item_spacing.x;
                let available = ui.available_width();
                let minimum_name_width =
                    field_width(ui, catalog, &[TextKey::OutputFileName], 160.0);
                let inline = available
                    >= group_width
                        + root_width
                        + action_width
                        + minimum_name_width
                        + 225.0
                        + 4.0 * gap;
                let fields_width = if inline {
                    available - group_width - root_width - action_width - 4.0 * gap
                } else {
                    available - gap
                };
                let name_width = (fields_width * 0.35).max(minimum_name_width);
                let directory_width = fields_width - name_width;

                ui.horizontal_top(|ui| {
                    field(ui, group_width, catalog.text(TextKey::CurrentGroup), |ui| {
                        egui::ComboBox::from_id_salt("active-model-group")
                            .selected_text(group_label)
                            .width(ui.available_width())
                            .truncate()
                            .show_ui(ui, |ui| {
                                for candidate in 0..self.state.groups().len() {
                                    if ui
                                        .selectable_label(
                                            index == candidate,
                                            group_name(catalog, candidate),
                                        )
                                        .clicked()
                                    {
                                        self.select_group(candidate);
                                        index = candidate;
                                    }
                                }
                            });
                    });
                    let snapshot = self.state.groups()[index].plan.state();
                    let locked = self.state.groups()[index].task_id().is_some();
                    field(ui, root_width, catalog.text(TextKey::RootModel), |ui| {
                        ui.add_enabled_ui(!locked, |ui| {
                            let mut mode = snapshot.root_mode;
                            egui::ComboBox::from_id_salt("root-mode")
                                .selected_text(catalog.text(match mode {
                                    RootMode::Automatic => TextKey::Automatic,
                                    RootMode::Manual => TextKey::Manual,
                                }))
                                .width(ui.available_width())
                                .truncate()
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut mode,
                                        RootMode::Automatic,
                                        catalog.text(TextKey::Automatic),
                                    );
                                    ui.selectable_value(
                                        &mut mode,
                                        RootMode::Manual,
                                        catalog.text(TextKey::Manual),
                                    );
                                });
                            if mode != snapshot.root_mode {
                                self.state.set_group_root_mode(index, mode);
                            }
                        });
                    });
                    if inline {
                        self.output_fields(ui, index, directory_width, name_width);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            ui.add_space(FIELD_LABEL_HEIGHT + FIELD_LABEL_GAP);
                            if let Some(task_id) = self.state.groups()[index].task_id() {
                                if ui
                                    .add_sized(
                                        [action_width, FIELD_HEIGHT],
                                        egui::Button::new(catalog.text(TextKey::Cancel)),
                                    )
                                    .clicked()
                                {
                                    self.scheduler.cancel(task_id);
                                }
                            } else if ui
                                .add_enabled(
                                    snapshot.is_ready,
                                    egui::Button::new(
                                        RichText::new(catalog.text(TextKey::StartGroup))
                                            .color(palette.on_primary),
                                    )
                                    .fill(palette.primary)
                                    .min_size(egui::vec2(action_width, FIELD_HEIGHT)),
                                )
                                .clicked()
                            {
                                self.schedule_group(index, false);
                            }
                        })
                    });
                });
                if !inline {
                    ui.horizontal_top(|ui| {
                        self.output_fields(ui, index, directory_width, name_width)
                    });
                }
                self.output_status(ui, index, catalog);
                let group_id = self.state.groups()[index].id();
                self.notices.show_group(ui, catalog, group_id);
            });
    }

    fn output_fields(
        &mut self,
        ui: &mut egui::Ui,
        index: usize,
        directory_width: f32,
        name_width: f32,
    ) {
        let catalog = self.catalog();
        let snapshot = self.state.groups()[index].plan.state();
        let locked = self.state.groups()[index].task_id().is_some();
        field(
            ui,
            directory_width,
            catalog.text(TextKey::OutputFolder),
            |ui| {
                ui.add_enabled_ui(!locked, |ui| {
                    ui.horizontal(|ui| {
                        let browse = catalog.text(TextKey::Browse);
                        let button_width =
                            text_width(ui, browse, 15.0) + 2.0 * ui.spacing().button_padding.x;
                        let width =
                            (ui.available_width() - button_width - ui.spacing().item_spacing.x)
                                .max(40.0);
                        let directory = snapshot.output_directory.display().to_string();
                        ui.add_sized(
                            [width, FIELD_HEIGHT],
                            egui::TextEdit::singleline(&mut directory.as_str())
                                .vertical_align(egui::Align::Center)
                                .hint_text(catalog.text(TextKey::SelectOutput)),
                        )
                        .on_hover_text(directory);
                        if ui
                            .add_sized([button_width, FIELD_HEIGHT], egui::Button::new(browse))
                            .clicked()
                        {
                            self.choose_output_directory(index);
                        }
                    });
                });
            },
        );
        field(
            ui,
            name_width,
            catalog.text(TextKey::OutputFileName),
            |ui| {
                let mut name = snapshot.output_file_name;
                let width = ui.available_width();
                ui.add_enabled_ui(!locked, |ui| {
                    if ui
                        .add_sized(
                            [width, FIELD_HEIGHT],
                            egui::TextEdit::singleline(&mut name)
                                .id_salt(("group-output-name", self.state.groups()[index].id()))
                                .vertical_align(egui::Align::Center),
                        )
                        .changed()
                    {
                        self.state.groups_mut()[index]
                            .plan
                            .set_output_file_name(name);
                    }
                });
            },
        );
    }

    fn output_status(&mut self, ui: &mut egui::Ui, index: usize, catalog: Catalog) {
        let group = &self.state.groups()[index];
        let palette = theme::palette(ui);
        let progress = group
            .task_id()
            .and_then(|id| self.scheduler.snapshot(id))
            .or_else(|| group.task_snapshot().cloned());
        let progress_value = progress
            .as_ref()
            .and_then(|snapshot| snapshot.progress.as_ref())
            .map_or(0.0, |progress| {
                normalized_progress(progress.current, progress.total)
            });
        let snapshot = group.plan.state();
        let output = group.last_output().map(Path::to_path_buf);
        let locked = group.task_id().is_some();
        let status = progress.as_ref().map_or_else(
            || {
                catalog
                    .text(if snapshot.is_ready {
                        TextKey::Ready
                    } else {
                        TextKey::NeedTwoToFifteen
                    })
                    .to_owned()
            },
            |task| {
                task.progress.as_ref().map_or_else(
                    || messages::task_label(catalog, Some(task.state)).to_owned(),
                    |value| messages::task_progress(catalog, value),
                )
            },
        );
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(catalog.text(TextKey::GroupStatus))
                    .strong()
                    .color(palette.foreground),
            );
            ui.add(egui::Label::new(RichText::new(&status).color(palette.secondary)).truncate())
                .on_hover_text(&status);
            ui.scope(|ui| {
                ui.visuals_mut().extreme_bg_color = palette.progress_track;
                ui.add(
                    egui::ProgressBar::new(progress_value)
                        .desired_width(86.0)
                        .desired_height(8.0)
                        .corner_radius(4.0)
                        .fill(palette.primary),
                );
            });
            ui.label(
                RichText::new(format!("{:.0}%", progress_value * 100.0))
                    .size(13.0)
                    .color(palette.secondary),
            );
            if snapshot.root_mode == RootMode::Manual && !snapshot.part_files.is_empty() {
                self.manual_root(ui, index, &snapshot, locked);
            }
            self.remember_output_control(ui, catalog);
            if let Some(path) = output.as_deref().filter(|path| path.is_file())
                && ui.button(catalog.text(TextKey::PreviewMerged)).clicked()
            {
                self.open_preview(path);
            }
        });
    }

    pub(super) fn run_log_panel(&self, ui: &mut egui::Ui, index: usize, catalog: Catalog) {
        let group = &self.state.groups()[index];
        let palette = theme::palette(ui);
        egui::Frame::new()
            .fill(palette.surface)
            .stroke(Stroke::new(1.0, palette.border))
            .corner_radius(7.0)
            .inner_margin(8)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.label(RichText::new(group_name(catalog, index)).color(palette.foreground));
                egui::ScrollArea::vertical()
                    .id_salt(("run-log-entries", group.id()))
                    .max_height(100.0)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        if group.log().is_empty() {
                            ui.label(
                                RichText::new(catalog.text(TextKey::NoRunLog))
                                    .size(13.0)
                                    .color(palette.secondary),
                            );
                        }
                        for entry in group.log() {
                            let message = match entry {
                                GroupLog::Status(key) => catalog.text(*key).to_owned(),
                                GroupLog::Warning(warning) => {
                                    messages::merge_warning(catalog, warning)
                                }
                                GroupLog::Output(path) => catalog
                                    .text(TextKey::ProgressCompleted)
                                    .replace("{0}", &path.display().to_string()),
                                GroupLog::Error(error) => messages::task_error(catalog, error),
                            };
                            ui.label(RichText::new(message).size(13.0).color(palette.secondary));
                        }
                    });
            });
    }

    fn manual_root(
        &mut self,
        ui: &mut egui::Ui,
        index: usize,
        snapshot: &GroupPlanState,
        locked: bool,
    ) {
        let current = snapshot
            .manual_root_file
            .as_ref()
            .and_then(|root| snapshot.part_files.iter().position(|part| part == root))
            .unwrap_or(0);
        let mut selected = current;
        ui.add_enabled_ui(!locked, |ui| {
            egui::ComboBox::from_id_salt(("manual-root", self.state.groups()[index].id()))
                .selected_text(short_name(&snapshot.part_files[current]))
                .width(156.0)
                .truncate()
                .show_ui(ui, |ui| {
                    for (part_index, path) in snapshot.part_files.iter().enumerate() {
                        ui.selectable_value(&mut selected, part_index, short_name(path));
                    }
                });
        });
        if selected != current {
            self.state.set_group_manual_root(index, selected);
        }
    }

    pub(super) fn central_workspace(&mut self, root: &mut egui::Ui) {
        let context = root.ctx().clone();
        self.drop_target = file_drop_pointer(&context)
            .filter(|point| context.content_rect().contains(*point))
            .map(|_| FileDropTarget::Import);
        if self.drop_target.is_none() && context.input(|input| !input.raw.dropped_files.is_empty())
        {
            self.drop_target = Some(FileDropTarget::Import);
        }
        let palette = theme::palette(root);
        let sidebar = egui::Panel::left("parts-sidebar")
            .exact_size((root.available_width() * 0.205).clamp(220.0, 280.0))
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(palette.sidebar)
                    .stroke(Stroke::new(1.0, palette.border))
                    .inner_margin(egui::Margin::same(10)),
            )
            .show(root, |ui| self.parts_sidebar(ui))
            .response
            .rect;
        let preview = egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(palette.panel))
            .show(root, |ui| self.preview_workspace(ui))
            .response
            .rect;
        if file_drop_contains_pointer(root, preview) {
            self.drop_target = Some(FileDropTarget::Preview(preview));
        }
        if self.modal_open() {
            self.drop_target = None;
        }
        let feedback = match self.drop_target {
            Some(FileDropTarget::Preview(rect)) => Some((rect, TextKey::DropPreview)),
            Some(FileDropTarget::Import) => Some((sidebar, TextKey::DropBatch)),
            _ => None,
        };
        if let Some((rect, key)) = feedback {
            self.paint_drop_hint(root, rect, key);
        }
        if let Some(index) = self.pending_group_delete.take() {
            let id = self.state.groups().get(index).map(|group| group.id());
            if self.state.delete_group(index)
                && let Some(id) = id
            {
                self.notices.clear_group(id);
                if self.active_group == Some(id) {
                    self.select_group(index.min(self.state.groups().len() - 1));
                }
            }
        }
    }

    pub(super) fn modal_open(&self) -> bool {
        self.about.is_open()
            || self.ammunition.open
            || self.armature.open
            || !self.pending_overwrites.is_empty()
    }

    fn paint_drop_hint(&self, ui: &egui::Ui, rect: egui::Rect, key: TextKey) {
        if rect.width() < 24.0
            || rect.height() < 15.0
            || !ui.input(|input| {
                input
                    .raw
                    .hovered_files
                    .iter()
                    .any(|file| file.path.as_deref().is_some_and(is_cast_path))
            })
        {
            return;
        }
        let palette = theme::palette(ui);
        let painter = ui.painter().with_clip_rect(rect);
        painter.rect_stroke(
            rect.shrink(2.0),
            7.0,
            Stroke::new(3.0, palette.primary),
            egui::StrokeKind::Inside,
        );
        let text = painter.layout(
            self.catalog().text(key).to_owned(),
            egui::FontId::proportional(14.0),
            palette.foreground,
            (rect.width() - 38.0).clamp(1.0, 360.0),
        );
        let size = text.size() + egui::vec2(20.0, 10.0);
        let pointer = file_drop_pointer(ui.ctx()).unwrap_or(rect.center());
        let position = egui::pos2(
            (pointer.x + 12.0)
                .min(rect.right() - size.x - 6.0)
                .max(rect.left() + 6.0),
            (pointer.y + 12.0)
                .min(rect.bottom() - size.y - 4.0)
                .max(rect.top() + 4.0),
        );
        painter.rect_filled(
            egui::Rect::from_min_size(position, size),
            6.0,
            palette.notice_info,
        );
        painter.galley(position + egui::vec2(10.0, 5.0), text, palette.foreground);
    }

    fn parts_sidebar(&mut self, ui: &mut egui::Ui) {
        let catalog = self.catalog();
        let palette = theme::palette(ui);
        let selected = self.active_group_index();
        let snapshot = self.state.groups()[selected].plan.state();
        let locked = self.state.groups()[selected].task_id().is_some();
        ui.spacing_mut().interact_size.y = 28.0;
        ui.spacing_mut().button_padding = egui::vec2(6.0, 3.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(catalog.text(TextKey::ModelParts))
                    .strong()
                    .color(palette.foreground),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("{} / {SLOT_COUNT}", snapshot.part_files.len()))
                        .size(13.0)
                        .color(palette.secondary),
                );
            });
        });
        ui.add_enabled_ui(!locked, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button(catalog.text(TextKey::AddPart)).clicked() {
                    self.add_part_dialog(selected, None);
                }
                if ui
                    .button(crate::ammunition::title(catalog.language()))
                    .clicked()
                {
                    let source = self.state.groups()[selected]
                        .last_output()
                        .map(Path::to_path_buf)
                        .or_else(|| snapshot.part_files.first().cloned());
                    self.ammunition.start(source);
                }
                if ui
                    .add_enabled(
                        !snapshot.part_files.is_empty(),
                        egui::Button::new(catalog.text(TextKey::Clear)).frame(false),
                    )
                    .clicked()
                {
                    self.clear_parts(selected);
                }
                if self.state.armature_tool_enabled()
                    && ui
                        .button(crate::armature::title(catalog.language()))
                        .clicked()
                {
                    let weapon = self.state.groups()[selected]
                        .last_output()
                        .map(Path::to_path_buf)
                        .or_else(|| snapshot.part_files.first().cloned());
                    self.armature.start(weapon);
                }
            });
        });
        let scroll_height = (ui.available_height() - 55.0).max(24.0);
        egui::ScrollArea::vertical()
            .id_salt("part-groups")
            .auto_shrink([false, false])
            .max_height(scroll_height)
            .show(ui, |ui| {
                for index in 0..self.state.groups().len() {
                    self.group_rows(ui, index, catalog);
                }
            });
        ui.label(
            RichText::new(catalog.text(TextKey::ModelPartsHint))
                .size(12.0)
                .color(palette.secondary),
        );
    }

    fn group_rows(&mut self, ui: &mut egui::Ui, index: usize, catalog: Catalog) {
        let group = &self.state.groups()[index];
        let group_id = group.id();
        let locked = group.task_id().is_some();
        let collapsed = group.collapsed;
        let selected = index == self.active_group_index();
        let snapshot = group.plan.state();
        let palette = theme::palette(ui);
        ui.separator();
        let rect = ui
            .push_id(group_id, |ui| {
                ui.horizontal(|ui| {
                    let label = format!(
                        "{} · {}",
                        group_name(catalog, index),
                        snapshot.part_files.len()
                    );
                    if disclosure_button(ui, &label, collapsed).clicked() {
                        self.active_group = Some(group_id);
                        self.state.set_collapsed(index, selected && !collapsed);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.state.groups().len() > 1
                            && !locked
                            && ui
                                .add(egui::Button::new("×").frame(false))
                                .on_hover_text(catalog.text(TextKey::DeleteGroup))
                                .clicked()
                        {
                            self.pending_group_delete = Some(index);
                        }
                        if locked {
                            ui.spinner();
                        }
                    });
                });
                if !collapsed {
                    for (part_index, path) in snapshot.part_files.iter().enumerate() {
                        self.part_row(ui, index, part_index, path, &snapshot, locked);
                    }
                }
                if selected {
                    // An accent at the header identifies the owner of the shared output settings.
                    let top = ui.min_rect().top();
                    let left = ui.min_rect().left() - 4.0;
                    ui.painter().vline(
                        left,
                        top + 7.0..=top + 25.0,
                        Stroke::new(2.0, palette.primary),
                    );
                }
            })
            .response
            .rect
            .intersect(ui.clip_rect());
        if file_drop_contains_pointer(ui, rect) {
            self.drop_target = Some(FileDropTarget::Group(index));
            if !locked && !self.modal_open() {
                self.paint_drop_hint(ui, rect, TextKey::DropHere);
            }
        }
    }

    fn part_row(
        &mut self,
        ui: &mut egui::Ui,
        index: usize,
        part_index: usize,
        path: &Path,
        snapshot: &GroupPlanState,
        locked: bool,
    ) {
        let catalog = self.catalog();
        let palette = theme::palette(ui);
        let active = self
            .previews
            .iter()
            .any(|preview| Some(preview.id) == self.active_preview && preview.path == path);
        let label_width =
            (ui.available_width() - field_width(ui, catalog, &[TextKey::Actions], 48.0) - 8.0)
                .max(46.0);
        egui::Frame::new()
            .fill(if active {
                palette.notice_info
            } else {
                egui::Color32::TRANSPARENT
            })
            .corner_radius(6.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let mut label = short_name(path);
                    if snapshot.root_mode == RootMode::Manual
                        && snapshot.manual_root_file.as_deref() == Some(path)
                    {
                        label = format!("{} · {label}", catalog.text(TextKey::RootBadge));
                    }
                    let present = path.is_file();
                    let response = ui
                        .add_sized(
                            [label_width, 28.0],
                            egui::Button::new("")
                                .left_text(RichText::new(label).size(13.0).color(if present {
                                    palette.foreground
                                } else {
                                    palette.destructive
                                }))
                                .frame(false)
                                .truncate(),
                        )
                        .on_hover_text(if present {
                            path.display().to_string()
                        } else {
                            format!("{}\n{}", catalog.text(TextKey::FileMissing), path.display())
                        });
                    if response.clicked() {
                        if let Some(preview) =
                            self.previews.iter().find(|preview| preview.path == path)
                        {
                            self.active_preview = Some(preview.id);
                        } else {
                            self.open_preview(path);
                        }
                    }
                    ui.add_enabled_ui(!locked, |ui| {
                        ui.menu_button(
                            RichText::new(catalog.text(TextKey::Actions)).size(12.0),
                            |ui| {
                                if ui.button(catalog.text(TextKey::Replace)).clicked() {
                                    self.add_part_dialog(index, Some(part_index));
                                    ui.close();
                                }
                                if ui.button(catalog.text(TextKey::SetAsRoot)).clicked() {
                                    self.state.set_group_manual_root(index, part_index);
                                    ui.close();
                                }
                                if ui
                                    .button(
                                        RichText::new(catalog.text(TextKey::Remove))
                                            .color(palette.destructive),
                                    )
                                    .clicked()
                                {
                                    self.remove_part(index, part_index);
                                    ui.close();
                                }
                            },
                        );
                    });
                });
            });
    }

    fn preview_workspace(&mut self, ui: &mut egui::Ui) {
        let catalog = self.catalog();
        let palette = theme::palette(ui);
        egui::Panel::top("preview-tab-bar").frame(egui::Frame::new().fill(palette.panel)
            .stroke(Stroke::new(1.0, palette.border)).inner_margin(egui::Margin::symmetric(12, 7)))
            .show(ui, |ui| {
                ui.spacing_mut().interact_size.y = 30.0;
                ui.spacing_mut().button_padding = egui::vec2(8.0, 4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(catalog.text(TextKey::Preview)).strong().color(palette.foreground));
                    let label = catalog.text(TextKey::OpenPreview);
                    let button_width = text_width(ui, label, 15.0) + ui.spacing().button_padding.x * 2.0 + 2.0;
                    let tab_width = (ui.available_width() - button_width - ui.spacing().item_spacing.x).max(0.0);
                    let mut closed = None;
                    ui.allocate_ui_with_layout(egui::vec2(tab_width, 32.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.set_width(tab_width);
                        egui::ScrollArea::horizontal().id_salt("preview-tab-scroll").auto_shrink([false, true]).show(ui, |ui| {
                            if !self.previews.is_empty() {
                                egui::Frame::new().fill(palette.surface).corner_radius(7.0).inner_margin(2).show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 4.0;
                                        for preview in &self.previews {
                                            let active = self.active_preview == Some(preview.id);
                                            egui::Frame::new().fill(if active { palette.control } else { egui::Color32::TRANSPARENT })
                                                .corner_radius(6.0).show(ui, |ui| {
                                                    ui.horizontal(|ui| {
                                                        if ui.add_sized([178.0, 28.0], egui::Button::new(RichText::new(&preview.title).size(13.0))
                                                            .frame(false).truncate()).on_hover_text(preview.path.display().to_string()).clicked() {
                                                            self.active_preview = Some(preview.id);
                                                        }
                                                        let close = ui.add(egui::Button::new("×").frame(false));
                                                        let close_label = format!("{} {}", catalog.text(TextKey::Close), preview.title);
                                                        close.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &close_label));
                                                        if close.on_hover_text(close_label).clicked() { closed = Some(preview.id); }
                                                    });
                                                });
                                        }
                                    });
                                });
                            }
                        });
                    });
                    if let Some(id) = closed { self.close_preview(id); }
                    if ui.add_sized([button_width, 30.0], egui::Button::new(label)).clicked() { self.open_preview_dialog(); }
                });
            });
        let active = self.active_preview;
        if let Some(preview) = self
            .previews
            .iter_mut()
            .find(|preview| Some(preview.id) == active)
        {
            preview.show(ui, catalog);
        } else {
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(palette.preview_canvas))
                .show(ui, |ui| {
                    ui.centered_and_justified(|ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(catalog.text(TextKey::PreviewDropZone))
                                        .color(palette.secondary),
                                )
                                .frame(false)
                                .wrap(),
                            )
                            .clicked()
                        {
                            self.open_preview_dialog();
                        }
                    });
                });
        }
    }
}

fn group_name(catalog: Catalog, index: usize) -> String {
    format!("{} {}", catalog.text(TextKey::Group), index + 1)
}

fn field_width(ui: &egui::Ui, catalog: Catalog, keys: &[TextKey], minimum: f32) -> f32 {
    keys.iter()
        .map(|key| text_width(ui, catalog.text(*key), 15.0) + 26.0)
        .fold(minimum, f32::max)
}

fn field(ui: &mut egui::Ui, width: f32, label: &str, contents: impl FnOnce(&mut egui::Ui)) {
    ui.allocate_ui_with_layout(
        egui::vec2(width, 0.0),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.set_width(width);
            ui.spacing_mut().item_spacing.y = FIELD_LABEL_GAP;
            ui.add_sized(
                [width, FIELD_LABEL_HEIGHT],
                egui::Label::new(RichText::new(label).size(13.0)).truncate(),
            );
            contents(ui);
        },
    );
}
