use bytemuck::{Pod, Zeroable};
use eframe::egui;
use eframe::egui_wgpu::{self, CallbackResources, CallbackTrait, ScreenDescriptor};
use eframe::wgpu::{self, util::DeviceExt as _};
use model_merger_engine::PreviewData;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub const GPU_SAMPLE_COUNT: u16 = 4;

const SHADER: &str = r#"
struct ViewUniform {
    center_extent: vec4<f32>,
    view: vec4<f32>,
    model_color: vec4<f32>,
    grid_color: vec4<f32>,
    axis_color: vec4<f32>,
    grid: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> view_uniform: ViewUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
};

fn rotate(point: vec3<f32>, yaw: f32, pitch: f32) -> vec3<f32> {
    let sin_yaw = sin(yaw);
    let cos_yaw = cos(yaw);
    let yaw_point = vec3<f32>(
        point.x * cos_yaw + point.z * sin_yaw,
        point.y,
        -point.x * sin_yaw + point.z * cos_yaw,
    );
    let sin_pitch = sin(pitch);
    let cos_pitch = cos(pitch);
    return vec3<f32>(
        yaw_point.x,
        yaw_point.y * cos_pitch - yaw_point.z * sin_pitch,
        yaw_point.y * sin_pitch + yaw_point.z * cos_pitch,
    );
}

fn project(point: vec3<f32>) -> vec4<f32> {
    let rotated = rotate(point, view_uniform.view.x, view_uniform.view.y);
    let scale = 2.35 * view_uniform.view.z;
    let aspect = view_uniform.view.w;
    let fit = select(vec2<f32>(1.0, aspect), vec2<f32>(1.0 / aspect, 1.0), aspect >= 1.0);
    let depth = 2.8 - rotated.z;
    // Perspective depth in WebGPU's [0, 1] range, near = .01 and far = 100.
    return vec4<f32>(rotated.x * scale * fit.x, -rotated.y * scale * fit.y,
        100.0 / 99.99 * depth - 1.0 / 99.99, depth);
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    let normalized = (input.position - view_uniform.center_extent.xyz) / view_uniform.center_extent.w;
    var output: VertexOutput;
    output.clip_position = project(normalized);
    output.normal = rotate(input.normal, view_uniform.view.x, view_uniform.view.y);
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let normal = input.normal / max(length(input.normal), 0.0001);
    let light = clamp(abs(normal.z), 0.15, 1.0);
    let color = view_uniform.model_color.rgb * (0.45 + 0.55 * light);
    return vec4<f32>(color, 1.0);
}

struct GridOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) plane: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_grid(input: VertexInput) -> GridOutput {
    var output: GridOutput;
    let point = vec3<f32>(input.position.x, view_uniform.grid.x, input.position.z);
    output.clip_position = project(point);
    output.plane = point.xz;
    output.color = select(view_uniform.grid_color, view_uniform.axis_color, input.normal.x > 0.5);
    output.color.a *= input.normal.y;
    return output;
}

@fragment
fn fs_grid(input: GridOutput) -> @location(0) vec4<f32> {
    let fade = 1.0 - smoothstep(2.0, 8.0, length(input.plane));
    return vec4<f32>(input.color.rgb, input.color.a * fade);
}
"#;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ViewUniform {
    center_extent: [f32; 4],
    view: [f32; 4],
    model_color: [f32; 4],
    grid_color: [f32; 4],
    axis_color: [f32; 4],
    grid: [f32; 4],
}

pub struct PreviewModel {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    center: [f32; 3],
    extent: f32,
    floor: f32,
}

impl PreviewModel {
    pub fn from_preview(data: &PreviewData) -> Self {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for mesh in &data.meshes {
            let Some(base) = u32::try_from(vertices.len()).ok() else {
                break;
            };
            vertices.extend(
                mesh.positions
                    .iter()
                    .enumerate()
                    .map(|(index, position)| Vertex {
                        position: *position,
                        normal: mesh.normals.get(index).copied().unwrap_or([0.0, 0.0, 1.0]),
                    }),
            );
            indices.extend(
                mesh.triangle_indices
                    .iter()
                    .filter_map(|index| base.checked_add(*index)),
            );
        }
        let minimum = data.bounds.minimum;
        let maximum = data.bounds.maximum;
        let extent = (maximum[0] - minimum[0])
            .max(maximum[1] - minimum[1])
            .max(maximum[2] - minimum[2])
            .max(0.0001);
        Self {
            vertices,
            indices,
            center: [
                (minimum[0] + maximum[0]) * 0.5,
                (minimum[1] + maximum[1]) * 0.5,
                (minimum[2] + maximum[2]) * 0.5,
            ],
            extent,
            // Existing preview coordinates use positive Y down; keep the floor below the model.
            floor: (maximum[1] - minimum[1]) * 0.5 / extent + 0.025,
        }
    }
}

