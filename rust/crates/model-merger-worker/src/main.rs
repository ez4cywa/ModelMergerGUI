mod merge_session;
mod output;
mod preview_session;
mod protocol;

use output::{WorkerOutput, emit_error};
use protocol::{IncomingCommand, ReaderMessage, spawn_input_reader};
use std::io::BufWriter;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, mpsc};

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let cancelled = Arc::new(AtomicBool::new(false));
    let (sender, receiver) = mpsc::channel();
    spawn_input_reader(Arc::clone(&cancelled), sender);
    let output: WorkerOutput = Mutex::new(BufWriter::new(std::io::stdout()));

    let first = match receiver.recv() {
        Ok(message) => message,
        Err(_) => {
            emit_error(&output, "protocol", "worker input closed before prepare");
            return 2;
        }
    };
    match first {
        ReaderMessage::Command(IncomingCommand::Prepare { request }) => {
            merge_session::run(request, &receiver, cancelled, &output)
        }
        ReaderMessage::Command(IncomingCommand::Preview { request }) => {
            preview_session::run(request, &receiver, &cancelled, &output)
        }
        ReaderMessage::Error(message) => {
            emit_error(&output, "protocol", &message);
            2
        }
        _ => {
            emit_error(
                &output,
                "protocol",
                "first command must be prepare or preview",
            );
            2
        }
    }
}
