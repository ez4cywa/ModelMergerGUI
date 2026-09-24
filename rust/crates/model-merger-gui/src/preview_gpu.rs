use bytemuck::{Pod, Zeroable};
use eframe::egui;
use eframe::egui_wgpu::{self, CallbackResources, CallbackTrait, ScreenDescriptor};
use eframe::wgpu::{self, util::DeviceExt as _};
use model_merger_engine::{PreviewData, PreviewMaterialProfile};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
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

struct MaterialUniform {
    base_color: vec4<f32>,
    // x: materials enabled, y: has albedo, z: has NOG, w: has opacity.
    flags: vec4<f32>,
    // x: profile id, y: metalness, z: SSS weight, w: transmission.
    profile: vec4<f32>,
    // x: roughness offset, y: coat, z: normal strength, w: gloss map weight.
    params: vec4<f32>,
    // x: sheen, y: dielectric F0 (from IOR), z: transparent alpha base.
    extras: vec4<f32>,
};

@group(1) @binding(0)
var albedo_texture: texture_2d<f32>;
@group(1) @binding(1)
var albedo_sampler: sampler;
@group(1) @binding(2)
var nog_texture: texture_2d<f32>;
@group(1) @binding(3)
var nog_sampler: sampler;
@group(1) @binding(4)
var opacity_texture: texture_2d<f32>;
@group(1) @binding(5)
var opacity_sampler: sampler;
@group(1) @binding(6)
var<uniform> material_uniform: MaterialUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) view_position: vec3<f32>,
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

fn rotate_view(point: vec3<f32>) -> vec3<f32> {
    return rotate(point, view_uniform.view.x, view_uniform.view.y);
}

fn project_rotated(rotated: vec3<f32>) -> vec4<f32> {
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
    let rotated = rotate_view(normalized);
    var output: VertexOutput;
    output.clip_position = project_rotated(rotated);
    output.normal = rotate_view(input.normal);
    output.uv = input.uv;
    output.view_position = rotated;
    return output;
}