struct ModelBuffers {
    vertex: wgpu::Buffer,
    index: wgpu::Buffer,
    uniform: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    index_count: u32,
}

struct PreviewResources {
    pipeline: wgpu::RenderPipeline,
    grid_pipeline: wgpu::RenderPipeline,
    grid_vertex: wgpu::Buffer,
    grid_vertex_count: u32,
    uniform_layout: wgpu::BindGroupLayout,
    models: HashMap<u64, ModelBuffers>,
}

impl PreviewResources {
    fn new(render_state: &egui_wgpu::RenderState) -> Self {
        let device = &render_state.device;
        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("preview uniform layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("preview shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("preview pipeline layout"),
            bind_group_layouts: &[Some(&uniform_layout)],
            immediate_size: 0,
        });
        let mut pipeline_descriptor = wgpu::RenderPipelineDescriptor {
            label: Some("preview pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: render_state.target_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: u32::from(GPU_SAMPLE_COUNT),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        };
        let pipeline = device.create_render_pipeline(&pipeline_descriptor);
        pipeline_descriptor.label = Some("preview ground grid pipeline");
        pipeline_descriptor.vertex.entry_point = Some("vs_grid");
        let grid_targets = [Some(wgpu::ColorTargetState {
            format: render_state.target_format,
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
        })];
        if let Some(fragment) = &mut pipeline_descriptor.fragment {
            fragment.entry_point = Some("fs_grid");
            fragment.targets = &grid_targets;
        }
        pipeline_descriptor.primitive.topology = wgpu::PrimitiveTopology::LineList;
        if let Some(depth) = &mut pipeline_descriptor.depth_stencil {
            depth.depth_write_enabled = Some(false);
        }
        let grid_pipeline = device.create_render_pipeline(&pipeline_descriptor);
        let grid = grid_vertices();
        let grid_vertex = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("preview ground grid vertices"),
            contents: bytemuck::cast_slice(&grid),
            usage: wgpu::BufferUsages::VERTEX,
        });
        Self {
            pipeline,
            grid_pipeline,
            grid_vertex,
            grid_vertex_count: grid.len() as u32,
            uniform_layout,
            models: HashMap::new(),
        }
    }

    fn prepare_model(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        id: u64,
        model: &PreviewModel,
        uniform: ViewUniform,
    ) {
        if !self.models.contains_key(&id) {
            let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("preview view uniform"),
                contents: bytemuck::bytes_of(&uniform),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("preview view bind group"),
                layout: &self.uniform_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                }],
            });
            let vertex = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("preview vertices"),
                contents: bytemuck::cast_slice(&model.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let index = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("preview indices"),
                contents: bytemuck::cast_slice(&model.indices),
                usage: wgpu::BufferUsages::INDEX,
            });
            self.models.insert(
                id,
                ModelBuffers {
                    vertex,
                    index,
                    uniform: uniform_buffer,
                    bind_group,
                    index_count: model.indices.len() as u32,
                },
            );
        }
        if let Some(buffers) = self.models.get(&id) {
            queue.write_buffer(&buffers.uniform, 0, bytemuck::bytes_of(&uniform));
        }
    }
}

struct PreviewCallback {
    id: u64,
    model: Arc<PreviewModel>,
    uniform: ViewUniform,
}

impl CallbackTrait for PreviewCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &ScreenDescriptor,
        _egui_encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if let Some(resources) = callback_resources.get_mut::<PreviewResources>() {
            resources.prepare_model(device, queue, self.id, &self.model, self.uniform);
        }
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        callback_resources: &CallbackResources,
    ) {
        let Some(resources) = callback_resources.get::<PreviewResources>() else {
            return;
        };
        let Some(model) = resources.models.get(&self.id) else {
            return;
        };
        render_pass.set_pipeline(&resources.pipeline);
        render_pass.set_bind_group(0, &model.bind_group, &[]);
        render_pass.set_vertex_buffer(0, model.vertex.slice(..));
        render_pass.set_index_buffer(model.index.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..model.index_count, 0, 0..1);
        if self.uniform.grid[1] > 0.5 {
            render_pass.set_pipeline(&resources.grid_pipeline);
            render_pass.set_vertex_buffer(0, resources.grid_vertex.slice(..));
            render_pass.draw(0..resources.grid_vertex_count, 0..1);
        }
    }
}

