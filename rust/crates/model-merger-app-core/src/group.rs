use crate::RootMode;
use crate::paths::identity_key;
use model_merger_engine::{MergeRequest, RootSelection};
use std::path::{Path, PathBuf};

pub const MINIMUM_PARTS: usize = 2;
pub const MAXIMUM_PARTS: usize = 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddPartStatus {
    Added,
    InvalidPath,
    Missing,
    UnsupportedFormat,
    Duplicate,
    Full,
    InvalidIndex,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddPartResult {
    pub status: AddPartStatus,
    pub file_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupPlanState {
    pub part_files: Vec<PathBuf>,
    pub output_directory: PathBuf,
    pub output_file_name: String,
    pub has_explicit_output_directory: bool,
    pub root_mode: RootMode,
    pub manual_root_file: Option<PathBuf>,
    pub recent_input_directory: Option<PathBuf>,
    pub is_ready: bool,
}

#[derive(Debug, Clone)]
pub struct GroupPlan {
    part_files: Vec<PathBuf>,
    output_directory: PathBuf,
    output_file_name: String,
    has_explicit_output_directory: bool,
    root_mode: RootMode,
    manual_root_file: Option<PathBuf>,
    recent_input_directory: Option<PathBuf>,
}

impl Default for GroupPlan {
    fn default() -> Self {
        Self::new(None, RootMode::Automatic)
    }
}

impl GroupPlan {
    pub fn new(preferred_output_directory: Option<&Path>, root_mode: RootMode) -> Self {
        let preferred = preferred_output_directory
            .filter(|path| path.is_dir())
            .map(Path::to_path_buf);
        Self {
            part_files: Vec::new(),
            output_directory: preferred.clone().unwrap_or_default(),
            output_file_name: String::new(),
            has_explicit_output_directory: preferred.is_some(),
            root_mode,
            manual_root_file: None,
            recent_input_directory: None,
        }
    }

    pub fn state(&self) -> GroupPlanState {
        GroupPlanState {
            part_files: self.part_files.clone(),
            output_directory: self.output_directory.clone(),
            output_file_name: self.output_file_name.clone(),
            has_explicit_output_directory: self.has_explicit_output_directory,
            root_mode: self.root_mode,
            manual_root_file: self.manual_root_file.clone(),
            recent_input_directory: self.recent_input_directory.clone(),
            is_ready: self.is_ready(),
        }
    }

    pub fn add_part(&mut self, file_path: impl AsRef<Path>) -> AddPartResult {
        if self.part_files.len() >= MAXIMUM_PARTS {
            return result(AddPartStatus::Full, None);
        }
        let validated = match validate_part(file_path.as_ref()) {
            Ok(path) => path,
            Err(status) => return result(status, None),
        };
        if self
            .part_files
            .iter()
            .any(|path| paths_equal(path, &validated))
        {
            return result(AddPartStatus::Duplicate, Some(validated));
        }
        self.part_files.push(validated.clone());
        self.remember_input_directory(&validated);
        self.normalize_after_parts_changed();
        result(AddPartStatus::Added, Some(validated))
    }

    pub fn replace_part(&mut self, index: usize, file_path: impl AsRef<Path>) -> AddPartResult {
        if index >= self.part_files.len() {
            return result(AddPartStatus::InvalidIndex, None);
        }
        let validated = match validate_part(file_path.as_ref()) {
            Ok(path) => path,
            Err(status) => return result(status, None),
        };
        if self
            .part_files
            .iter()
            .enumerate()
            .any(|(candidate, path)| candidate != index && paths_equal(path, &validated))
        {
            return result(AddPartStatus::Duplicate, Some(validated));
        }
        let replaces_root = self
            .manual_root_file
            .as_ref()
            .is_some_and(|root| paths_equal(root, &self.part_files[index]));
        self.part_files[index] = validated.clone();
        if replaces_root {
            self.manual_root_file = Some(validated.clone());
        }
        self.remember_input_directory(&validated);
        self.normalize_after_parts_changed();
        result(AddPartStatus::Added, Some(validated))
    }

    pub fn remove_part(&mut self, index: usize) -> bool {
        if index >= self.part_files.len() {
            return false;
        }
        self.part_files.remove(index);
        self.normalize_after_parts_changed();
        true
    }

    pub fn clear_parts(&mut self) {
        self.part_files.clear();
        self.manual_root_file = None;
        self.normalize_after_parts_changed();
    }

    pub fn choose_output_directory(&mut self, directory: impl AsRef<Path>) {
        let directory = directory.as_ref();
        if directory.as_os_str().is_empty() {
            self.use_automatic_output_directory();
        } else {
            self.output_directory = directory.to_path_buf();
            self.has_explicit_output_directory = true;
        }
    }

    pub fn use_automatic_output_directory(&mut self) {
        self.has_explicit_output_directory = false;
        self.recalculate_automatic_output_directory();
    }

    pub fn set_output_file_name(&mut self, file_name: impl Into<String>) {
        self.output_file_name = file_name.into();
    }

    pub fn set_root_mode(&mut self, mode: RootMode) {
        self.root_mode = mode;
        self.normalize_manual_root();
    }

    pub fn set_manual_root(&mut self, index: usize) -> bool {
        let Some(path) = self.part_files.get(index) else {
            return false;
        };
        self.manual_root_file = Some(path.clone());
        self.root_mode = RootMode::Manual;
        true
    }

    pub fn reset_preferences(
        &mut self,
        preferred_output_directory: Option<&Path>,
        root_mode: RootMode,
    ) {
        self.root_mode = root_mode;
        self.output_file_name.clear();
        let preferred = preferred_output_directory
            .filter(|path| path.is_dir())
            .map(Path::to_path_buf);
        self.has_explicit_output_directory = preferred.is_some();
        if let Some(preferred) = preferred {
            self.output_directory = preferred;
        } else {
            self.recalculate_automatic_output_directory();
        }
        self.normalize_manual_root();
    }

    pub fn create_request(&self, overwrite: bool) -> Option<MergeRequest> {
        if !self.is_ready() {
            return None;
        }
        Some(MergeRequest {
            input_files: self.part_files.clone(),
            output_directory: self.output_directory.clone(),
            output_file_name: (!self.output_file_name.trim().is_empty())
                .then(|| self.output_file_name.trim().to_owned()),
            root_selection: match self.root_mode {
                RootMode::Automatic => RootSelection::Automatic,
                RootMode::Manual => RootSelection::Manual(self.manual_root_file.clone()?),
            },
            overwrite,
        })
    }

    fn is_ready(&self) -> bool {
        (MINIMUM_PARTS..=MAXIMUM_PARTS).contains(&self.part_files.len())
            && self.part_files.iter().all(|path| {
                path.is_file()
                    && path
                        .extension()
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("cast"))
            })
            && !self.output_directory.as_os_str().is_empty()
            && (self.root_mode == RootMode::Automatic
                || self
                    .manual_root_file
                    .as_ref()
                    .is_some_and(|root| self.part_files.iter().any(|path| paths_equal(path, root))))
    }

    fn remember_input_directory(&mut self, file_path: &Path) {
        self.recent_input_directory = file_path
            .parent()
            .filter(|path| path.is_dir())
            .map(Path::to_path_buf);
    }

    fn normalize_after_parts_changed(&mut self) {
        self.normalize_manual_root();
        if !self.has_explicit_output_directory {
            self.recalculate_automatic_output_directory();
        }
    }

    fn normalize_manual_root(&mut self) {
        if self
            .manual_root_file
            .as_ref()
            .is_some_and(|root| !self.part_files.iter().any(|path| paths_equal(path, root)))
        {
            self.manual_root_file = None;
        }
        if self.root_mode == RootMode::Manual && self.manual_root_file.is_none() {
            self.manual_root_file = self.part_files.first().cloned();
        }
    }

    fn recalculate_automatic_output_directory(&mut self) {
        self.output_directory = self
            .part_files
            .first()
            .and_then(|path| path.parent())
            .map(|directory| directory.join("Merged Models"))
            .unwrap_or_default();
    }
}

fn validate_part(path: &Path) -> Result<PathBuf, AddPartStatus> {
    if path.as_os_str().is_empty() {
        return Err(AddPartStatus::InvalidPath);
    }
    if !path.is_file() {
        return Err(AddPartStatus::Missing);
    }
    if path
        .extension()
        .is_none_or(|extension| !extension.eq_ignore_ascii_case("cast"))
    {
        return Err(AddPartStatus::UnsupportedFormat);
    }
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        std::env::current_dir()
            .map(|directory| directory.join(path))
            .map_err(|_| AddPartStatus::InvalidPath)
    }
}

fn paths_equal(left: &Path, right: &Path) -> bool {
    identity_key(left) == identity_key(right)
}

fn result(status: AddPartStatus, file_path: Option<PathBuf>) -> AddPartResult {
    AddPartResult { status, file_path }
}
