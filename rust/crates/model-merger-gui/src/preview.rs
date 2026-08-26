use crate::theme;
use eframe::egui::{self, Color32, RichText, Sense, Shape};
use model_merger_app_core::{Catalog, TextKey};
use model_merger_engine::{PreviewData, PreviewError};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread::JoinHandle;

const PREVIEW_TRIANGLE_LIMIT: usize = 75_000;

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
    Loading(Receiver<Result<PreviewData, PreviewError>>),
    Ready(PreviewData),
    Failed(PreviewFailure),
}

enum PreviewFailure {
    Engine,
    WorkerStopped,
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
                        let key = match failure {
                            PreviewFailure::Engine => TextKey::PreviewFailed,
                            PreviewFailure::WorkerStopped => TextKey::PreviewWorkerStopped,
                        };
                        ui.colored_label(theme::DESTRUCTIVE, catalog.text(key));
                    }
                    PreviewLoadState::Ready(data) => {
                        let model_name = data.model_name.clone();
                        let source_mesh_count = data.source_mesh_count;
                        let displayed_triangle_count = data.displayed_triangle_count;
                        let is_simplified = data.is_simplified;
                        let bounds = data.bounds;
                        let meshes = &data.meshes;
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
                        painter.rect_stroke(
                            response.rect,
                            6.0,
                            egui::Stroke::new(1.0, theme::BORDER),
                            egui::StrokeKind::Inside,
                        );
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
                        let shape = build_model_shape(
                            response.rect,
                            meshes,
                            bounds.minimum,
                            bounds.maximum,
                            self.yaw,
                            self.pitch,
                            self.zoom,
                        );
                        painter.add(shape);
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
            Ok(Err(_error)) => {
                self.finish_worker();
                self.state = PreviewLoadState::Failed(PreviewFailure::Engine);
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

fn build_model_shape(
    rect: egui::Rect,
    source_meshes: &[model_merger_engine::PreviewMesh],
    minimum: [f32; 3],
    maximum: [f32; 3],
    yaw: f32,
    pitch: f32,
    zoom: f32,
) -> Shape {
    let center = [
        (minimum[0] + maximum[0]) * 0.5,
        (minimum[1] + maximum[1]) * 0.5,
        (minimum[2] + maximum[2]) * 0.5,
    ];
    let extent = (maximum[0] - minimum[0])
        .max(maximum[1] - minimum[1])
        .max(maximum[2] - minimum[2])
        .max(0.0001);
    let scale = rect.width().min(rect.height()) * 0.42 * zoom / extent;
    let mut triangles = Vec::new();
    for source in source_meshes {
        let transformed: Vec<_> = source
            .positions
            .iter()
            .map(|point| rotate(subtract(*point, center), yaw, pitch))
            .collect();
        for triangle in source.triangle_indices.chunks_exact(3) {
            let Some(a) = transformed.get(triangle[0] as usize) else {
                continue;
            };
            let Some(b) = transformed.get(triangle[1] as usize) else {
                continue;
            };
            let Some(c) = transformed.get(triangle[2] as usize) else {
                continue;
            };
            triangles.push([*a, *b, *c]);
        }
    }
    // egui meshes do not expose a depth buffer. Draw farther triangles first so nearer
    // surfaces remain visible instead of depending on source-file face order.
    triangles.sort_by(|left, right| triangle_depth(*left).total_cmp(&triangle_depth(*right)));
    let mut mesh = egui::Mesh::default();
    for [a, b, c] in triangles {
        let shade = face_shade(a, b, c);
        let color = Color32::from_rgb(
            (45.0 + shade * 75.0) as u8,
            (93.0 + shade * 80.0) as u8,
            (150.0 + shade * 65.0) as u8,
        );
        let base = mesh.vertices.len() as u32;
        mesh.colored_vertex(project(rect, a, scale), color);
        mesh.colored_vertex(project(rect, b, scale), color);
        mesh.colored_vertex(project(rect, c, scale), color);
        mesh.add_triangle(base, base + 1, base + 2);
    }
    Shape::mesh(mesh)
}

fn triangle_depth(triangle: [[f32; 3]; 3]) -> f32 {
    (triangle[0][2] + triangle[1][2] + triangle[2][2]) / 3.0
}

fn subtract(point: [f32; 3], center: [f32; 3]) -> [f32; 3] {
    [
        point[0] - center[0],
        point[1] - center[1],
        point[2] - center[2],
    ]
}

fn rotate(point: [f32; 3], yaw: f32, pitch: f32) -> [f32; 3] {
    let (sin_yaw, cos_yaw) = yaw.sin_cos();
    let x = point[0] * cos_yaw + point[2] * sin_yaw;
    let z = -point[0] * sin_yaw + point[2] * cos_yaw;
    let (sin_pitch, cos_pitch) = pitch.sin_cos();
    [
        x,
        point[1] * cos_pitch - z * sin_pitch,
        point[1] * sin_pitch + z * cos_pitch,
    ]
}

fn project(rect: egui::Rect, point: [f32; 3], scale: f32) -> egui::Pos2 {
    egui::pos2(
        rect.center().x + point[0] * scale,
        rect.center().y - point[1] * scale,
    )
}

fn face_shade(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> f32 {
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let normal = [
        ab[1] * ac[2] - ab[2] * ac[1],
        ab[2] * ac[0] - ab[0] * ac[2],
        ab[0] * ac[1] - ab[1] * ac[0],
    ];
    let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2])
        .sqrt()
        .max(0.0001);
    (normal[2].abs() / length).clamp(0.15, 1.0)
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
    fn rotation_preserves_distance_from_origin() {
        let point = [2.0, -1.0, 3.0];
        let rotated = rotate(point, 0.7, -0.4);
        let before = point.iter().map(|value| value * value).sum::<f32>();
        let after = rotated.iter().map(|value| value * value).sum::<f32>();
        assert!((before - after).abs() < 0.0001);
    }

    #[test]
    fn preview_title_keeps_the_selected_file_name() {
        assert_eq!(
            "Preview — body.cast",
            PreviewSession::path_title(Path::new("C:/models/body.cast"), "Preview")
        );
    }

    #[test]
    fn triangle_depth_uses_the_average_camera_space_z() {
        let triangle = [[0.0, 0.0, -3.0], [1.0, 0.0, 0.0], [0.0, 1.0, 3.0]];

        assert_eq!(0.0, triangle_depth(triangle));
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
