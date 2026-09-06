use crate::{messages, theme};
use eframe::egui::{self, Color32, Stroke};
use model_merger_app_core::{AddPartStatus, Catalog, GroupId, TextKey};
use std::collections::HashMap;

pub(super) enum UiNotice {
    Key(TextKey),
    AddPart(AddPartStatus),
    SettingsSaveFailed(String),
}

impl UiNotice {
    fn text(&self, catalog: Catalog) -> String {
        match self {
            Self::Key(key) => catalog.text(*key).to_owned(),
            Self::AddPart(status) => messages::add_part_error(catalog, *status).to_owned(),
            Self::SettingsSaveFailed(detail) => {
                format!("{}: {detail}", catalog.text(TextKey::SettingsSaveFailed))
            }
        }
    }

    fn is_error(&self) -> bool {
        matches!(self, Self::AddPart(_) | Self::SettingsSaveFailed(_))
            || matches!(
                self,
                Self::Key(TextKey::NeedTwoToFifteen | TextKey::MergeFailed)
            )
    }
}

#[derive(Default)]
pub(super) struct NoticeCenter {
    global: Option<UiNotice>,
    groups: HashMap<GroupId, UiNotice>,
}

impl NoticeCenter {
    pub fn set_global(&mut self, notice: UiNotice) {
        self.global = Some(notice);
    }

    pub fn show_global(&mut self, ui: &mut egui::Ui, catalog: Catalog) {
        let Some(notice) = &self.global else {
            return;
        };
        if show_banner(ui, catalog, notice) {
            self.global = None;
        }
        ui.add_space(8.0);
    }

    pub fn set_group(&mut self, group: GroupId, notice: UiNotice) {
        self.groups.insert(group, notice);
    }

    pub fn clear_group(&mut self, group: GroupId) {
        self.groups.remove(&group);
    }

    #[cfg(test)]
    pub fn contains_group(&self, group: GroupId) -> bool {
        self.groups.contains_key(&group)
    }

    pub fn show_group(&mut self, ui: &mut egui::Ui, catalog: Catalog, group: GroupId) {
        let Some(notice) = self.groups.get(&group) else {
            return;
        };
        ui.add_space(8.0);
        if show_banner(ui, catalog, notice) {
            self.groups.remove(&group);
        }
    }
}

fn show_banner(ui: &mut egui::Ui, catalog: Catalog, notice: &UiNotice) -> bool {
    let is_error = notice.is_error();
    let accent = if is_error {
        theme::DESTRUCTIVE
    } else {
        theme::PRIMARY
    };
    let fill = if is_error {
        Color32::from_rgb(254, 242, 242)
    } else {
        Color32::from_rgb(239, 246, 255)
    };
    let mut close = false;
    egui::Frame::new()
        .fill(fill)
        .stroke(Stroke::new(1.0, accent))
        .corner_radius(6.0)
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(accent, notice.text(catalog));
                if ui.button(catalog.text(TextKey::Close)).clicked() {
                    close = true;
                }
            });
        });
    close
}

#[cfg(test)]
mod tests {
    use super::*;
    use model_merger_app_core::{AppSettings, WorkspaceState};

    #[test]
    fn group_notices_do_not_replace_global_or_sibling_messages() {
        let mut workspace = WorkspaceState::new(AppSettings::default());
        workspace.add_group();
        let first = workspace.groups()[0].id();
        let second = workspace.groups()[1].id();
        let mut notices = NoticeCenter::default();

        notices.set_global(UiNotice::Key(TextKey::SettingsSaved));
        notices.set_group(first, UiNotice::AddPart(AddPartStatus::Duplicate));
        notices.set_group(second, UiNotice::AddPart(AddPartStatus::Full));
        notices.clear_group(first);

        assert!(notices.global.is_some());
        assert!(!notices.groups.contains_key(&first));
        assert!(notices.groups.contains_key(&second));
    }
}