fn linear_to_srgb(color: vec3<f32>) -> vec3<f32> {
    return pow(clamp(color, vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(1.0 / 2.2));
}

// Derivative-based cotangent tangent frame for meshes without a tangent attribute.
fn cotangent_frame(normal: vec3<f32>, position: vec3<f32>, uv: vec2<f32>) -> mat3x3<f32> {
    let dp1 = dpdx(position);
    let dp2 = dpdy(position);
    let duv1 = dpdx(uv);
    let duv2 = dpdy(uv);
    let dp2perp = cross(dp2, normal);
    let dp1perp = cross(normal, dp1);
    let tangent = dp2perp * duv1.x + dp1perp * duv2.x;
    let bitangent = dp2perp * duv1.y + dp1perp * duv2.y;
    let scale = inverseSqrt(max(dot(tangent, tangent), dot(bitangent, bitangent)));
    return mat3x3<f32>(normalize(scale * tangent), normalize(scale * bitangent), normal);
}

// COD packed NOG: G/Alpha jointly encode the tangent-space normal, R is a gloss candidate.
fn decode_nog(sampled: vec4<f32>) -> vec3<f32> {
    let x = sampled.g + sampled.a - 1.0;
    let y = sampled.g - sampled.a;
    let z = 1.0 - abs(x) - abs(y);
    return normalize(vec3<f32>(x, y, z));
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let geometric_normal = input.normal / max(length(input.normal), 0.0001);
    let albedo_sample = textureSampleLevel(albedo_texture, albedo_sampler, input.uv, 0.0);
    let nog_sample = textureSampleLevel(nog_texture, nog_sampler, input.uv, 0.0);
    let opacity_sample = textureSampleLevel(opacity_texture, opacity_sampler, input.uv, 0.0);
    if (material_uniform.flags.x < 0.5) {
        let light = clamp(abs(geometric_normal.z), 0.15, 1.0);
        let color = view_uniform.model_color.rgb * (0.45 + 0.55 * light);
        return vec4<f32>(color, 1.0);
    }

    let profile_id = material_uniform.profile.x;
    let metalness = material_uniform.profile.y;
    let sss = material_uniform.profile.z;
    let transmission = material_uniform.profile.w;
    let roughness_offset = material_uniform.params.x;
    let coat = material_uniform.params.y;
    let normal_strength = material_uniform.params.z;
    let gloss_weight = material_uniform.params.w;
    let sheen = material_uniform.extras.x;
    let dielectric_f0 = material_uniform.extras.y;
    let alpha_base = material_uniform.extras.z;

    // Unresolved overlays contribute nothing in the research project; keep a
    // faint silhouette so the geometry stays visible in the preview.
    if (profile_id > 9.5) {
        return vec4<f32>(view_uniform.model_color.rgb, 0.15);
    }

    var albedo = material_uniform.base_color.rgb;
    var alpha_metal = 0.0;
    if (material_uniform.flags.y > 0.5) {
        albedo = albedo * albedo_sample.rgb;
        alpha_metal = albedo_sample.a;
    }

    var shaded_normal = geometric_normal;
    var gloss = 0.5;
    if (material_uniform.flags.z > 0.5) {
        let frame = cotangent_frame(geometric_normal, input.view_position, input.uv);
        let perturbed = normalize(frame * decode_nog(nog_sample));
        shaded_normal = normalize(mix(geometric_normal, perturbed, normal_strength));
        gloss = nog_sample.r;
    }
    // Roughness from the NOG gloss candidate; profiles with a zero Roughness
    // Map Weight ignore it and keep the offset-driven base value.
    var roughness = clamp(mix(0.5, 1.0 - gloss, gloss_weight) + roughness_offset, 0.04, 1.0);

    let view_dir = vec3<f32>(0.0, 0.0, 1.0);
    let NdotV = max(dot(shaded_normal, view_dir), 0.0001);
    let key_light = normalize(vec3<f32>(0.35, 0.55, 0.75));
    let fill_light = normalize(vec3<f32>(-0.6, -0.25, 0.45));

    // Weapon master: the albedo alpha is a metal candidate.
    let metal = clamp(
        metalness * select(1.0, step(0.1, alpha_metal), material_uniform.flags.y > 0.5),
        0.0,
        1.0,
    );

    // Wrap diffuse softened by the SSS weight (skin 0.22, oral 0.04).
    let wrap = sss * 0.45;
    let key_diffuse = clamp((dot(shaded_normal, key_light) + wrap) / (1.0 + wrap), 0.0, 1.0);
    let fill_diffuse = clamp((dot(shaded_normal, fill_light) + wrap) / (1.0 + wrap), 0.0, 1.0);
    var diffuse = albedo * (0.30 + 0.62 * key_diffuse + 0.22 * fill_diffuse);
    diffuse = diffuse * (1.0 - metal);

    let spec_power = mix(12.0, 140.0, 1.0 - roughness);
    let half_key = normalize(key_light + view_dir);
    let half_fill = normalize(fill_light + view_dir);
    let f0 = mix(vec3<f32>(dielectric_f0), albedo, metal);
    let key_spec = pow(max(dot(shaded_normal, half_key), 0.0), spec_power);
    let fill_spec = pow(max(dot(shaded_normal, half_fill), 0.0), spec_power);
    var specular = (key_spec * 0.62 + fill_spec * 0.22) * f0 * 8.0;
    // Clear-coat lobe (eye 0.8, tearline 0.6, oral 0.2, skin 0.04).
    let coat_spec = pow(max(dot(shaded_normal, half_key), 0.0), 180.0);
    specular = specular + vec3<f32>(coat_spec * coat * 0.5);
    // Sheen rim (cloth 0.15, hair 0.12).
    let rim = pow(1.0 - NdotV, 4.0);
    let lit = diffuse + specular + albedo * rim * sheen * 0.8;

    if (transmission > 0.5) {
        // Thin-wall transmission (optic glass IOR 1.46, cornea IOR 1.376):
        // tinted transmission with a broad viewing-angle sheen and fresnel
        // edge brightening, so dark lenses still read as glass.
        let fresnel = dielectric_f0 + (1.0 - dielectric_f0) * pow(1.0 - NdotV, 5.0);
        let sheen = pow(1.0 - NdotV, 2.0);
        let transmitted = albedo * (0.40 + 0.45 * key_diffuse);
        let highlight = key_spec * 1.4 + coat_spec * 0.8;
        let glassy = mix(transmitted, vec3<f32>(0.9), sheen * 0.4 + fresnel * 0.5)
            + vec3<f32>(highlight * 0.5);
        let alpha = clamp(alpha_base + sheen * 0.25 + fresnel * 0.5, 0.0, 1.0);
        return vec4<f32>(linear_to_srgb(glassy), alpha);
    }
    // Opacity masks (hair cards, tearline, eye atlas) feed alpha-to-coverage
    // for soft dithered edges instead of a binary cutout.
    var coverage: f32 = 1.0;
    if (material_uniform.flags.w > 0.5) {
        coverage = opacity_sample.r;
        if (coverage < 0.02) {
            discard;
        }
    }
    return vec4<f32>(linear_to_srgb(lit), coverage);
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
    output.clip_position = project_rotated(rotate_view(point));
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
    uv: [f32; 2],
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

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct MaterialUniform {
    base_color: [f32; 4],
    flags: [f32; 4],
    profile: [f32; 4],
    params: [f32; 4],
    extras: [f32; 4],
}

/// Texture roles a cast material slot can provide to the preview shader.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureRole {
    Albedo,
    Nog,
    Opacity,
}

/// CPU-side description of one material's preview textures.
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialSpec {
    pub profile: PreviewMaterialProfile,
    pub albedo: Option<PathBuf>,
    pub nog: Option<PathBuf>,
    pub opacity: Option<PathBuf>,
    pub base_color: Option<[f32; 4]>,
}

impl MaterialSpec {
    pub fn texture(&self, role: TextureRole) -> Option<&PathBuf> {
        match role {
            TextureRole::Albedo => self.albedo.as_ref(),
            TextureRole::Nog => self.nog.as_ref(),
            TextureRole::Opacity => self.opacity.as_ref(),
        }
    }
}

/// Per-profile shading parameters, transcribed from the shader project's
/// `scripts/profiles.json` controls plus the optic glass master group.
struct ProfileShading {
    id: f32,
    metalness: f32,
    sss: f32,
    transmission: f32,
    roughness_offset: f32,
    coat: f32,
    normal_strength: f32,
    gloss_weight: f32,
    sheen: f32,
    f0: f32,
    transparent: bool,
    alpha_base: f32,
}

fn profile_shading(profile: PreviewMaterialProfile) -> ProfileShading {
    let base = ProfileShading {
        id: 0.0,
        metalness: 0.0,
        sss: 0.0,
        transmission: 0.0,
        roughness_offset: 0.0,
        coat: 0.0,
        normal_strength: 1.0,
        gloss_weight: 1.0,
        sheen: 0.0,
        f0: 0.04,
        transparent: false,
        alpha_base: 1.0,
    };
    match profile {
        PreviewMaterialProfile::Generic => base,
        PreviewMaterialProfile::Weapon => ProfileShading {
            id: 1.0,
            metalness: 1.0,
            ..base
        },
        PreviewMaterialProfile::Glass => ProfileShading {
            id: 2.0,
            transmission: 1.0,
            f0: 0.036,
            transparent: true,
            alpha_base: 0.55,
            ..base
        },
        PreviewMaterialProfile::Skin => ProfileShading {
            id: 3.0,
            sss: 0.22,
            coat: 0.04,
            roughness_offset: 0.05,
            f0: 0.028,
            ..base
        },
        PreviewMaterialProfile::HairCard => ProfileShading {
            id: 4.0,
            roughness_offset: -0.05,
            normal_strength: 0.4,
            gloss_weight: 0.0,
            sheen: 0.12,
            ..base
        },
        PreviewMaterialProfile::Eye => ProfileShading {
            id: 5.0,
            coat: 0.8,
            roughness_offset: -0.36,
            normal_strength: 0.15,
            gloss_weight: 0.0,
            ..base
        },
        PreviewMaterialProfile::Cornea => ProfileShading {
            id: 6.0,
            transmission: 1.0,
            roughness_offset: -0.47,
            normal_strength: 0.0,
            gloss_weight: 0.0,
            f0: 0.027,
            transparent: true,
            alpha_base: 0.1,
            ..base
        },
        PreviewMaterialProfile::Tearline => ProfileShading {
            id: 7.0,
            coat: 0.6,
            roughness_offset: -0.4,
            normal_strength: 0.15,
            gloss_weight: 0.0,
            ..base
        },
        PreviewMaterialProfile::Oral => ProfileShading {
            id: 8.0,
            sss: 0.04,
            coat: 0.2,
            roughness_offset: 0.1,
            ..base
        },
        PreviewMaterialProfile::Cloth => ProfileShading {
            id: 9.0,
            roughness_offset: 0.1,
            sheen: 0.15,
            ..base
        },
        PreviewMaterialProfile::Overlay => ProfileShading {
            id: 10.0,
            transmission: 1.0,
            transparent: true,
            alpha_base: 0.15,
            ..base
        },
    }
}

/// A decoded texture ready for GPU upload, produced by the loader thread.
#[derive(Debug, Clone)]
pub struct TextureUpload {
    pub material: usize,
    pub role: TextureRole,
    pub width: u32,
    pub height: u32,
    pub rgba: Arc<[u8]>,
}

pub struct PreviewModel {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    segments: Vec<ModelSegment>,
    material_specs: Vec<MaterialSpec>,
    center: [f32; 3],
    extent: f32,
    floor: f32,
}

#[derive(Clone)]
struct ModelSegment {
    index_start: usize,
    index_count: usize,
    material: usize,
    transparent: bool,
}

impl PreviewModel {
    pub fn from_preview(data: &PreviewData) -> Self {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut segments = Vec::new();
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
                        uv: mesh.uvs.get(index).copied().unwrap_or([0.0, 0.0]),
                    }),
            );
            let index_start = indices.len();
            indices.extend(
                mesh.triangle_indices
                    .iter()
                    .filter_map(|index| base.checked_add(*index)),
            );
            let material = mesh.material_index.unwrap_or(usize::MAX);
            let transparent = data
                .materials
                .get(material)
                .map(|spec| profile_shading(spec.profile).transparent)
                .unwrap_or(false);
            segments.push(ModelSegment {
                index_start,
                index_count: indices.len() - index_start,
                material,
                transparent,
            });
        }
        let material_specs = data
            .materials
            .iter()
            .map(|material| MaterialSpec {
                profile: material.profile,
                albedo: material.albedo.clone(),
                nog: material.nog.clone(),
                opacity: material.opacity.clone(),
                base_color: material.base_color,
            })
            .collect();
        let minimum = data.bounds.minimum;
        let maximum = data.bounds.maximum;
        let extent = (maximum[0] - minimum[0])
            .max(maximum[1] - minimum[1])
            .max(maximum[2] - minimum[2])
            .max(0.0001);
        Self {
            vertices,
            indices,
            segments,
            material_specs,
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

    pub fn material_specs(&self) -> &[MaterialSpec] {
        &self.material_specs
    }
}

