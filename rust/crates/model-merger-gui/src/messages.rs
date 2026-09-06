use model_merger_app_core::{AddPartStatus, Catalog, TaskError, TaskProgress, TaskState, TextKey};

pub(super) fn add_part_error(catalog: Catalog, status: AddPartStatus) -> &'static str {
    catalog.text(match status {
        AddPartStatus::InvalidPath | AddPartStatus::InvalidIndex => TextKey::AddPartInvalidPath,
        AddPartStatus::Missing => TextKey::AddPartMissing,
        AddPartStatus::UnsupportedFormat => TextKey::AddPartNotCast,
        AddPartStatus::Duplicate => TextKey::AddPartDuplicate,
        AddPartStatus::Full => TextKey::AddPartFull,
        AddPartStatus::Added => TextKey::Completed,
    })
}

pub(super) fn preview_error(catalog: Catalog, error: &model_merger_engine::PreviewError) -> String {
    use model_merger_engine::PreviewError;

    match error {
        PreviewError::InvalidTriangleLimit => catalog.text(TextKey::PreviewFailed).to_owned(),
        PreviewError::InvalidPath(path) => {
            localized_path(catalog.text(TextKey::AddPartInvalidPath), path)
        }
        PreviewError::MissingFile(path) => {
            localized_path(catalog.text(TextKey::AddPartMissing), path)
        }
        PreviewError::UnsupportedFormat(path) => {
            localized_path(catalog.text(TextKey::AddPartNotCast), path)
        }
        PreviewError::Io { path, .. } | PreviewError::ModelRead { path, .. } => {
            localized_path(catalog.text(TextKey::PreviewFailed), path)
        }
        PreviewError::NoGeometry(path) => {
            localized_path(catalog.text(TextKey::PreviewFailed), path)
        }
        PreviewError::Cancelled => catalog.text(TextKey::Cancelled).to_owned(),
    }
}

fn localized_path(message: &str, path: &std::path::Path) -> String {
    format!("{message}: {}", path.display())
}

pub(super) fn task_label(catalog: Catalog, state: Option<TaskState>) -> &'static str {
    catalog.text(match state {
        None => TextKey::NeedTwoToFifteen,
        Some(TaskState::Queued) => TextKey::Queued,
        Some(TaskState::Running) => TextKey::Running,
        Some(TaskState::Succeeded) => TextKey::Succeeded,
        Some(TaskState::Failed) => TextKey::Failed,
        Some(TaskState::Cancelled) => TextKey::Cancelled,
    })
}

pub(super) fn task_progress(catalog: Catalog, progress: &TaskProgress) -> String {
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

pub(super) fn task_error(catalog: Catalog, error: &TaskError) -> String {
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

pub(super) fn merge_warning(
    catalog: Catalog,
    warning: &model_merger_engine::MergeWarning,
) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use model_merger_app_core::AppLanguage;
    use std::path::PathBuf;

    #[test]
    fn merge_warnings_rerender_from_semantic_data() {
        let warning = model_merger_engine::MergeWarning {
            code: model_merger_engine::MergeWarningCode::NoAttachmentBone,
            model_name: "arm".to_owned(),
            root_model_name: "body".to_owned(),
        };

        let chinese = merge_warning(Catalog::new(AppLanguage::ChineseSimplified), &warning);
        let english = merge_warning(Catalog::new(AppLanguage::English), &warning);

        assert!(chinese.contains("arm"));
        assert!(chinese.contains("body"));
        assert_ne!(chinese, english);
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

        assert!(task_progress(catalog, &progress).contains("arm.cast"));
        let message = task_error(catalog, &error);
        assert!(message.contains("broken.cast"));
        assert!(message.contains("truncated face buffer"));
    }

    #[test]
    fn preview_errors_rerender_in_the_selected_language() {
        let error = model_merger_engine::PreviewError::MissingFile(PathBuf::from("missing.cast"));

        let chinese = preview_error(Catalog::new(AppLanguage::ChineseSimplified), &error);
        let english = preview_error(Catalog::new(AppLanguage::English), &error);

        assert!(chinese.contains("missing.cast"));
        assert!(english.contains("missing.cast"));
        assert_ne!(chinese, english);

        let read_error = model_merger_engine::PreviewError::ModelRead {
            path: PathBuf::from("broken.cast"),
            message: "truncated face buffer".to_owned(),
        };
        let message = preview_error(Catalog::new(AppLanguage::ChineseSimplified), &read_error);
        assert!(message.contains("broken.cast"));
        assert!(!message.contains("truncated face buffer"));
    }
}
