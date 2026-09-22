//! The wgpu side: pipelines, buffers and the per-frame draw.
//!
//! Deliberately mirrors `vrage_render` in `D:\work\ttc\Aether_terrain`:
//! wgpu 27, per-bone data in a **uniform** buffer laid out like
//! `pipelines::figure::BoneData`, `vek` column-major matrices, and the same
//! `mipmap_filter` / `push_constant_ranges` spellings.

use anyhow::{anyhow, Context, Result};
use blockymodel::{BoneMatrix, Mat4, Mesh, Vec2};
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const MAX_SAMPLE_COUNT: u32 = 4;

/// The format's own ceiling: "models can have a maximum of 255 nodes".
pub const MAX_BONES: usize = 255;

/// Mirrors `Bone` in `shader.wgsl`, and `BoneData` in
/// `vrage_render::pipelines::figure`, plus a UV offset.
///
/// The UV offset is a `vec4` rather than a `vec2` on purpose. WGSL's *uniform*
/// address space requires every struct member to sit on a 16-byte boundary, so
/// a trailing `vec2` would be padded out and the array stride would silently
/// disagree with this `repr(C)` layout. A `vec4` makes both sides 144 bytes.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct BoneData {
    /// Bone matrix: `bone_world · T(shape.offset) · S(animated stretch)`.
    pub bone_mat: [[f32; 4]; 4],
    /// Normal matrix: the linear part inverted and transposed.
    pub normals_mat: [[f32; 4]; 4],
    /// Per-shape UV offset in `.xy`, already normalized to UV units.
    pub uv_offset: [f32; 4],
}

impl BoneData {
    pub fn new(bone: &BoneMatrix, inv_texture_size: Vec2<f32>) -> Self {
        Self {
            bone_mat: bone.bone_mat.into_col_arrays(),
            normals_mat: bone.normals_mat.into_col_arrays(),
            uv_offset: [
                bone.uv_offset.x * inv_texture_size.x,
                bone.uv_offset.y * inv_texture_size.y,
                0.0,
                0.0,
            ],
        }
    }
}

impl Default for BoneData {
    fn default() -> Self {
        Self {
            bone_mat: Mat4::<f32>::identity().into_col_arrays(),
            normals_mat: Mat4::<f32>::identity().into_col_arrays(),
            uv_offset: [0.0; 4],
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

/// Per-frame counters, surfaced in the window title.
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameStats {
    pub vertices: u32,
    pub triangles: u32,
    pub bones: u32,
    pub draw_calls: u32,
}

/// GPU resources for one model.
///
/// `bone_bind_group_layout`, `texture_view` and `sampler` are kept so the bind
/// group can be rebuilt when a caller swaps the mesh or texture through
/// [`Renderer::set_mesh`] / [`Renderer::set_texture`].
#[allow(dead_code)]
pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: (u32, u32),
    sample_count: u32,

    pipeline_single: wgpu::RenderPipeline,
    pipeline_double: wgpu::RenderPipeline,

    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,

    bone_bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    parts: Vec<GpuPart>,

    depth_view: wgpu::TextureView,
    msaa_view: Option<wgpu::TextureView>,

    pub stats: FrameStats,
}

struct GpuPart {
    bone_buffer: wgpu::Buffer,
    bone_bind_group: wgpu::BindGroup,
    texture_size: Vec2<f32>,
    vertex_buffer: wgpu::Buffer,
    index_buffer_single: wgpu::Buffer,
    index_buffer_double: wgpu::Buffer,
    index_count_single: u32,
    index_count_double: u32,
}

impl Renderer {
    pub fn new(
        window: std::sync::Arc<winit::window::Window>,
        source_parts: &[(&Mesh, &[u8], (u32, u32), usize)],
    ) -> Result<Self> {
        if source_parts.is_empty() {
            return Err(anyhow!("at least one model part is required"));
        }
        for (_, _, _, bone_count) in source_parts {
            if *bone_count > MAX_BONES {
                return Err(anyhow!(
                    "a model part has {bone_count} bones, more than the format's limit of {MAX_BONES}"
                ));
            }
        }

        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY | wgpu::Backends::SECONDARY,
            flags: wgpu::InstanceFlags::from_build_config().with_env(),
            backend_options: wgpu::BackendOptions::default(),
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
        });

        let surface = instance
            .create_surface(window)
            .context("creating a wgpu surface for the window")?;

        let adapter = futures_lite::future::block_on(instance.request_adapter(
            &wgpu::RequestAdapterOptionsBase {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ))
        .map_err(|err| anyhow!("no suitable GPU adapter: {err}"))?;

        let (device, queue) =
            futures_lite::future::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("blockyanim-device"),
                required_features: wgpu::Features::empty(),
                required_limits: adapter.limits(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            }))
            .context("requesting a wgpu device")?;