struct ModelBuffers {
    vertex: wgpu::Buffer,
    index: wgpu::Buffer,
    uniform: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    index_count: u32,
    segments: Vec<ModelSegment>,
    materials: Vec<MaterialGpu>,
    fallback_material: MaterialGpu,
}

struct MaterialGpu {
    uniform: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    views: [Option<wgpu::TextureView>; 3],
    has: [f32; 3],
    constant_base: Option<[f32; 4]>,
    profile: PreviewMaterialProfile,
}

struct PreviewResources {
    pipeline: wgpu::RenderPipeline,
    transparent_pipeline: wgpu::RenderPipeline,
    grid_pipeline: wgpu::RenderPipeline,
    grid_vertex: wgpu::Buffer,
    grid_vertex_count: u32,
    uniform_layout: wgpu::BindGroupLayout,
    material_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    fallback_views: [wgpu::TextureView; 3],
    models: HashMap<u64, ModelBuffers>,
}

fn texture_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn sampler_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

impl PreviewResources {
    fn new(render_state: &egui_wgpu::RenderState) -> Self {
        let device = &render_state.device;
        let queue = &render_state.queue;
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
        let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("preview material layout"),
            entries: &[
                texture_layout_entry(0),
                sampler_layout_entry(1),
                texture_layout_entry(2),
                sampler_layout_entry(3),
                texture_layout_entry(4),
                sampler_layout_entry(5),
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("preview material sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let fallback_views = [
            constant_texture_view(
                device,
                queue,
                wgpu::TextureFormat::Rgba8UnormSrgb,
                [255, 255, 255, 255],
            ),
            constant_texture_view(
                device,
                queue,
                wgpu::TextureFormat::Rgba8Unorm,
                [128, 128, 128, 128],
            ),
            constant_texture_view(
                device,
                queue,
                wgpu::TextureFormat::Rgba8Unorm,
                [255, 255, 255, 255],
            ),
        ];
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("preview shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("preview pipeline layout"),
            bind_group_layouts: &[Some(&uniform_layout), Some(&material_layout)],
            immediate_size: 0,
        });
        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2],
        };
        let mut pipeline_descriptor = wgpu::RenderPipelineDescriptor {
            label: Some("preview pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(vertex_layout)],
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
                // Opacity-masked materials (hair cards, tearline, eye) output
                // their coverage as alpha; MSAA dithers it into soft edges
                // while keeping depth writes.
                alpha_to_coverage_enabled: true,
            },
            multiview_mask: None,
            cache: None,
        };
        let pipeline = device.create_render_pipeline(&pipeline_descriptor);
        // Transparent variant for glass/cornea profiles: alpha blended and
        // depth-write disabled so they layer over opaque geometry.
        pipeline_descriptor.label = Some("preview transparent pipeline");
        pipeline_descriptor.multisample.alpha_to_coverage_enabled = false;
        let transparent_targets = [Some(wgpu::ColorTargetState {
            format: render_state.target_format,
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
        })];
        if let Some(fragment) = &mut pipeline_descriptor.fragment {
            fragment.targets = &transparent_targets;
        }
        if let Some(depth) = &mut pipeline_descriptor.depth_stencil {
            depth.depth_write_enabled = Some(false);
        }
        let transparent_pipeline = device.create_render_pipeline(&pipeline_descriptor);
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
            transparent_pipeline,
            grid_pipeline,
            grid_vertex,
            grid_vertex_count: grid.len() as u32,
            uniform_layout,
            material_layout,
            sampler,
            fallback_views,
            models: HashMap::new(),
        }
    }

    fn material_bind_group(
        &self,
        device: &wgpu::Device,
        views: &[Option<wgpu::TextureView>; 3],
        uniform_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        let albedo = views[0].as_ref().unwrap_or(&self.fallback_views[0]);
        let nog = views[1].as_ref().unwrap_or(&self.fallback_views[1]);
        let opacity = views[2].as_ref().unwrap_or(&self.fallback_views[2]);
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("preview material bind group"),
            layout: &self.material_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(albedo),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(nog),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(opacity),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        })
    }

    fn create_model_buffers(
        &self,
        device: &wgpu::Device,
        model: &PreviewModel,
        uniform: &ViewUniform,
    ) -> ModelBuffers {
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("preview view uniform"),
            contents: bytemuck::bytes_of(uniform),
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
        let mut materials = Vec::with_capacity(model.material_specs.len());
        for spec in &model.material_specs {
            let shading = profile_shading(spec.profile);
            let material_uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("preview material uniform"),
                contents: bytemuck::bytes_of(&MaterialUniform {
                    base_color: spec.base_color.unwrap_or(uniform.model_color),
                    flags: [1.0, 0.0, 0.0, 0.0],
                    profile: [
                        shading.id,
                        shading.metalness,
                        shading.sss,
                        shading.transmission,
                    ],
                    params: [
                        shading.roughness_offset,
                        shading.coat,
                        shading.normal_strength,
                        shading.gloss_weight,
                    ],
                    extras: [shading.sheen, shading.f0, shading.alpha_base, 0.0],
                }),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
            let bind_group =
                self.material_bind_group(device, &[None, None, None], &material_uniform);
            materials.push(MaterialGpu {
                uniform: material_uniform,
                bind_group,
                views: [None, None, None],
                has: [0.0; 3],
                constant_base: spec.base_color,
                profile: spec.profile,
            });
        }
        let fallback_uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("preview fallback material uniform"),
            contents: bytemuck::bytes_of(&MaterialUniform {
                base_color: uniform.model_color,
                flags: [0.0, 0.0, 0.0, 0.0],
                profile: [0.0; 4],
                params: [0.0; 4],
                extras: [0.0; 4],
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let fallback_bind_group =
            self.material_bind_group(device, &[None, None, None], &fallback_uniform);
        let fallback_material = MaterialGpu {
            uniform: fallback_uniform,
            bind_group: fallback_bind_group,
            views: [None, None, None],
            has: [0.0; 3],
            constant_base: None,
            profile: PreviewMaterialProfile::Generic,
        };
        ModelBuffers {
            vertex,
            index,
            uniform: uniform_buffer,
            bind_group,
            index_count: model.indices.len() as u32,
            segments: model.segments.clone(),
            materials,
            fallback_material,
        }
    }

    fn store_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        id: u64,
        upload: &TextureUpload,
    ) {
        let (views, uniform) = {
            let Some(buffers) = self.models.get_mut(&id) else {
                return;
            };
            let Some(material) = buffers.materials.get_mut(upload.material) else {
                return;
            };
            // Cornea shells unplug their base color; decoding an albedo
            // texture for them would be wasted work.
            if upload.role == TextureRole::Albedo
                && material.profile == PreviewMaterialProfile::Cornea
            {
                return;
            }
            let format = match upload.role {
                TextureRole::Albedo => wgpu::TextureFormat::Rgba8UnormSrgb,
                TextureRole::Nog | TextureRole::Opacity => wgpu::TextureFormat::Rgba8Unorm,
            };
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("preview material texture"),
                size: wgpu::Extent3d {
                    width: upload.width.max(1),
                    height: upload.height.max(1),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &upload.rgba,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(upload.width.max(1) * 4),
                    rows_per_image: Some(upload.height.max(1)),
                },
                wgpu::Extent3d {
                    width: upload.width.max(1),
                    height: upload.height.max(1),
                    depth_or_array_layers: 1,
                },
            );
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let slot = match upload.role {
                TextureRole::Albedo => 0,
                TextureRole::Nog => 1,
                TextureRole::Opacity => 2,
            };
            material.views[slot] = Some(view);
            material.has[slot] = 1.0;
            (material.views.clone(), material.uniform.clone())
        };
        let bind_group = self.material_bind_group(device, &views, &uniform);
        if let Some(buffers) = self.models.get_mut(&id)
            && let Some(material) = buffers.materials.get_mut(upload.material)
        {
            material.bind_group = bind_group;
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
            let buffers = self.create_model_buffers(device, model, &uniform);
            self.models.insert(id, buffers);
        }
        if let Some(buffers) = self.models.get_mut(&id) {
            queue.write_buffer(&buffers.uniform, 0, bytemuck::bytes_of(&uniform));
            queue.write_buffer(
                &buffers.fallback_material.uniform,
                0,
                bytemuck::bytes_of(&MaterialUniform {
                    base_color: uniform.model_color,
                    flags: [0.0, 0.0, 0.0, 0.0],
                    profile: [0.0; 4],
                    params: [0.0; 4],
                    extras: [0.0; 4],
                }),
            );
            for material in &buffers.materials {
                let shading = profile_shading(material.profile);
                // Cornea candidates unplug base color to white and ignore
                // albedo textures (profiles.json unplug_base_color). Glass
                // falls back to a dark blue-grey tint instead of the flat
                // palette color, which read as a black hole over dark
                // scope interiors.
                let cornea = material.profile == PreviewMaterialProfile::Cornea;
                let glass = material.profile == PreviewMaterialProfile::Glass;
                let base_color = if cornea {
                    [1.0, 1.0, 1.0, 1.0]
                } else if glass && material.constant_base.is_none() && material.has[0] == 0.0 {
                    [0.30, 0.34, 0.40, 1.0]
                } else {
                    let tint = material.constant_base.unwrap_or(uniform.model_color);
                    [tint[0], tint[1], tint[2], 1.0]
                };
                let material_uniform = MaterialUniform {
                    base_color,
                    flags: [
                        1.0,
                        if cornea { 0.0 } else { material.has[0] },
                        material.has[1],
                        // Only profiles that declare `requires: ["opacity"]`
                        // in profiles.json mask with the 4a texture; the eye
                        // atlas carries a smooth gradient there that would
                        // otherwise dither the eyeball away.
                        if matches!(
                            material.profile,
                            PreviewMaterialProfile::HairCard | PreviewMaterialProfile::Tearline
                        ) {
                            material.has[2]
                        } else {
                            0.0
                        },
                    ],
                    profile: [
                        shading.id,
                        shading.metalness,
                        shading.sss,
                        shading.transmission,
                    ],
                    params: [
                        shading.roughness_offset,
                        shading.coat,
                        shading.normal_strength,
                        shading.gloss_weight,
                    ],
                    extras: [shading.sheen, shading.f0, shading.alpha_base, 0.0],
                };
                queue.write_buffer(&material.uniform, 0, bytemuck::bytes_of(&material_uniform));
            }
        }
    }
}

