use crate::{preview_gpu, theme};
use eframe::egui::{self, Color32, RichText, Sense};
use model_merger_app_core::{Catalog, TextKey};
use model_merger_engine::PreviewError;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread::JoinHandle;

const PREVIEW_TRIANGLE_LIMIT: usize = 250_000;

pub struct PreviewSession {
    pub id: u64,
    pub open: bool,
    pub title: String,
    state: PreviewLoadState,
    yaw: f32,
    pitch: f32,
    zoom: f32,
    cancelled: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

enum PreviewLoadState {
    Loading(Receiver<Result<LoadedPreview, PreviewError>>),
    Ready(LoadedPreview),
    Failed(PreviewFailure),
}

enum PreviewFailure {
    Engine(PreviewError),
    WorkerStopped,
}

struct LoadedPreview {
    summary: PreviewSummary,
    gpu_model: Arc<preview_gpu::PreviewModel>,
}

struct PreviewSummary {
    model_name: String,
    source_mesh_count: usize,
    displayed_triangle_count: usize,
    is_simplified: bool,
}

impl PreviewSession {
    pub fn load(id: u64, path: &Path) -> Self {
        let title = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let path = path.to_path_buf();
        let (sender, receiver) = mpsc::channel();
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);
        let worker = std::thread::spawn(move || {
            let result = model_merger_engine::load_preview(&path, PREVIEW_TRIANGLE_LIMIT, || {
                worker_cancelled.load(Ordering::Acquire)
            })
            .map(|data| {
                let gpu_model = Arc::new(preview_gpu::PreviewModel::from_preview(&data));
                let summary = PreviewSummary {
                    model_name: data.model_name,
                    source_mesh_count: data.source_mesh_count,
                    displayed_triangle_count: data.displayed_triangle_count,
                    is_simplified: data.is_simplified,
                };
                LoadedPreview { summary, gpu_model }
            });
            let _ = sender.send(result);
        });
        Self {
            id,
            open: true,
            title,
            state: PreviewLoadState::Loading(receiver),
            yaw: -0.55,
            pitch: 0.35,
            zoom: 1.0,
            cancelled,
            worker: Some(worker),
        }
    }

    pub fn path_title(path: &Path, prefix: &str) -> String {
        format!("{prefix} — {}", short_name(path))
    }