        let capabilities = surface.get_capabilities(&adapter);

        // Prefer an sRGB target so the texture's sRGB values survive the trip
        // to the display.
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(capabilities.formats[0]);

        let sample_count = if adapter
            .get_texture_format_features(format)
            .flags
            .sample_count_supported(MAX_SAMPLE_COUNT)
        {
            MAX_SAMPLE_COUNT
        } else {
            1
        };

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: capabilities
                .present_modes
                .iter()
                .copied()
                .find(|m| *m == wgpu::PresentMode::AutoVsync)
                .unwrap_or(capabilities.present_modes[0]),
            desired_maximum_frame_latency: 2,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: Vec::new(),
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera bind group"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let bone_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("model layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        // Pixel art: nearest sampling keeps the 256x128 atlas crisp, and the
        // UV inset in the mesh builder handles any residual bleeding.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("model sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("model pipeline layout"),
            bind_group_layouts: &[&camera_layout, &bone_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline_single = create_pipeline(
            &device,
            &pipeline_layout,
            &shader,
            config.format,
            sample_count,
            wgpu::Face::Back,
            "model single sided",
        );
        let pipeline_double = create_pipeline(
            &device,
            &pipeline_layout,
            &shader,
            config.format,
            sample_count,
            wgpu::Face::Front,
            "model double sided",
        );

        let attachments = Attachments::new(&device, &config, sample_count);

        let mut parts = Vec::with_capacity(source_parts.len());
        let mut vertices = 0;
        let mut triangles = 0;
        let mut bones = 0;
        for (mesh, texture_rgba, texture_size, bone_count) in source_parts {
            let texture_view = create_texture(&device, &queue, texture_rgba, *texture_size);
            let bone_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("part bones"),
                size: (MAX_BONES * std::mem::size_of::<BoneData>()) as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let bone_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("part bind group"),
                layout: &bone_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: bone_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            });
            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("part vertices"),
                contents: bytemuck::cast_slice(&mesh.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            parts.push(GpuPart {
                bone_buffer,
                bone_bind_group,
                texture_size: Vec2::new(texture_size.0 as f32, texture_size.1 as f32),
                vertex_buffer,
                index_buffer_single: create_index_buffer(
                    &device,
                    "part indices single sided",
                    &mesh.indices_single,
                ),
                index_buffer_double: create_index_buffer(
                    &device,
                    "part indices double sided",
                    &mesh.indices_double,
                ),
                index_count_single: mesh.indices_single.len() as u32,
                index_count_double: mesh.indices_double.len() as u32,
            });
            vertices += mesh.vertices.len() as u32;
            triangles += mesh.triangle_count() as u32;
            bones += *bone_count as u32;
        }

