use crate::{
    AddPartResult, AddPartStatus, AppLanguage, AppSettings, GroupPlan, RootMode, TaskError, TaskId,
    TaskSnapshot, TextKey, WindowBounds,
};
use model_merger_engine::MergeValidationCode;
use model_merger_engine::MergeWarning;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GroupId(u64);

#[derive(Debug, Clone)]
pub struct GroupSession {
    id: GroupId,
    pub plan: GroupPlan,
    pub collapsed: bool,
    task_id: Option<TaskId>,
    task_snapshot: Option<TaskSnapshot>,
    log: Vec<GroupLog>,
    last_output: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub enum GroupLog {
    Status(TextKey),
    Warning(MergeWarning),
    Output(PathBuf),
    Error(TaskError),
}

impl GroupSession {
    fn new(id: GroupId, settings: &AppSettings) -> Self {
        Self {
            id,
            plan: GroupPlan::new(
                settings.preferred_output_directory.as_deref(),
                settings.root_mode,
            ),
            collapsed: false,
            task_id: None,
            task_snapshot: None,
            log: Vec::new(),
            last_output: None,
        }
    }

    pub fn id(&self) -> GroupId {
        self.id
    }

    pub fn task_id(&self) -> Option<TaskId> {
        self.task_id
    }

    pub fn task_snapshot(&self) -> Option<&TaskSnapshot> {
        self.task_snapshot.as_ref()
    }

    pub fn log(&self) -> &[GroupLog] {
        &self.log
    }

