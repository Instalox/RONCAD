//! Real-wgpu rendering of 3D bodies into the egui viewport.
//!
//! Faces and edges are drawn into an offscreen color+depth target via
//! per-pixel shaders, then composited into the egui color attachment with
//! a fullscreen blit. All other viewport overlays (sketches, dimensions,
//! HUDs, the grid) continue to be painter-driven and naturally land on
//! top of the 3D output because the callback is inserted before them.

use std::collections::{HashMap, HashSet};

use bytemuck::{Pod, Zeroable};
use egui::Rect;
use egui_wgpu::{CallbackResources, CallbackTrait, ScreenDescriptor};
use glam::{DVec2, DVec3};
use roncad_core::{
    ids::BodyId,
    selection::{Selection, SelectionItem},
};
use roncad_geometry::{Feature, Project, SketchProfile, Workplane};
use roncad_rendering::{extrude_mesh, revolve_mesh, Camera2d, EdgeKind, FeatureMesh3d};

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const BODY_MSAA_SAMPLES: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Default)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    eye: [f32; 4],
    viewport_size_px: [f32; 4],
    light_key_dir: [f32; 4],
    light_key_color: [f32; 4],
    light_fill_dir: [f32; 4],
    light_fill_color: [f32; 4],
    light_back_dir: [f32; 4],
    light_back_color: [f32; 4],
    ambient_sky: [f32; 4],
    ambient_ground: [f32; 4],
    spec_params: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct FaceVertex {
    position: [f32; 3],
    normal: [f32; 3],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct EdgeInstance {
    start: [f32; 3],
    end: [f32; 3],
    color: [f32; 4],
    params: [f32; 2],
}

const FACE_VERTEX_SIZE: u64 = std::mem::size_of::<FaceVertex>() as u64;
const EDGE_INSTANCE_SIZE: u64 = std::mem::size_of::<EdgeInstance>() as u64;

/// Per-device GPU resources for body rendering. Inserted into
/// `egui_wgpu::Renderer::callback_resources` once at app startup.
pub struct BodyRenderResources {
    target_format: wgpu::TextureFormat,
    camera_buffer: wgpu::Buffer,
    scene_bind_group: wgpu::BindGroup,
    face_pipeline: wgpu::RenderPipeline,
    edge_pipeline: wgpu::RenderPipeline,
    blit_pipeline: wgpu::RenderPipeline,
    blit_bind_group_layout: wgpu::BindGroupLayout,
    blit_sampler: wgpu::Sampler,
    scene_key: Option<u64>,
    body_buffers: HashMap<BodyId, CachedBodyBuffers>,
    offscreen: Option<OffscreenTargets>,
}

struct OffscreenTargets {
    size: (u32, u32),
    color_msaa_view: wgpu::TextureView,
    color_resolve_view: wgpu::TextureView,
    depth_view: wgpu::TextureView,
    blit_bg: wgpu::BindGroup,
}

struct CachedBodyBuffers {
    revision: u64,
    selected: bool,
    face_buffer: Option<CachedVertexBuffer>,
    edge_buffer: Option<CachedVertexBuffer>,
}

struct CachedVertexBuffer {
    buffer: wgpu::Buffer,
    count: u32,
    capacity: u64,
}

impl BodyRenderResources {
    pub fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("roncad body shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("body_shader.wgsl").into()),
        });

        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("roncad camera uniform"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let scene_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("roncad scene bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(
                        std::mem::size_of::<CameraUniform>() as u64
                    ),
                },
                count: None,
            }],
        });
        let scene_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("roncad scene bg"),
            layout: &scene_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let scene_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("roncad scene pl"),
            bind_group_layouts: &[Some(&scene_bgl)],
            immediate_size: 0,
        });

        let face_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("roncad face pipeline"),
            layout: Some(&scene_pl),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_face"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: FACE_VERTEX_SIZE,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x3,
                        1 => Float32x3,
                        2 => Float32x4,
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_face"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: Some(wgpu::Face::Back),
                front_face: wgpu::FrontFace::Ccw,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: BODY_MSAA_SAMPLES,
                ..Default::default()
            },
            multiview_mask: None,
            cache: None,
        });

        let edge_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("roncad edge pipeline"),
            layout: Some(&scene_pl),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_edge"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: EDGE_INSTANCE_SIZE,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x3,
                        1 => Float32x3,
                        2 => Float32x4,
                        3 => Float32x2,
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_edge"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: wgpu::StencilState::default(),
                // The edge shader applies a small clip-space z offset so
                // contour quads win the depth fight with coplanar faces.
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: BODY_MSAA_SAMPLES,
                ..Default::default()
            },
            multiview_mask: None,
            cache: None,
        });

        let blit_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("roncad blit bgl"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let blit_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("roncad blit pl"),
            bind_group_layouts: &[Some(&blit_bind_group_layout)],
            immediate_size: 0,
        });

        let blit_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("roncad blit pipeline"),
            layout: Some(&blit_pl),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_blit"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_blit"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let blit_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("roncad blit sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        Self {
            target_format,
            camera_buffer,
            scene_bind_group,
            face_pipeline,
            edge_pipeline,
            blit_pipeline,
            blit_bind_group_layout,
            blit_sampler,
            scene_key: None,
            body_buffers: HashMap::new(),
            offscreen: None,
        }
    }

    fn ensure_offscreen(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        if let Some(off) = &self.offscreen {
            if off.size == size {
                return;
            }
        }
        let color_msaa = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("roncad offscreen color msaa"),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: BODY_MSAA_SAMPLES,
            dimension: wgpu::TextureDimension::D2,
            format: self.target_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let color_resolve = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("roncad offscreen color resolve"),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.target_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("roncad offscreen depth"),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: BODY_MSAA_SAMPLES,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let color_msaa_view = color_msaa.create_view(&wgpu::TextureViewDescriptor::default());
        let color_resolve_view = color_resolve.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());
        let blit_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("roncad blit bg"),
            layout: &self.blit_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&color_resolve_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.blit_sampler),
                },
            ],
        });
        self.offscreen = Some(OffscreenTargets {
            size,
            color_msaa_view,
            color_resolve_view,
            depth_view,
            blit_bg,
        });
    }

    fn ensure_body_buffers(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        body: &SceneBody,
    ) {
        let cached = self.body_buffers.get(&body.body_id);
        if cached
            .is_some_and(|entry| entry.revision == body.revision && entry.selected == body.selected)
        {
            return;
        }

        let (face_vertices, edge_instances) = body.build_vertices();
        let entry = self
            .body_buffers
            .entry(body.body_id)
            .or_insert_with(CachedBodyBuffers::default);
        entry.revision = body.revision;
        entry.selected = body.selected;
        write_cached_vertex_buffer(
            device,
            queue,
            &mut entry.face_buffer,
            &face_vertices,
            FACE_VERTEX_SIZE,
            "roncad body face vb",
        );
        write_cached_vertex_buffer(
            device,
            queue,
            &mut entry.edge_buffer,
            &edge_instances,
            EDGE_INSTANCE_SIZE,
            "roncad body edge vb",
        );
    }
}