fn constant_texture_view(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    format: wgpu::TextureFormat,
    rgba: [u8; 4],
) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("preview fallback texture"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4),
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

struct PreviewCallback {
    id: u64,
    model: Arc<PreviewModel>,
    uniform: ViewUniform,
    materials_enabled: bool,
    textures: Arc<[TextureUpload]>,
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
            for upload in self.textures.iter() {
                resources.store_texture(device, queue, self.id, upload);
            }
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
        if self.materials_enabled && !model.segments.is_empty() {
            for segment in &model.segments {
                if segment.transparent {
                    continue;
                }
                let material = model
                    .materials
                    .get(segment.material)
                    .unwrap_or(&model.fallback_material);
                render_pass.set_bind_group(1, &material.bind_group, &[]);
                render_pass.draw_indexed(
                    segment.index_start as u32..(segment.index_start + segment.index_count) as u32,
                    0,
                    0..1,
                );
            }
            if model.segments.iter().any(|segment| segment.transparent) {
                render_pass.set_pipeline(&resources.transparent_pipeline);
                for segment in &model.segments {
                    if !segment.transparent {
                        continue;
                    }
                    let material = model
                        .materials
                        .get(segment.material)
                        .unwrap_or(&model.fallback_material);
                    render_pass.set_bind_group(1, &material.bind_group, &[]);
                    render_pass.draw_indexed(
                        segment.index_start as u32
                            ..(segment.index_start + segment.index_count) as u32,
                        0,
                        0..1,
                    );
                }
            }
        } else {
            render_pass.set_bind_group(1, &model.fallback_material.bind_group, &[]);
            render_pass.draw_indexed(0..model.index_count, 0, 0..1);
        }
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
            vertices.push(Vertex {
                position,
                normal,
                uv: [0.0, 0.0],
            });
        }
    }
    vertices
}