    pub fn last_output(&self) -> Option<&Path> {
        self.last_output.as_deref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskCompletion {
    pub group_id: GroupId,
    pub requires_overwrite_confirmation: bool,
}

#[derive(Debug, Clone)]
pub struct WorkspaceState {
    settings: AppSettings,
    language: AppLanguage,
    groups: Vec<GroupSession>,
    next_group_id: u64,
}

impl WorkspaceState {
    pub fn new(settings: AppSettings) -> Self {
        let language = settings.language.unwrap_or_else(detect_system_language);
        let first_group = GroupSession::new(GroupId(1), &settings);
        Self {
            settings,
            language,
            groups: vec![first_group],
            next_group_id: 2,
        }
    }

    pub fn language(&self) -> AppLanguage {
        self.language
    }

    pub fn settings(&self) -> &AppSettings {
        &self.settings
    }

    pub fn set_check_updates_on_startup(&mut self, enabled: bool) {
        self.settings.check_updates_on_startup = enabled;
    }

    pub fn groups(&self) -> &[GroupSession] {
        &self.groups
    }

    pub fn groups_mut(&mut self) -> &mut [GroupSession] {
        &mut self.groups
    }

    pub fn set_language(&mut self, language: AppLanguage) {
        self.language = language;
        self.settings.language = Some(language);
    }

    pub fn add_group(&mut self) -> usize {
        let index = self.groups.len();
        let id = GroupId(self.next_group_id);
        self.next_group_id = self.next_group_id.saturating_add(1);
        self.groups.push(GroupSession::new(id, &self.settings));
        index
    }

    pub fn group_index(&self, id: GroupId) -> Option<usize> {
        self.groups.iter().position(|group| group.id() == id)
    }

    pub fn start_task(
        &mut self,
        group_index: usize,
        task_id: TaskId,
        snapshot: Option<TaskSnapshot>,
    ) -> bool {
        let Some(group) = self.groups.get_mut(group_index) else {
            return false;
        };
        if group.task_id.is_some() {
            return false;
        }
        group.task_id = Some(task_id);
        group.task_snapshot = snapshot;
        group.log.push(GroupLog::Status(TextKey::Queued));
        true
    }

    pub fn apply_task_snapshot(
        &mut self,
        group_index: usize,
        snapshot: TaskSnapshot,
    ) -> Option<TaskCompletion> {
        let group = self.groups.get_mut(group_index)?;
        if group.task_id != Some(snapshot.id) {
            return None;
        }
        group.task_snapshot = Some(snapshot.clone());
        if !matches!(
            snapshot.state,
            crate::TaskState::Succeeded | crate::TaskState::Failed | crate::TaskState::Cancelled
        ) {
            return None;
        }

        let requires_overwrite_confirmation = matches!(
            snapshot.error.as_ref(),
            Some(TaskError::Validation {
                code: MergeValidationCode::OutputAlreadyExists,
                ..
            })
        );
        if let Some(result) = snapshot.result {
            for warning in result.warnings {
                group.log.push(GroupLog::Warning(warning));
            }
            group.last_output = Some(result.output_path.clone());
            group.log.push(GroupLog::Output(result.output_path));
        } else if let Some(error) = snapshot.error {
            group.log.push(GroupLog::Error(error));
        }
        group.task_id = None;
        Some(TaskCompletion {
            group_id: group.id,
            requires_overwrite_confirmation,
        })
    }

    pub fn delete_group(&mut self, index: usize) -> bool {
        if self.groups.len() == 1 || index >= self.groups.len() {
            return false;
        }
        self.groups.remove(index);
        true
    }

    pub fn set_collapsed(&mut self, index: usize, collapsed: bool) -> bool {
        let Some(group) = self.groups.get_mut(index) else {
            return false;
        };
        group.collapsed = collapsed;
        true
    }

    pub fn add_part(&mut self, group_index: usize, path: impl AsRef<Path>) -> AddPartResult {
        let Some(group) = self.groups.get_mut(group_index) else {
            return invalid_index();
        };
        group.plan.add_part(path)
    }

    pub fn add_parts<I, P>(&mut self, group_index: usize, paths: I) -> Vec<AddPartResult>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        paths
            .into_iter()
            .map(|path| self.add_part(group_index, path))
            .collect()
    }

    pub fn replace_part(
        &mut self,
        group_index: usize,
        part_index: usize,
        path: impl AsRef<Path>,
    ) -> AddPartResult {
        let Some(group) = self.groups.get_mut(group_index) else {
            return invalid_index();
        };
        group.plan.replace_part(part_index, path)
    }

    pub fn remove_part(&mut self, group_index: usize, part_index: usize) -> bool {
        self.groups
            .get_mut(group_index)
            .is_some_and(|group| group.plan.remove_part(part_index))
    }

    pub fn clear_parts(&mut self, group_index: usize) -> bool {
        let Some(group) = self.groups.get_mut(group_index) else {
            return false;
        };
        group.plan.clear_parts();
        true
    }

    pub fn reset_defaults(&mut self) {
        let language = self.language;
        self.settings = AppSettings {
            language: Some(language),
            ..AppSettings::default()
        };
        for group in &mut self.groups {
            group.plan.reset_preferences(None, self.settings.root_mode);
        }
    }

    pub fn set_remember_output(&mut self, remember: bool) {
        self.settings.remember_output_directory = remember;
        if !remember {
            self.settings.preferred_output_directory = None;
        } else if self.settings.preferred_output_directory.is_none() {
            self.settings.preferred_output_directory = self.groups.iter().rev().find_map(|group| {
                let state = group.plan.state();
                state
                    .has_explicit_output_directory
                    .then_some(state.output_directory)
            });
        }
    }

    pub fn remember_output_directory(&mut self, directory: PathBuf) {
        if self.settings.remember_output_directory {
            self.settings.preferred_output_directory = Some(directory);
        }
    }

    pub fn set_group_root_mode(&mut self, group_index: usize, root_mode: RootMode) -> bool {
        let Some(group) = self.groups.get_mut(group_index) else {
            return false;
        };
        group.plan.set_root_mode(root_mode);
        self.settings.root_mode = root_mode;
        true
    }

    pub fn set_group_manual_root(&mut self, group_index: usize, part_index: usize) -> bool {
        let Some(group) = self.groups.get_mut(group_index) else {
            return false;
        };
        if !group.plan.set_manual_root(part_index) {
            return false;
        }
        self.settings.root_mode = RootMode::Manual;
        true
    }

    pub fn update_window_bounds(&mut self, bounds: WindowBounds) {
        self.settings.window_bounds = Some(bounds);
    }
}

fn invalid_index() -> AddPartResult {
    AddPartResult {
        status: AddPartStatus::InvalidIndex,
        file_path: None,
    }
}

fn detect_system_language() -> AppLanguage {
    system_locale()
        .as_deref()
        .map(language_from_locale)
        .unwrap_or(AppLanguage::ChineseSimplified)
}

fn language_from_locale(locale: &str) -> AppLanguage {
    let locale = locale.to_ascii_lowercase();
    if locale.starts_with("zh") {
        AppLanguage::ChineseSimplified
    } else if locale.starts_with("fr") {
        AppLanguage::French
    } else if locale.starts_with("ru") {
        AppLanguage::Russian
    } else if locale.starts_with("es") {
        AppLanguage::Spanish
    } else if locale.starts_with("en") {
        AppLanguage::English
    } else {
        AppLanguage::ChineseSimplified
    }
}

#[cfg(windows)]
fn system_locale() -> Option<String> {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        #[link_name = "GetUserDefaultUILanguage"]
        fn get_user_default_ui_language() -> u16;
        #[link_name = "LCIDToLocaleName"]
        fn lcid_to_locale_name(
            locale: u32,
            locale_name: *mut u16,
            locale_name_count: i32,
            flags: u32,
        ) -> i32;
    }
    let mut buffer = [0_u16; 85];
    // SAFETY: Both functions have no borrowed inputs, and Windows writes at most the supplied
    // UTF-16 buffer length. A LANGID is also a valid default-sort LCID.
    let length = unsafe {
        let language = get_user_default_ui_language();
        if language == 0 {
            return None;
        }
        lcid_to_locale_name(
            u32::from(language),
            buffer.as_mut_ptr(),
            i32::try_from(buffer.len()).ok()?,
            0,
        )
    };
    (length > 1).then(|| String::from_utf16_lossy(&buffer[..length as usize - 1]))
}

#[cfg(not(windows))]
fn system_locale() -> Option<String> {
    std::env::var("LANG").ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_system_locales_select_the_matching_catalog() {
        assert_eq!(
            AppLanguage::ChineseSimplified,
            language_from_locale("zh-CN")
        );
        assert_eq!(AppLanguage::French, language_from_locale("fr-FR"));
        assert_eq!(AppLanguage::Russian, language_from_locale("ru-RU"));
        assert_eq!(AppLanguage::Spanish, language_from_locale("es-MX"));
        assert_eq!(AppLanguage::English, language_from_locale("en-US"));
        assert_eq!(
            AppLanguage::ChineseSimplified,
            language_from_locale("de-DE")
        );
    }

    #[test]
    fn enabling_output_memory_captures_the_latest_explicit_directory() {
        let mut workspace = WorkspaceState::new(AppSettings::default());
        let directory = std::env::current_dir().unwrap();
        workspace.groups_mut()[0]
            .plan
            .choose_output_directory(&directory);

        workspace.set_remember_output(true);

        assert!(workspace.settings().remember_output_directory);
        assert_eq!(
            Some(directory.as_path()),
            workspace.settings().preferred_output_directory.as_deref()
        );
    }

    #[test]
    fn selecting_a_manual_root_updates_the_saved_root_preference() {
        let directory = std::env::temp_dir().join(format!(
            "model-merger-workspace-root-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let part = directory.join("root.cast");
        std::fs::write(&part, b"cast").unwrap();
        let mut workspace = WorkspaceState::new(AppSettings::default());
        assert_eq!(AddPartStatus::Added, workspace.add_part(0, &part).status);

        assert!(workspace.set_group_manual_root(0, 0));

        assert_eq!(RootMode::Manual, workspace.settings().root_mode);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn group_ids_remain_stable_when_an_earlier_group_is_deleted() {
        let mut workspace = WorkspaceState::new(AppSettings::default());
        workspace.add_group();
        workspace.add_group();
        let retained = workspace.groups()[2].id();

        assert!(workspace.delete_group(1));

        assert_eq!(Some(1), workspace.group_index(retained));
    }
}