    pub fn show(&mut self, ui: &mut egui::Ui, catalog: Catalog) {
        if ui.input(|input| {
            input.viewport().close_requested() || input.key_pressed(egui::Key::Escape)
        }) {
            self.open = false;
            return;
        }
        self.receive_preview();
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(theme::BACKGROUND)
                    .inner_margin(egui::Margin::same(16)),
            )
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if control(ui, catalog.text(TextKey::RotateLeft)).clicked() {
                        self.yaw -= 0.18;
                    }
                    if control(ui, catalog.text(TextKey::RotateRight)).clicked() {
                        self.yaw += 0.18;
                    }
                    if control(ui, catalog.text(TextKey::ZoomIn)).clicked() {
                        self.zoom = (self.zoom * 1.15).min(8.0);
                    }
                    if control(ui, catalog.text(TextKey::ZoomOut)).clicked() {
                        self.zoom = (self.zoom / 1.15).max(0.2);
                    }
                    if control(ui, catalog.text(TextKey::ResetView)).clicked() {
                        self.yaw = -0.55;
                        self.pitch = 0.35;
                        self.zoom = 1.0;
                    }
                    if control(ui, catalog.text(TextKey::Close)).clicked() {
                        self.open = false;
                    }
                });
                ui.label(
                    RichText::new(catalog.text(TextKey::PreviewInstructions))
                        .size(13.0)
                        .color(theme::SECONDARY),
                );
                ui.add_space(8.0);
                match &self.state {
                    PreviewLoadState::Loading(_) => {
                        ui.spinner();
                        ui.label(catalog.text(TextKey::PreviewLoading));
                        ui.ctx()
                            .request_repaint_after(std::time::Duration::from_millis(30));
                    }
                    PreviewLoadState::Failed(failure) => {
                        let message = match failure {
                            PreviewFailure::Engine(error) => {
                                crate::messages::preview_error(catalog, error)
                            }
                            PreviewFailure::WorkerStopped => {
                                catalog.text(TextKey::PreviewWorkerStopped).to_owned()
                            }
                        };
                        ui.colored_label(theme::DESTRUCTIVE, message);
                    }
                    PreviewLoadState::Ready(loaded) => {
                        let model_name = loaded.summary.model_name.clone();
                        let source_mesh_count = loaded.summary.source_mesh_count;
                        let displayed_triangle_count = loaded.summary.displayed_triangle_count;
                        let is_simplified = loaded.summary.is_simplified;
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(model_name).size(17.0));
                            ui.label(format!(
                                "{} {} · {} {}",
                                source_mesh_count,
                                catalog.text(TextKey::Meshes),
                                displayed_triangle_count,
                                catalog.text(TextKey::Triangles)
                            ));
                        });
                        if is_simplified {
                            ui.label(
                                RichText::new(catalog.text(TextKey::PreviewSimplified))
                                    .size(13.0)
                                    .color(theme::SECONDARY),
                            );
                        }
                        let available = ui.available_size();
                        let size = egui::vec2(available.x.max(320.0), available.y.max(300.0));
                        let (response, painter) = ui.allocate_painter(size, Sense::drag());
                        painter.rect_filled(response.rect, 6.0, Color32::from_rgb(241, 245, 249));
                        if response.dragged() {
                            let delta = ui.input(|input| input.pointer.delta());
                            self.yaw += delta.x * 0.01;
                            self.pitch = (self.pitch + delta.y * 0.01).clamp(-1.45, 1.45);
                        }
                        if response.hovered() {
                            let scroll = ui.input(|input| input.smooth_scroll_delta.y);
                            if scroll.abs() > f32::EPSILON {
                                self.zoom = (self.zoom * (1.0 + scroll * 0.0015)).clamp(0.2, 8.0);
                            }
                        }
                        if ui.input(|input| input.key_pressed(egui::Key::ArrowLeft)) {
                            self.yaw -= 0.12;
                        }
                        if ui.input(|input| input.key_pressed(egui::Key::ArrowRight)) {
                            self.yaw += 0.12;
                        }
                        if ui.input(|input| input.key_pressed(egui::Key::ArrowUp)) {
                            self.pitch = (self.pitch - 0.12).max(-1.45);
                        }
                        if ui.input(|input| input.key_pressed(egui::Key::ArrowDown)) {
                            self.pitch = (self.pitch + 0.12).min(1.45);
                        }
                        if ui.input(|input| {
                            input.key_pressed(egui::Key::Plus)
                                || input.key_pressed(egui::Key::Equals)
                        }) {
                            self.zoom = (self.zoom * 1.15).min(8.0);
                        }
                        if ui.input(|input| input.key_pressed(egui::Key::Minus)) {
                            self.zoom = (self.zoom / 1.15).max(0.2);
                        }
                        if ui.input(|input| input.key_pressed(egui::Key::R)) {
                            self.yaw = -0.55;
                            self.pitch = 0.35;
                            self.zoom = 1.0;
                        }
                        painter.add(preview_gpu::paint_callback(
                            response.rect,
                            self.id,
                            Arc::clone(&loaded.gpu_model),
                            self.yaw,
                            self.pitch,
                            self.zoom,
                        ));
                        painter.rect_stroke(
                            response.rect,
                            6.0,
                            egui::Stroke::new(1.0, theme::BORDER),
                            egui::StrokeKind::Inside,
                        );
                    }
                }
            });
    }

    fn receive_preview(&mut self) {
        let PreviewLoadState::Loading(receiver) = &self.state else {
            return;
        };
        match receiver.try_recv() {
            Ok(Ok(data)) => {
                self.finish_worker();
                self.state = PreviewLoadState::Ready(data);
            }
            Ok(Err(error)) => {
                self.finish_worker();
                let detail = error.to_string();
                crate::diagnostics::record_runtime_error("preview-error", &detail);
                self.state = PreviewLoadState::Failed(PreviewFailure::Engine(error));
            }
            Err(TryRecvError::Disconnected) => {
                self.finish_worker();
                self.state = PreviewLoadState::Failed(PreviewFailure::WorkerStopped);
            }
            Err(TryRecvError::Empty) => {}
        }
    }

    fn finish_worker(&mut self) {
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Drop for PreviewSession {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
        // Dropping a JoinHandle detaches the cancellable loader. Never wait for disk I/O or
        // decoding on the GUI thread when a preview window is dismissed.
        self.worker.take();
    }
}

fn control(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(egui::Button::new(label).min_size(egui::vec2(92.0, 44.0)))
}

fn short_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_title_keeps_the_selected_file_name() {
        assert_eq!(
            "Preview — body.cast",
            PreviewSession::path_title(Path::new("C:/models/body.cast"), "Preview")
        );
    }

    #[test]
    fn dropping_a_loading_preview_never_waits_for_the_worker() {
        let (sender, receiver) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(400));
            drop(sender);
        });
        let session = PreviewSession {
            id: 1,
            open: true,
            title: "slow.cast".to_owned(),
            state: PreviewLoadState::Loading(receiver),
            yaw: 0.0,
            pitch: 0.0,
            zoom: 1.0,
            cancelled: Arc::new(AtomicBool::new(false)),
            worker: Some(worker),
        };
        let started = std::time::Instant::now();

        drop(session);

        assert!(started.elapsed() < std::time::Duration::from_millis(100));
    }
}
