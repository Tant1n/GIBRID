//! Отрисовка чанковых мешей на GPU.
//! Хранит vertex/index буферы для каждой секции и рендерит их все за один проход.

use std::collections::HashMap;

use gibrid_mesh::MeshRevision;
use wgpu::util::DeviceExt;

/// GPU-буферы одной секции чанка.
pub struct ChunkMeshGpu {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
    pub revision: MeshRevision,
}

/// Рендерер чанковых мешей: загрузка на GPU и отрисовка всех загруженных секций.
pub struct ChunkRenderer {
    meshes: HashMap<(i32, i32, i32), ChunkMeshGpu>,
    pipeline: wgpu::RenderPipeline,
}

impl ChunkRenderer {
    /// Создать рендерер чанков с pipelines.
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("chunk pipeline layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("chunk shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/chunk.wgsl").into()),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("chunk pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<gibrid_mesh::MeshVertex>() as wgpu::BufferAddress,
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
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
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
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        Self {
            meshes: HashMap::new(),
            pipeline,
        }
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

    /// Отрисовать все загруженные чанковые меши.
    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_pipeline(&self.pipeline);
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