impl Default for CachedBodyBuffers {
    fn default() -> Self {
        Self {
            revision: 0,
            selected: false,
            face_buffer: None,
            edge_buffer: None,
        }
    }
}

fn write_cached_vertex_buffer<T: Pod>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    slot: &mut Option<CachedVertexBuffer>,
    data: &[T],
    stride: u64,
    label: &'static str,
) {
    if data.is_empty() {
        *slot = None;
        return;
    }

    let needed = (data.len() as u64) * stride;
    let needs_realloc = match slot.as_ref() {
        Some(buffer) => buffer.capacity < needed,
        None => true,
    };
    if needs_realloc {
        let capacity = needed.next_power_of_two().max(4096);
        *slot = Some(CachedVertexBuffer {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: capacity,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            count: 0,
            capacity,
        });
    }

    let buffer = slot.as_mut().expect("buffer allocated above");
    buffer.count = data.len() as u32;
    queue.write_buffer(&buffer.buffer, 0, bytemuck::cast_slice(data));
}

struct SceneBody {
    body_id: BodyId,
    revision: u64,
    selected: bool,
    features: Vec<SceneFeature>,
}

enum SceneFeature {
    Extrude {
        profile: SketchProfile,
        distance_mm: f64,
        workplane: WorkplaneTransform,
    },
    Revolve {
        profile: SketchProfile,
        axis_origin: DVec2,
        axis_dir: DVec2,
        angle_rad: f64,
        workplane: WorkplaneTransform,
    },
}

