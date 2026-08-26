use model_merger_engine::{
    MergeError, MergeObserver, MergeRequest, MergeStage, MergeValidationCode, MergeWarningCode,
    RootSelection, prepare,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};

const PROTOCOL_VERSION: u32 = 1;

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let cancelled = Arc::new(AtomicBool::new(false));
    let (sender, receiver) = mpsc::channel();
    spawn_input_reader(Arc::clone(&cancelled), sender);
    let output = Mutex::new(BufWriter::new(std::io::stdout()));
    let observer = JsonObserver {
        cancelled: Arc::clone(&cancelled),
        output: &output,
    };

    let first = match receiver.recv() {
        Ok(message) => message,
        Err(_) => {
            emit_error(&output, "protocol", "worker input closed before prepare");
            return 2;
        }
    };
    let request = match first {
        ReaderMessage::Command(IncomingCommand::Prepare { request }) => request,
        ReaderMessage::Error(message) => {
            emit_error(&output, "protocol", &message);
            return 2;
        }
        _ => {
            emit_error(&output, "protocol", "first command must be prepare");
            return 2;
        }
    };
    let request = match request.into_engine_request() {
        Ok(request) => request,
        Err(message) => {
            emit_error(&output, "protocol", &message);
            return 2;
        }
    };
    let prepared = match prepare(request, &observer) {
        Ok(prepared) => prepared,
        Err(error) => {
            emit_merge_error(&output, &error);
            return 1;
        }
    };
    emit(
        &output,
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
                        &output,
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
                    emit_merge_error(&output, &error);
                    return 1;
                }
            }
            0
        }
        Ok(ReaderMessage::Command(IncomingCommand::Cancel)) => {
            emit_error(&output, "cancelled", "merge was cancelled");
            1
        }
        Ok(ReaderMessage::Command(IncomingCommand::Prepare { .. })) => {
            emit_error(
                &output,
                "protocol",
                "worker accepts only one prepare command",
            );
            2
        }
        Ok(ReaderMessage::Error(message)) => {
            emit_error(&output, "protocol", &message);
            2
        }
        Err(_) => {
            emit_error(&output, "protocol", "worker input closed before execute");
            2
        }
    }
}

fn spawn_input_reader(cancelled: Arc<AtomicBool>, sender: mpsc::Sender<ReaderMessage>) {
    std::thread::spawn(move || {
        let input = BufReader::new(std::io::stdin());
        for line in input.lines() {
            let line = match line {
                Ok(line) => line,
                Err(error) => {
                    let _ = sender.send(ReaderMessage::Error(format!(
                        "failed to read input: {error}"
                    )));
                    return;
                }
            };
            let envelope: CommandEnvelope = match serde_json::from_str(&line) {
                Ok(envelope) => envelope,
                Err(error) => {
                    let _ = sender.send(ReaderMessage::Error(format!(
                        "invalid JSON command: {error}"
                    )));
                    return;
                }
            };
            if envelope.protocol != PROTOCOL_VERSION {
                let _ = sender.send(ReaderMessage::Error(format!(
                    "unsupported protocol version {}",
                    envelope.protocol
                )));
                return;
            }
            if matches!(envelope.command, IncomingCommand::Cancel) {
                cancelled.store(true, Ordering::Release);
            }
            if sender
                .send(ReaderMessage::Command(envelope.command))
                .is_err()
            {
                return;
            }
        }
    });
}

#[derive(Debug, Deserialize)]
struct CommandEnvelope {
    protocol: u32,
    #[serde(flatten)]
    command: IncomingCommand,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
enum IncomingCommand {
    Prepare { request: WireMergeRequest },
    Execute,
    Cancel,
}

#[derive(Debug, Deserialize)]
struct WireMergeRequest {
    input_files: Vec<PathBuf>,
    output_directory: PathBuf,
    output_file_name: Option<String>,
    root_selection_mode: String,
    manual_root_file: Option<PathBuf>,
    overwrite: bool,
}

impl WireMergeRequest {
    fn into_engine_request(self) -> Result<MergeRequest, String> {
        let root_selection = match self.root_selection_mode.as_str() {
            "automatic" => RootSelection::Automatic,
            "manual" => RootSelection::Manual(
                self.manual_root_file
                    .ok_or_else(|| "manual root mode requires manual_root_file".to_owned())?,
            ),
            value => return Err(format!("unsupported root selection mode '{value}'")),
        };
        Ok(MergeRequest {
            input_files: self.input_files,
            output_directory: self.output_directory,
            output_file_name: self.output_file_name,
            root_selection,
            overwrite: self.overwrite,
        })
    }
}

enum ReaderMessage {
    Command(IncomingCommand),
    Error(String),
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

fn emit_merge_error(output: &Mutex<BufWriter<std::io::Stdout>>, error: &MergeError) {
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
        MergeError::AlreadyExecuted => "protocol",
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

fn emit_error(output: &Mutex<BufWriter<std::io::Stdout>>, code: &str, message: &str) {
    emit(
        output,
        &json!({
            "protocol": PROTOCOL_VERSION,
            "event": "error",
            "code": code,
            "message": message,
        }),
    );
}

fn emit<T: Serialize>(output: &Mutex<BufWriter<std::io::Stdout>>, event: &T) {
    let mut output = output
        .lock()
        .expect("worker output lock should not be poisoned");
    serde_json::to_writer(&mut *output, event).expect("worker event should serialize");
    writeln!(output).expect("worker event should write");
    output.flush().expect("worker event should flush");
}