pub fn paint_callback(
    rect: egui::Rect,
    id: u64,
    model: Arc<PreviewModel>,
    settings: ViewSettings,
    materials_enabled: bool,
    textures: Arc<[TextureUpload]>,
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
            materials_enabled,
            textures,
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

    fn sample_mesh() -> PreviewMesh {
        PreviewMesh {
            positions: vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]],
            normals: vec![[0.0, 0.0, 1.0]; 3],
            uvs: vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
            material_index: Some(0),
            triangle_indices: vec![0, 1, 2],
        }
    }

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
        let mesh = sample_mesh();
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
            materials: vec![model_merger_engine::PreviewMaterial {
                name: "mat".to_owned(),
                profile: model_merger_engine::PreviewMaterialProfile::Generic,
                albedo: Some(PathBuf::from("albedo.png")),
                nog: None,
                opacity: None,
                base_color: None,
            }],
        };

        let model = PreviewModel::from_preview(&data);

        assert_eq!(6, model.vertices.len());
        assert_eq!(&[0, 1, 2, 3, 4, 5], model.indices.as_slice());
        assert_eq!([1.0, 1.0, 0.0], model.center);
        assert_eq!(2.0, model.extent);
        assert!((model.floor - 0.525).abs() < 0.0001);
        assert_eq!(2, model.segments.len());
        assert!(model.segments.iter().all(|segment| segment.material == 0));
        assert!(model.segments.iter().all(|segment| !segment.transparent));
        assert_eq!(1, model.material_specs.len());
        assert_eq!(
            Some(PathBuf::from("albedo.png")),
            model.material_specs[0].albedo
        );
    }

    #[test]
    fn meshes_without_materials_fall_back_to_the_palette_segment() {
        let mut mesh = sample_mesh();
        mesh.material_index = None;
        let data = PreviewData {
            file_path: PathBuf::from("sample.cast"),
            model_name: "sample".to_owned(),
            source_mesh_count: 1,
            source_vertex_count: 3,
            source_triangle_count: 1,
            displayed_triangle_count: 1,
            is_simplified: false,
            bounds: PreviewBounds {
                minimum: [0.0, 0.0, 0.0],
                maximum: [2.0, 2.0, 0.0],
            },
            meshes: vec![mesh],
            materials: Vec::new(),
        };

        let model = PreviewModel::from_preview(&data);

        assert_eq!(1, model.segments.len());
        assert_eq!(usize::MAX, model.segments[0].material);
        assert!(!model.segments[0].transparent);
    }

    #[test]
    fn glass_and_cornea_materials_render_in_the_transparent_pass() {
        let mut glass_mesh = sample_mesh();
        glass_mesh.material_index = Some(0);
        let mut cornea_mesh = sample_mesh();
        cornea_mesh.material_index = Some(1);
        let mut weapon_mesh = sample_mesh();
        weapon_mesh.material_index = Some(2);
        let data = PreviewData {
            file_path: PathBuf::from("sample.cast"),
            model_name: "sample".to_owned(),
            source_mesh_count: 3,
            source_vertex_count: 9,
            source_triangle_count: 3,
            displayed_triangle_count: 3,
            is_simplified: false,
            bounds: PreviewBounds {
                minimum: [0.0, 0.0, 0.0],
                maximum: [2.0, 2.0, 0.0],
            },
            meshes: vec![glass_mesh, cornea_mesh, weapon_mesh],
            materials: vec![
                model_merger_engine::PreviewMaterial {
                    name: "wpn_optic_glass".to_owned(),
                    profile: model_merger_engine::PreviewMaterialProfile::Glass,
                    albedo: None,
                    nog: None,
                    opacity: None,
                    base_color: None,
                },
                model_merger_engine::PreviewMaterial {
                    name: "cornea_shell".to_owned(),
                    profile: model_merger_engine::PreviewMaterialProfile::Cornea,
                    albedo: None,
                    nog: None,
                    opacity: None,
                    base_color: None,
                },
                model_merger_engine::PreviewMaterial {
                    name: "wpn_rec".to_owned(),
                    profile: model_merger_engine::PreviewMaterialProfile::Weapon,
                    albedo: None,
                    nog: None,
                    opacity: None,
                    base_color: None,
                },
            ],
        };

        let model = PreviewModel::from_preview(&data);

        assert!(model.segments[0].transparent);
        assert!(model.segments[1].transparent);
        assert!(!model.segments[2].transparent);
    }
}