        Ok(Self {
            surface,
            device,
            queue,
            config,
            size: (width, height),
            sample_count,
            pipeline_single,
            pipeline_double,
            camera_buffer,
            camera_bind_group,
            bone_bind_group_layout,
            sampler,
            parts,
            depth_view: attachments.depth_view,
            msaa_view: attachments.msaa_view,
            stats: FrameStats {
                vertices,
                triangles,
                bones,
                draw_calls: 0,
            },
        })
    }

    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    pub fn sample_count(&self) -> u32 {
        self.sample_count
    }

    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.size = (width, height);
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        let attachments = Attachments::new(&self.device, &self.config, self.sample_count);
        self.depth_view = attachments.depth_view;
        self.msaa_view = attachments.msaa_view;
    }

    /// Uploads this frame's camera and bone data.
    pub fn update(&mut self, view_proj: Mat4<f32>, part_bones: &[&[BoneMatrix]]) {
        let camera = CameraUniform {
            view_proj: view_proj.into_col_arrays(),
        };
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&camera));

        for (part, bones) in self.parts.iter().zip(part_bones) {
            let inv_texture_size = Vec2::new(
                1.0 / part.texture_size.x.max(1.0),
                1.0 / part.texture_size.y.max(1.0),
            );
            let upload: Vec<BoneData> = bones
                .iter()
                .take(MAX_BONES)
                .map(|b| BoneData::new(b, inv_texture_size))
                .collect();
            if upload.is_empty() {
                continue;
            }
            self.queue
                .write_buffer(&part.bone_buffer, 0, bytemuck::cast_slice(&upload));
        }
    }

    /// Draws one frame.
    pub fn render(&mut self, background: wgpu::Color) -> Result<()> {
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            Err(wgpu::SurfaceError::Timeout | wgpu::SurfaceError::Other) => return Ok(()),
            Err(err) => return Err(anyhow!("could not acquire a surface texture: {err}")),
        };

        let surface_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // With MSAA the pass renders into the multisampled view and resolves
        // into the swap chain image; without it, straight to the swap chain.
        let (color_view, resolve_target) = match &self.msaa_view {
            Some(msaa) => (msaa, Some(&surface_view)),
            None => (&surface_view, None),
        };

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("model pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color_view,
                    depth_slice: None,
                    resolve_target,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(background),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            let mut draws = 0;
            pass.set_bind_group(0, &self.camera_bind_group, &[]);
            for part in &self.parts {
                pass.set_bind_group(1, &part.bone_bind_group, &[]);
                pass.set_vertex_buffer(0, part.vertex_buffer.slice(..));
                if part.index_count_single > 0 {
                    pass.set_pipeline(&self.pipeline_single);
                    pass.set_index_buffer(
                        part.index_buffer_single.slice(..),
                        wgpu::IndexFormat::Uint32,
                    );
                    pass.draw_indexed(0..part.index_count_single, 0, 0..1);
                    draws += 1;
                }
                if part.index_count_double > 0 {
                    pass.set_pipeline(&self.pipeline_double);
                    pass.set_index_buffer(
                        part.index_buffer_double.slice(..),
                        wgpu::IndexFormat::Uint32,
                    );
                    pass.draw_indexed(0..part.index_count_double, 0, 0..1);
                    draws += 1;
                }
            }

            self.stats.draw_calls = draws;
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
        Ok(())
    }
}

struct Attachments {
    depth_view: wgpu::TextureView,
    msaa_view: Option<wgpu::TextureView>,
}

impl Attachments {
    fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, sample_count: u32) -> Self {
        let size = wgpu::Extent3d {
            width: config.width.max(1),
            height: config.height.max(1),
            depth_or_array_layers: 1,
        };

        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth"),
            size,
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let msaa_view = (sample_count > 1).then(|| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("msaa color"),
                    size,
                    mip_level_count: 1,
                    sample_count,
                    dimension: wgpu::TextureDimension::D2,
                    format: config.format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default())
        });

        Self {
            depth_view: depth.create_view(&wgpu::TextureViewDescriptor::default()),
            msaa_view,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn create_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
    sample_count: u32,
    cull: wgpu::Face,
    label: &str,
) -> wgpu::RenderPipeline {
    let vertex_attributes = [
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x3,
            offset: 0,
            shader_location: 0,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x3,
            offset: 12,
            shader_location: 1,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x2,
            offset: 24,
            shader_location: 2,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Uint32,
            offset: 32,
            shader_location: 3,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32,
            offset: 36,
            shader_location: 4,
        },
    ];

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<blockymodel::Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &vertex_attributes,
            }],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(cull),
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: sample_count,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
        cache: None,
    })
}

fn create_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    rgba: &[u8],
    size: (u32, u32),
) -> wgpu::TextureView {
    let (width, height) = (size.0.max(1), size.1.max(1));
    let extent = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("model texture"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
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
        rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * width),
            rows_per_image: Some(height),
        },
        extent,
    );

    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

fn create_index_buffer(device: &wgpu::Device, label: &str, indices: &[u32]) -> wgpu::Buffer {
    // A zero-sized buffer is invalid, so keep a 4-byte stub when a model has
    // no single-sided (or no double-sided) geometry at all.
    let contents: &[u32] = if indices.is_empty() { &[0] } else { indices };
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: bytemuck::cast_slice(contents),
        usage: wgpu::BufferUsages::INDEX,
    })
}
