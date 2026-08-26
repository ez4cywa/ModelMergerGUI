use crate::output::{PROTOCOL_VERSION, WorkerOutput, emit, emit_error};
use crate::protocol::{IncomingCommand, ReaderMessage, WirePreviewRequest};
use model_merger_engine::{PreviewError, load_preview};
use model_merger_worker::preview_payload::{PayloadError, PreviewPayload};
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

pub fn run(
    request: WirePreviewRequest,
    receiver: &mpsc::Receiver<ReaderMessage>,
    cancelled: &AtomicBool,
    output: &WorkerOutput,
) -> i32 {
    let preview = match load_preview(&request.file_path, request.triangle_limit, || {
        cancelled.load(Ordering::Acquire)
    }) {
        Ok(preview) => preview,
        Err(error) => {
            emit_preview_error(output, &error);
            return 1;
        }
    };
    let payload = match PreviewPayload::create(&preview, || cancelled.load(Ordering::Acquire)) {
        Ok(payload) => payload,
        Err(PayloadError::Cancelled) => {
            emit_error(output, "cancelled", "preview was cancelled");
            return 1;
        }
        Err(PayloadError::Io(error)) => {
            emit_error(output, "io", &error.to_string());
            return 1;
        }
    };
    emit(
        output,
        &json!({
            "protocol": PROTOCOL_VERSION,
            "event": "preview_result",
            "file_path": preview.file_path,
            "model_name": preview.model_name,
            "source_mesh_count": preview.source_mesh_count,
            "source_vertex_count": preview.source_vertex_count,
            "source_triangle_count": preview.source_triangle_count,
            "displayed_triangle_count": preview.displayed_triangle_count,
            "is_simplified": preview.is_simplified,
            "bounds": {
                "minimum": preview.bounds.minimum,
                "maximum": preview.bounds.maximum,
            },
            "payload_path": payload.path(),
            "payload_length": payload.length(),
        }),
    );

    match receiver.recv() {
        Ok(ReaderMessage::Command(IncomingCommand::ReleasePreview)) => 0,
        Ok(ReaderMessage::Command(IncomingCommand::Cancel)) => {
            emit_error(output, "cancelled", "preview was cancelled");
            1
        }
        Ok(ReaderMessage::Error(message)) => {
            emit_error(output, "protocol", &message);
            2
        }
        Ok(ReaderMessage::Command(_)) => {
            emit_error(output, "protocol", "preview result must be released");
            2
        }
        Err(_) => 0,
    }
}

fn emit_preview_error(output: &WorkerOutput, error: &PreviewError) {
    let (code, path) = match error {
        PreviewError::InvalidTriangleLimit => ("invalid_triangle_limit", None),
        PreviewError::InvalidPath(path) => ("invalid_path", Some(path)),
        PreviewError::MissingFile(path) => ("missing_file", Some(path)),
        PreviewError::UnsupportedFormat(path) => ("unsupported_format", Some(path)),
        PreviewError::Io { path, .. } => ("unreadable_model", Some(path)),
        PreviewError::ModelRead { path, .. } => ("unreadable_model", Some(path)),
        PreviewError::NoGeometry(path) => ("no_geometry", Some(path)),
        PreviewError::Cancelled => {
            emit_error(output, "cancelled", &error.to_string());
            return;
        }
    };
    emit(
        output,
        &json!({
            "protocol": PROTOCOL_VERSION,
            "event": "error",
            "code": "preview",
            "preview_code": code,
            "message": error.to_string(),
            "path": path,
        }),
    );
}