#[derive(Clone, Copy)]
struct WorkplaneTransform {
    origin: DVec3,
    u: DVec3,
    v: DVec3,
    normal: DVec3,
}

impl From<&Workplane> for WorkplaneTransform {
    fn from(workplane: &Workplane) -> Self {
        let u = workplane.u.normalize_or_zero();
        let v = workplane.v.normalize_or_zero();
        let normal = u.cross(v).normalize_or_zero();
        Self {
            origin: workplane.origin,
            u,
            v,
            normal,
        }
    }
}

impl WorkplaneTransform {
    fn local_position(&self, position: DVec3) -> DVec3 {
        self.origin + self.u * position.x + self.v * position.y + self.normal * position.z
    }
}

impl SceneBody {
    fn build_vertices(&self) -> (Vec<FaceVertex>, Vec<EdgeInstance>) {
        let face_color = body_face_color(self.selected);
        let mut face_vertices = Vec::<FaceVertex>::new();
        let mut edge_instances = Vec::<EdgeInstance>::new();

        for feature in &self.features {
            match feature {
                SceneFeature::Extrude {
                    profile,
                    distance_mm,
                    workplane,
                } => {
                    let mesh = extrude_mesh(profile, *distance_mm);
                    append_feature_mesh(
                        &mesh,
                        workplane,
                        self.selected,
                        face_color,
                        &mut face_vertices,
                        &mut edge_instances,
                    );
                }
                SceneFeature::Revolve {
                    profile,
                    axis_origin,
                    axis_dir,
                    angle_rad,
                    workplane,
                } => {
                    let mesh = revolve_mesh(profile, *axis_origin, *axis_dir, *angle_rad);
                    append_feature_mesh(
                        &mesh,
                        workplane,
                        self.selected,
                        face_color,
                        &mut face_vertices,
                        &mut edge_instances,
                    );
                }
            }
        }

        (face_vertices, edge_instances)
    }
}

/// Per-frame scene description shipped into the egui paint callback.
pub struct BodyCallback {
    pub scene_key: u64,
    bodies: Vec<SceneBody>,
    pub camera_uniform: CameraUniform,
    pub viewport_rect: Rect,
}

