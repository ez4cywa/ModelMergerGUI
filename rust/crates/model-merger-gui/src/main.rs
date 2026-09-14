#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use model_merger_gui::{NativeApp, startup_viewport};
use rfd::{MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};
use std::panic::AssertUnwindSafe;
use std::path::Path;

fn main() {
    let log_directory = model_merger_gui::diagnostics::install_panic_hook()
        .unwrap_or_else(|_| model_merger_gui::diagnostics::log_directory());
    let result = std::panic::catch_unwind(AssertUnwindSafe(run));
    match result {
        Ok(Ok(())) => model_merger_gui::diagnostics::record_event("shutdown", "normal exit"),
        Ok(Err(error)) => report_failure(&log_directory, &error.to_string()),
        Err(_) => report_failure(
            &log_directory,
            "The application stopped unexpectedly. See the panic entry and backtrace in the log.",
        ),
    }
}

fn run() -> eframe::Result {
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png"))
        .expect("application icon should be valid PNG data");
    let options = eframe::NativeOptions {
        viewport: startup_viewport()
            .with_app_id("io.github.echo000.cast-model-merger")
            .with_icon(icon),
        depth_buffer: 24,
        multisampling: model_merger_gui::GPU_SAMPLE_COUNT,
        persist_window: false,
        ..Default::default()
    };
    eframe::run_native(
        "Cast Model Merger",
        options,
        Box::new(|creation| Ok(Box::new(NativeApp::new(creation)))),
    )
}

fn report_failure(log_directory: &Path, details: &str) {
    model_merger_gui::diagnostics::record_runtime_error("fatal", details);
    let message = format!(
        "Cast 模型合并器遇到错误，已尝试保存诊断日志。\n请将日志文件提供给开发者。是否打开日志文件夹？\n\nCast Model Merger encountered an error.\nOpen the diagnostics folder to share the logs?\n\n{}",
        log_directory.display()
    );
    let result = MessageDialog::new()
        .set_level(MessageLevel::Error)
        .set_title("Cast Model Merger")
        .set_description(message)
        .set_buttons(MessageButtons::YesNo)
        .show();
    if result == MessageDialogResult::Yes {
        let _ = std::process::Command::new("explorer.exe")
            .arg(log_directory)
            .spawn();
    }
}
