// NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.
// (Это неофициальный продукт Minecraft, не одобрен Mojang или Microsoft и не связан с nimi.

//! Отрисовка чанковых мешей на GPU с текстурным атласом.
//! Хранит vertex/index буферы для каждой секции и рендерит их все за один проход.

use std::collections::HashMap;

use bytemuck_derive::{Pod, Zeroable};
use gibrid_mesh::{MeshRevision, MeshVertex};
use wgpu::util::DeviceExt;

/// GPU-буферы одной секции чанка.
pub struct ChunkMeshGpu {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
    pub revision: MeshRevision,
}

/// Uniform камеры: view-projection матрица (16 f32, column-major).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
}

/// Текстурный атлас на GPU (передаётся лаунчером).
pub struct GpuTextureAtlas {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub width: u32,
    pub height: u32,
}

/// Рендерер чанковых мешей: загрузка на GPU и отрисовка всех загруженных секций.
pub struct ChunkRenderer {
    meshes: HashMap<(i32, i32, i32), ChunkMeshGpu>,
    pipeline: wgpu::RenderPipeline,
    atlas: Option<GpuTextureAtlas>,
    surface_format: wgpu::TextureFormat,
    /// Uniform буфер камеры.
    camera_buffer: wgpu::Buffer,
    /// Bind group камеры (@group(1)).
    camera_bind_group: wgpu::BindGroup,
    /// Bind group layout камеры.
    camera_bind_group_layout: wgpu::BindGroupLayout,
    /// Текущая матрица view-proj.
    camera_view_proj: [[f32; 4]; 4],
}

impl ChunkRenderer {
    /// Создать рендерер чанков с pipelines.
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, surface_format: wgpu::TextureFormat) -> Self {
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("chunk pipeline layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("chunk shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/chunk.wgsl").into()),
        });

        // Создаём пустую 1x1 белую текстуру как заглушку, чтобы шейдер с текстурой заработал сразу.
        let white: [u8; 4] = [255, 255, 255, 255];
        let white_size = wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        };
        let white_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("default white texture"),
            size: white_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &white_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &white,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            white_size,
        );
        let white_view = white_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let white_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("default sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("atlas bind group layout"),
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

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("default atlas bind group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&white_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&white_sampler),
                },
            ],
        });

        // --- Камера: uniform буфер + bind group (@group(1)) ---
        let camera_uniform = CameraUniform {
            view_proj: identity_mat4(),
        };
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera uniform buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera bind group layout"),
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
            ],
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera bind group"),
            layout: &camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
            ],
        });

        // Pipeline layout с двумя bind group: atlas @group(0), camera @group(1)
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("chunk pipeline layout"),
            bind_group_layouts: &[&bind_group_layout, &camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = Self::build_pipeline(device, &pipeline_layout, &shader, surface_format);

        Self {
            meshes: HashMap::new(),
            pipeline,
            atlas: Some(GpuTextureAtlas {
                texture: white_texture,
                view: white_view,
                sampler: white_sampler,
                bind_group,
                bind_group_layout,
                width: 1,
                height: 1,
            }),
            surface_format,
            camera_buffer,
            camera_bind_group,
            camera_bind_group_layout,
            camera_view_proj: identity_mat4(),
        }
    }

    fn build_pipeline(
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        shader: &wgpu::ShaderModule,
        surface_format: wgpu::TextureFormat,
    ) -> wgpu::RenderPipeline {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("chunk pipeline"),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<MeshVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                        wgpu::VertexAttribute {
                            offset: 12,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                        wgpu::VertexAttribute {
                            offset: 24,
                            shader_location: 2,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        wgpu::VertexAttribute {
                            offset: 32,
                            shader_location: 3,
                            format: wgpu::VertexFormat::Unorm8x4,
                        },
                        wgpu::VertexAttribute {
                            offset: 36,
                            shader_location: 4,
                            format: wgpu::VertexFormat::Uint32,
                        },
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        })
    }

    /// Обновить матрицу камеры (view-proj) и записать в GPU буфер.
    pub fn update_camera(&mut self, queue: &wgpu::Queue, eye: [f32; 3], yaw_deg: f32, pitch_deg: f32, aspect: f32) {
        let view = look_at(eye, yaw_deg, pitch_deg);
        let proj = perspective(60.0_f32.to_radians(), aspect, 0.1, 1000.0);
        let view_proj = mat4_mul(proj, view);
        self.camera_view_proj = view_proj;

        let uniform = CameraUniform { view_proj };
        queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[uniform]),
        );
    }

    /// Отрендерить все загруженные чанковые меши.
    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_pipeline(&self.pipeline);
        if let Some(atlas) = &self.atlas {
            render_pass.set_bind_group(0, &atlas.bind_group, &[]);
        }
        render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
        for (_key, gpu) in self.meshes.iter() {
            render_pass.set_vertex_buffer(0, gpu.vertex_buffer.slice(..));
            render_pass.set_index_buffer(
                gpu.index_buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            render_pass.draw_indexed(0..gpu.index_count, 0, 0..1);
        }
    }
}