impl CallbackTrait for BodyCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        screen_descriptor: &ScreenDescriptor,
        egui_encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let resources: &mut BodyRenderResources = match callback_resources.get_mut() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let ppp = screen_descriptor.pixels_per_point.max(0.1);
        let width = (self.viewport_rect.width() * ppp).ceil().max(1.0) as u32;
        let height = (self.viewport_rect.height() * ppp).ceil().max(1.0) as u32;
        if width == 0 || height == 0 {
            return Vec::new();
        }

        if resources.scene_key != Some(self.scene_key) {
            resources.scene_key = Some(self.scene_key);
            resources.body_buffers.clear();
        }

        let active_body_ids = self
            .bodies
            .iter()
            .map(|body| body.body_id)
            .collect::<HashSet<_>>();
        resources
            .body_buffers
            .retain(|body_id, _| active_body_ids.contains(body_id));

        resources.ensure_offscreen(device, (width, height));
        queue.write_buffer(
            &resources.camera_buffer,
            0,
            bytemuck::bytes_of(&self.camera_uniform),
        );
        for body in &self.bodies {
            resources.ensure_body_buffers(device, queue, body);
        }

        let offscreen = resources.offscreen.as_ref().unwrap();
        let mut pass = egui_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("roncad body 3D pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &offscreen.color_msaa_view,
                depth_slice: None,
                resolve_target: Some(&offscreen.color_resolve_view),
                ops: wgpu::Operations {
                    // Transparent clear: areas without geometry let the egui
                    // backdrop (vignette + grid) show through after compositing.
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Discard,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &offscreen.depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        pass.set_bind_group(0, &resources.scene_bind_group, &[]);
        pass.set_pipeline(&resources.face_pipeline);
        for body in &self.bodies {
            let Some(buffers) = resources.body_buffers.get(&body.body_id) else {
                continue;
            };
            let Some(face_buffer) = &buffers.face_buffer else {
                continue;
            };
            pass.set_vertex_buffer(0, face_buffer.buffer.slice(..));
            pass.draw(0..face_buffer.count, 0..1);
        }

        pass.set_bind_group(0, &resources.scene_bind_group, &[]);
        pass.set_pipeline(&resources.edge_pipeline);
        for body in &self.bodies {
            let Some(buffers) = resources.body_buffers.get(&body.body_id) else {
                continue;
            };
            let Some(edge_buffer) = &buffers.edge_buffer else {
                continue;
            };
            pass.set_vertex_buffer(0, edge_buffer.buffer.slice(..));
            pass.draw(0..6, 0..edge_buffer.count);
        }

        drop(pass);
        Vec::new()
    }

    fn paint(
        &self,
        info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        callback_resources: &CallbackResources,
    ) {
        let resources: &BodyRenderResources = match callback_resources.get() {
            Some(r) => r,
            None => return,
        };
        let Some(offscreen) = &resources.offscreen else {
            return;
        };

        let vp = info.viewport_in_pixels();
        let screen_w = info.screen_size_px[0] as f32;
        let screen_h = info.screen_size_px[1] as f32;
        let left = (vp.left_px as f32).clamp(0.0, screen_w);
        let top = (vp.top_px as f32).clamp(0.0, screen_h);
        let width = (vp.width_px as f32).clamp(0.0, screen_w - left).max(1.0);
        let height = (vp.height_px as f32).clamp(0.0, screen_h - top).max(1.0);

        render_pass.set_viewport(left, top, width, height, 0.0, 1.0);
        render_pass.set_pipeline(&resources.blit_pipeline);
        render_pass.set_bind_group(0, &offscreen.blit_bg, &[]);
        render_pass.draw(0..3, 0..1);
    }
}

// ----------------------------------------------------------------------------
// Scene → callback conversion
// ----------------------------------------------------------------------------

/// Lighting and material constants. These mirror the painter-side polish so
/// the GPU and CPU paths render the same scene with the same look.
mod lighting {
    pub const KEY_DIR: [f32; 3] = [-0.42, 0.32, 0.85];
    pub const KEY_COLOR: [f32; 3] = [1.00, 0.97, 0.92];
    pub const KEY_INTENSITY: f32 = 0.92;

    pub const FILL_DIR: [f32; 3] = [0.58, -0.22, 0.55];
    pub const FILL_COLOR: [f32; 3] = [0.55, 0.70, 0.95];
    pub const FILL_INTENSITY: f32 = 0.36;

    pub const BACK_DIR: [f32; 3] = [0.20, -0.55, -0.65];
    pub const BACK_COLOR: [f32; 3] = [0.78, 0.86, 1.00];
    pub const BACK_INTENSITY: f32 = 0.45;

    pub const AMBIENT_SKY: [f32; 3] = [0.20, 0.25, 0.32];
    pub const AMBIENT_GROUND: [f32; 3] = [0.05, 0.06, 0.08];

    pub const SPEC_POWER: f32 = 56.0;
    pub const SPEC_WEIGHT: f32 = 0.32;
    pub const RIM_POWER: f32 = 2.6;
    pub const RIM_WEIGHT: f32 = 0.34;

    pub const BODY_BASE: [f32; 3] = [0.62, 0.66, 0.74];
    pub const BODY_SELECTED: [f32; 3] = [0.31, 0.66, 0.98];

