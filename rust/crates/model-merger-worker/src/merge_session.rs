use crate::output::{PROTOCOL_VERSION, WorkerOutput, emit, emit_error};
use crate::protocol::{IncomingCommand, ReaderMessage, WireMergeRequest};
use model_merger_engine::{
    MergeError, MergeObserver, MergeStage, MergeValidationCode, MergeWarningCode, prepare,
};
use serde_json::json;
use std::io::BufWriter;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, mpsc};

pub fn run(
    request: WireMergeRequest,
    receiver: &mpsc::Receiver<ReaderMessage>,
    cancelled: Arc<AtomicBool>,
    output: &WorkerOutput,
) -> i32 {
    let observer = JsonObserver { cancelled, output };
    let request = match request.into_engine_request() {
        Ok(request) => request,
        Err(message) => {
            emit_error(output, "protocol", &message);
            return 2;
        }
    };
    let prepared = match prepare(request, &observer) {
        Ok(prepared) => prepared,
        Err(error) => {
            emit_merge_error(output, &error);
            return 1;
        }
    };
    emit(
        output,
        &json!({
            "protocol": PROTOCOL_VERSION,
            "event": "prepared",
            "output_path": prepared.output_path(),
        }),
    );

    match receiver.recv() {
        Ok(ReaderMessage::Command(IncomingCommand::Execute)) => {
            match prepared.execute(&observer) {
                Ok(result) => {
                    let warnings: Vec<_> = result
                        .warnings
                        .iter()
                        .map(|warning| {
                            json!({
                                "code": warning_code_name(warning.code),
                                "model_name": warning.model_name,
                                "root_model_name": warning.root_model_name,
                            })
                        })
                        .collect();
                    emit(
                        output,
                        &json!({
                            "protocol": PROTOCOL_VERSION,
                            "event": "result",
                            "output_path": result.output_path,
                            "root_model_name": result.root_model_name,
                            "part_count": result.part_count,
                            "bone_count": result.bone_count,
                            "mesh_count": result.mesh_count,
                            "warnings": warnings,
                        }),
                    );
                }
                Err(error) => {
                    emit_merge_error(output, &error);
                    return 1;
                }
            }
            0
        }
        Ok(ReaderMessage::Command(IncomingCommand::Cancel)) => {
            emit_error(output, "cancelled", "merge was cancelled");
            1
        }
        Ok(ReaderMessage::Command(IncomingCommand::Prepare { .. })) => {
            emit_error(
                output,
                "protocol",
                "worker accepts only one prepare command",
            );
            2
        }
        Ok(ReaderMessage::Error(message)) => {
            emit_error(output, "protocol", &message);
            2
        }
        Err(_) => {
            emit_error(output, "protocol", "worker input closed before execute");
            2
        }
        Ok(ReaderMessage::Command(IncomingCommand::Preview { .. }))
        | Ok(ReaderMessage::Command(IncomingCommand::ReleasePreview)) => {
            emit_error(output, "protocol", "unexpected command during merge");
            2
        }
    }
}

struct JsonObserver<'a> {
    cancelled: Arc<AtomicBool>,
    output: &'a Mutex<BufWriter<std::io::Stdout>>,
}

impl MergeObserver for JsonObserver<'_> {
    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    fn on_progress(&self, stage: MergeStage, current: usize, total: usize, item: Option<&str>) {
        emit(
            self.output,
            &json!({
                "protocol": PROTOCOL_VERSION,
                "event": "progress",
                "stage": stage_name(stage),
                "current": current,
                "total": total,
                "item": item,
            }),
        );
    }
}

fn stage_name(stage: MergeStage) -> &'static str {
    match stage {
        MergeStage::Validating => "validating",
        MergeStage::Loading => "loading",
        MergeStage::SelectingRoot => "selecting_root",
        MergeStage::Merging => "merging",
        MergeStage::Saving => "saving",
        MergeStage::Verifying => "verifying",
        MergeStage::Completed => "completed",
    }
}

fn warning_code_name(code: MergeWarningCode) -> &'static str {
    match code {
        MergeWarningCode::NoAttachmentBone => "no_attachment_bone",
        MergeWarningCode::UnconnectedHierarchy => "unconnected_hierarchy",
    }
}

fn emit_merge_error(output: &WorkerOutput, error: &MergeError) {
    if let MergeError::Validation {
        code,
        message,
        path,
    } = error
    {
        emit(
            output,
            &json!({
                "protocol": PROTOCOL_VERSION,
                "event": "error",
                "code": "validation",
                "validation_code": validation_code_name(*code),
                "message": message,
                "path": path,
            }),
        );
        return;
    }
    if let MergeError::ModelRead { path, message } = error {
        emit(
            output,
            &json!({
                "protocol": PROTOCOL_VERSION,
                "event": "error",
                "code": "model_read",
                "message": message,
                "path": path,
                "format": "Cast",
            }),
        );
        return;
    }
    let code = match error {
        MergeError::Validation { .. } => unreachable!(),
        MergeError::Io { .. } => "io",
        MergeError::Codec(_) => "codec",
        MergeError::ModelRead { .. } => unreachable!(),
        MergeError::InvalidModel(_) => "invalid_model",
        MergeError::Cancelled => "cancelled",
    };
    emit_error(output, code, &error.to_string());
}

fn validation_code_name(code: MergeValidationCode) -> &'static str {
    match code {
        MergeValidationCode::InvalidPartCount => "invalid_part_count",
        MergeValidationCode::InvalidPath => "invalid_path",
        MergeValidationCode::MissingFile => "missing_file",
        MergeValidationCode::UnsupportedExtension => "unsupported_extension",
        MergeValidationCode::DuplicateFile => "duplicate_file",
        MergeValidationCode::InvalidOutputDirectory => "invalid_output_directory",
        MergeValidationCode::InvalidOutputFileName => "invalid_output_file_name",
        MergeValidationCode::OutputAlreadyExists => "output_already_exists",
        MergeValidationCode::ManualRootNotSelected => "manual_root_not_selected",
    }
}
