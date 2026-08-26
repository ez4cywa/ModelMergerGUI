use crate::output::PROTOCOL_VERSION;
use model_merger_engine::{MergeRequest, RootSelection};
use serde::Deserialize;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

pub fn spawn_input_reader(cancelled: Arc<AtomicBool>, sender: mpsc::Sender<ReaderMessage>) {
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
pub enum IncomingCommand {
    Prepare { request: WireMergeRequest },
    Preview { request: WirePreviewRequest },
    Execute,
    ReleasePreview,
    Cancel,
}

#[derive(Debug, Deserialize)]
pub struct WirePreviewRequest {
    pub file_path: PathBuf,
    pub triangle_limit: usize,
}

#[derive(Debug, Deserialize)]
pub struct WireMergeRequest {
    input_files: Vec<PathBuf>,
    output_directory: PathBuf,
    output_file_name: Option<String>,
    root_selection_mode: String,
    manual_root_file: Option<PathBuf>,
    overwrite: bool,
}

impl WireMergeRequest {
    pub fn into_engine_request(self) -> Result<MergeRequest, String> {
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

pub enum ReaderMessage {
    Command(IncomingCommand),
    Error(String),
}