    /// Edge colors in linear RGBA. Crease edges read as deep AO contour
    /// shadows; borders are softer to avoid overwhelming curved sections.
    pub const EDGE_CREASE: [f32; 4] = [0.04, 0.05, 0.06, 1.0];
    pub const EDGE_BORDER: [f32; 4] = [0.06, 0.07, 0.08, 0.65];
    pub const EDGE_CREASE_SELECTED: [f32; 4] = [0.31, 0.66, 0.98, 1.0];
    pub const EDGE_BORDER_SELECTED: [f32; 4] = [0.31, 0.66, 0.98, 0.55];
    pub const EDGE_CREASE_HALF_WIDTH_PX: f32 = 1.45;
    pub const EDGE_BORDER_HALF_WIDTH_PX: f32 = 1.05;
    pub const EDGE_SELECTED_BOOST_PX: f32 = 0.25;
    pub const EDGE_CREASE_FEATHER_PX: f32 = 0.95;
    pub const EDGE_BORDER_FEATHER_PX: f32 = 1.15;
}

fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len > 1e-12 {
        [v[0] / len, v[1] / len, v[2] / len]
    } else {
        [0.0, 0.0, 1.0]
    }
}

fn vec3_pad(v: [f32; 3], w: f32) -> [f32; 4] {
    [v[0], v[1], v[2], w]
}

fn body_face_color(selected: bool) -> [f32; 4] {
    let albedo = if selected {
        lighting::BODY_SELECTED
    } else {
        lighting::BODY_BASE
    };
    let emissive = if selected { 0.10 } else { 0.0 };
    [albedo[0], albedo[1], albedo[2], emissive]
}

fn body_edge_color(kind: EdgeKind, selected: bool) -> [f32; 4] {
    match (kind, selected) {
        (EdgeKind::Crease, false) => lighting::EDGE_CREASE,
        (EdgeKind::Crease, true) => lighting::EDGE_CREASE_SELECTED,
        (EdgeKind::Border, false) => lighting::EDGE_BORDER,
        (EdgeKind::Border, true) => lighting::EDGE_BORDER_SELECTED,
    }
}

fn body_edge_style(kind: EdgeKind, selected: bool) -> [f32; 2] {
    let (half_width, feather) = match kind {
        EdgeKind::Crease => (
            lighting::EDGE_CREASE_HALF_WIDTH_PX,
            lighting::EDGE_CREASE_FEATHER_PX,
        ),
        EdgeKind::Border => (
            lighting::EDGE_BORDER_HALF_WIDTH_PX,
            lighting::EDGE_BORDER_FEATHER_PX,
        ),
    };
    let half_width = if selected {
        half_width + lighting::EDGE_SELECTED_BOOST_PX
    } else {
        half_width
    };
    [half_width, feather]
}

fn append_feature_mesh(
    mesh: &FeatureMesh3d,
    workplane: &WorkplaneTransform,
    selected: bool,
    face_color: [f32; 4],
    face_vertices: &mut Vec<FaceVertex>,
    edge_instances: &mut Vec<EdgeInstance>,
) {
    let normal_origin = workplane.local_position(DVec3::ZERO);
    for triangle in &mesh.triangles {
        for vertex in &triangle.vertices {
            let position = workplane.local_position(vertex.position);
            let normal =
                (workplane.local_position(vertex.normal) - normal_origin).normalize_or_zero();
            face_vertices.push(FaceVertex {
                position: [position.x as f32, position.y as f32, position.z as f32],
                normal: [normal.x as f32, normal.y as f32, normal.z as f32],
                color: face_color,
            });
        }
    }

    for edge in &mesh.edges {
        let color = body_edge_color(edge.kind, selected);
        let params = body_edge_style(edge.kind, selected);
        let start = workplane.local_position(edge.start);
        let end = workplane.local_position(edge.end);
        edge_instances.push(EdgeInstance {
            start: [start.x as f32, start.y as f32, start.z as f32],
            end: [end.x as f32, end.y as f32, end.z as f32],
            color,
            params,
        });
    }
}