pub fn install(render_state: &egui_wgpu::RenderState) {
    render_state
        .renderer
        .write()
        .callback_resources
        .insert(PreviewResources::new(render_state));
}

pub fn retain_models(
    render_state: &egui_wgpu::RenderState,
    active_ids: impl IntoIterator<Item = u64>,
) {
    let active: HashSet<_> = active_ids.into_iter().collect();
    let mut renderer = render_state.renderer.write();
    if let Some(resources) = renderer.callback_resources.get_mut::<PreviewResources>() {
        resources.models.retain(|id, _| active.contains(id));
    }
}

pub struct ViewSettings {
    pub yaw: f32,
    pub pitch: f32,
    pub zoom: f32,
    pub show_grid: bool,
    pub model_color: egui::Color32,
    pub grid_color: egui::Color32,
    pub axis_color: egui::Color32,
}

fn grid_vertices() -> Vec<Vertex> {
    let mut vertices = Vec::with_capacity(65 * 4);
    for index in -32i32..=32 {
        let coordinate = index as f32 * 0.25;
        let normal = [
            if index == 0 { 1.0 } else { 0.0 },
            if index % 4 == 0 { 0.38 } else { 0.18 },
            0.0,
        ];
        for position in [
            [-8.0, 0.0, coordinate],
            [8.0, 0.0, coordinate],
            [coordinate, 0.0, -8.0],
            [coordinate, 0.0, 8.0],
        ] {
            vertices.push(Vertex { position, normal });
        }
    }
    vertices
}

pub fn paint_callback(
    rect: egui::Rect,
    id: u64,
    model: Arc<PreviewModel>,
    settings: ViewSettings,
) -> egui::Shape {
    let aspect = (rect.width() / rect.height().max(1.0)).max(0.0001);
    egui_wgpu::Callback::new_paint_callback(
        rect,
        PreviewCallback {
            id,
            uniform: ViewUniform {
                center_extent: [
                    model.center[0],
                    model.center[1],
                    model.center[2],
                    model.extent,
                ],
                view: [settings.yaw, settings.pitch, settings.zoom, aspect],
                model_color: settings
                    .model_color
                    .to_array()
                    .map(|v| f32::from(v) / 255.0),
                grid_color: settings.grid_color.to_array().map(|v| f32::from(v) / 255.0),
                axis_color: settings.axis_color.to_array().map(|v| f32::from(v) / 255.0),
                grid: [
                    model.floor,
                    if settings.show_grid { 1.0 } else { 0.0 },
                    0.0,
                    0.0,
                ],
            },
            model,
        },
    )
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use model_merger_engine::{PreviewBounds, PreviewMesh};
    use std::path::PathBuf;

    #[test]
    fn perspective_and_grid_shader_validate_together() {
        let module = wgpu::naga::front::wgsl::parse_str(SHADER).expect("valid preview WGSL");
        wgpu::naga::valid::Validator::new(
            wgpu::naga::valid::ValidationFlags::all(),
            wgpu::naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .expect("valid model/grid projection and shader stages");
        let grid = grid_vertices();
        assert_eq!(260, grid.len());
        assert!(
            grid.iter()
                .all(|vertex| vertex.position.iter().all(|value| value.is_finite()))
        );
        assert_eq!(
            4,
            grid.iter().filter(|vertex| vertex.normal[0] == 1.0).count()
        );
    }

    #[test]
    fn preview_model_concatenates_mesh_indices_and_bounds() {
        let mesh = PreviewMesh {
            positions: vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]],
            normals: vec![[0.0, 0.0, 1.0]; 3],
            triangle_indices: vec![0, 1, 2],
        };
        let data = PreviewData {
            file_path: PathBuf::from("sample.cast"),
            model_name: "sample".to_owned(),
            source_mesh_count: 2,
            source_vertex_count: 6,
            source_triangle_count: 2,
            displayed_triangle_count: 2,
            is_simplified: false,
            bounds: PreviewBounds {
                minimum: [0.0, 0.0, 0.0],
                maximum: [2.0, 2.0, 0.0],
            },
            meshes: vec![mesh.clone(), mesh],
        };

        let model = PreviewModel::from_preview(&data);

        assert_eq!(6, model.vertices.len());
        assert_eq!(&[0, 1, 2, 3, 4, 5], model.indices.as_slice());
        assert_eq!([1.0, 1.0, 0.0], model.center);
        assert_eq!(2.0, model.extent);
        assert!((model.floor - 0.525).abs() < 0.0001);
    }
}