// ==================== Matemатика камеры ====================

/// Единичная матрица 4x4 в column-major порядке ([f32; 16]).
pub(crate) fn identity_mat4() -> [[f32; 4]; 4] {
    glam::Mat4::IDENTITY.to_cols_array_2d()
}

/// Умножение матриц A * B (оба column-major, как в GLSL/WGSL).
pub(crate) fn mat4_mul(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    (glam::Mat4::from_cols_array_2d(&a) * glam::Mat4::from_cols_array_2d(&b)).to_cols_array_2d()
}

/// LookAt матрица: камера смотрит из `eye` в направлении yaw/pitch.
pub(crate) fn look_at(eye: [f32; 3], yaw_deg: f32, pitch_deg: f32) -> [[f32; 4]; 4] {
    let yaw = yaw_deg.to_radians();
    let pitch = pitch_deg.to_radians();
    let eye_vec = glam::Vec3::from(eye);
    let forward = glam::Vec3::new(
        yaw.cos() * pitch.cos(),
        pitch.sin(),
        yaw.sin() * pitch.cos(),
    );
    let center = eye_vec + forward;
    let up = glam::Vec3::Y;
    glam::Mat4::look_at_rh(eye_vec, center, up).to_cols_array_2d()
}

/// Перспективная проекция (column-major, как в OpenGL/WGSL).
pub(crate) fn perspective(fov_y_rad: f32, aspect: f32, near: f32, far: f32) -> [[f32; 4]; 4] {
    glam::Mat4::perspective_rh(fov_y_rad, aspect, near, far).to_cols_array_2d()
}

impl ChunkRenderer {
    /// Установить текстурный атлас (полученный от лаунчера).
    pub fn set_texture_atlas(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        data: &[u8],
    ) {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("texture atlas"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("atlas sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let atlas_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("atlas bind group layout"),
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

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("atlas bind group"),
            layout: &atlas_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        self.atlas = Some(GpuTextureAtlas {
            texture,
            view,
            sampler,
            bind_group,
            bind_group_layout: atlas_bind_group_layout,
            width,
            height,
        });

        // Пересоздаём pipeline с bind group layout.
        let pipeline_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("atlas bind group layout"),
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("chunk pipeline layout"),
            bind_group_layouts: &[&pipeline_bind_group_layout, &self.camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("chunk shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/chunk.wgsl").into()),
        });

        self.pipeline = Self::build_pipeline(device, &pipeline_layout, &shader, self.surface_format);
    }

    /// Загрузить или обновить меш секции на GPU.
    pub fn upload_or_update(
        &mut self,
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        result: &gibrid_mesh::MeshResult,
    ) {
        let key = (
            result.mesh.section_x,
            result.mesh.section_y,
            result.mesh.section_z,
        );

        if self.meshes.contains_key(&key) && self.meshes[&key].revision >= result.revision {
            return; // уже актуально
        }

        let mesh = &result.mesh;

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("chunk vertex buffer"),
            contents: bytemuck::cast_slice(&mesh.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("chunk index buffer"),
            contents: bytemuck::cast_slice(&mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        self.meshes.insert(
            key,
            ChunkMeshGpu {
                vertex_buffer,
                index_buffer,
                index_count: mesh.index_count,
                revision: result.revision,
            },
        );
    }

    /// Удалить секцию из рендерера (например, при выгрузке чанка).
    pub fn remove(&mut self, section_x: i32, section_y: i32, section_z: i32) {
        self.meshes.remove(&(section_x, section_y, section_z));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_mesh_gpu_has_fields() {
        // Проверка только компиляции типа ChunkMeshGpu.
        let _mesh: Option<ChunkMeshGpu> = None;
        assert!(_mesh.is_none());
    }
}