/// Build a `BodyCallback` from the project. Per-body body/feature descriptions
/// are gathered on the CPU, and only invalidated bodies rebuild/upload GPU
/// buffers inside the callback resource cache.
pub fn build_callback(
    project: &Project,
    selection: &Selection,
    camera: &Camera2d,
    rect_points: Rect,
    pixels_per_point: f32,
) -> BodyCallback {
    let viewport_size_px = DVec2::new(
        (rect_points.width() as f64 * pixels_per_point as f64).max(1.0),
        (rect_points.height() as f64 * pixels_per_point as f64).max(1.0),
    );

    let view_proj = camera.view_proj_f32(viewport_size_px);
    let eye = camera.eye_mm();

    let camera_uniform = CameraUniform {
        view_proj,
        eye: [eye.x as f32, eye.y as f32, eye.z as f32, 1.0],
        viewport_size_px: [
            viewport_size_px.x as f32,
            viewport_size_px.y as f32,
            0.0,
            0.0,
        ],
        light_key_dir: vec3_pad(normalize3(lighting::KEY_DIR), lighting::KEY_INTENSITY),
        light_key_color: vec3_pad(scale3(lighting::KEY_COLOR, lighting::KEY_INTENSITY), 0.0),
        light_fill_dir: vec3_pad(normalize3(lighting::FILL_DIR), lighting::FILL_INTENSITY),
        light_fill_color: vec3_pad(scale3(lighting::FILL_COLOR, lighting::FILL_INTENSITY), 0.0),
        light_back_dir: vec3_pad(normalize3(lighting::BACK_DIR), lighting::BACK_INTENSITY),
        light_back_color: vec3_pad(scale3(lighting::BACK_COLOR, lighting::BACK_INTENSITY), 0.0),
        ambient_sky: vec3_pad(lighting::AMBIENT_SKY, 0.0),
        ambient_ground: vec3_pad(lighting::AMBIENT_GROUND, 0.0),
        spec_params: [
            lighting::SPEC_POWER,
            lighting::SPEC_WEIGHT,
            lighting::RIM_POWER,
            lighting::RIM_WEIGHT,
        ],
    };

    let mut bodies = Vec::<SceneBody>::new();
    for (body_id, body) in project.bodies.iter() {
        let selected = selection.contains(&SelectionItem::Body(body_id));
        let features = project
            .body_features(body_id)
            .filter_map(|(_, feature)| scene_feature_from_project(project, feature))
            .collect();
        bodies.push(SceneBody {
            body_id,
            revision: body.mesh_revision(),
            selected,
            features,
        });
    }

    BodyCallback {
        scene_key: project.render_cache_key(),
        bodies,
        camera_uniform,
        viewport_rect: rect_points,
    }
}

fn scene_feature_from_project(project: &Project, feature: &Feature) -> Option<SceneFeature> {
    if !feature.is_profile_valid() {
        return None;
    }

    let workplane = feature
        .source_sketch()
        .and_then(|sketch_id| project.sketch_workplane(sketch_id))
        .or_else(|| project.workplanes.iter().next().map(|(_, plane)| plane))
        .map(WorkplaneTransform::from)?;

    Some(match feature {
        Feature::Extrude(feature) => SceneFeature::Extrude {
            profile: feature.profile.clone(),
            distance_mm: feature.distance_mm,
            workplane,
        },
        Feature::Revolve(feature) => SceneFeature::Revolve {
            profile: feature.profile.clone(),
            axis_origin: feature.axis_origin,
            axis_dir: feature.axis_dir,
            angle_rad: feature.angle_rad,
            workplane,
        },
    })
}

fn scale3(v: [f32; 3], s: f32) -> [f32; 3] {
    [v[0] * s, v[1] * s, v[2] * s]
}

#[cfg(test)]
mod tests {
    use naga::{
        front::wgsl,
        valid::{Capabilities, ValidationFlags, Validator},
    };

    #[test]
    fn body_shader_wgsl_is_valid() {
        let module =
            wgsl::parse_str(include_str!("body_shader.wgsl")).expect("body shader parses as WGSL");
        Validator::new(ValidationFlags::all(), Capabilities::all())
            .validate(&module)
            .expect("body shader validates");
    }
}
