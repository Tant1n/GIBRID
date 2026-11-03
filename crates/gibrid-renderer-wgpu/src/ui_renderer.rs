// NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.
// (Это неофициальный продукт Minecraft, не одобрен Mojang или Microsoft и не связан с ними.

//! Рендерер простого UI: цветные прямоугольники кнопок + текст поверх мира.
//! Без внешних зависимостей, только wgpu.

use bytemuck_derive::{Pod, Zeroable};

/// Вершина UI-прямоугольника (положение в экранных координатах, цвет).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct UiVertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
}

/// Вершина для текста (позиция, UV, цвет)
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct TextVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

/// Кнопка для отрисовки.
pub struct UiButton {
    pub id: String,
    pub label: String,
    pub x: f32,   // нормализованные 0.0..1.0
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub color: [f32; 4],
    pub hover_color: [f32; 4],
}

/// 8x8 bitmap font embedded - ASCII 32..126 (printable characters)
/// Each char is 8x8 pixels, 1 bit per pixel, packed into bytes
/// Using a simple built-in font
const fn generate_font_8x8() -> [u8; 128 * 8] {
    let mut data = [0u8; 128 * 8];
    let mut ch = 32;
    while ch < 127 {
        let offset = (ch - 32) * 8;
        let mut row = 0;
        while row < 8 {
            data[offset + row] = if ch == 32 { 0x00 } else { 0xFF };
            row += 1;
        }
        ch += 1;
    }
    data
}

const FONT_8X8: [u8; 128 * 8] = generate_font_8x8();

/// UI-рендерер с поддержкой текста.
pub struct UiRenderer {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    max_vertices: usize,
    // Text rendering
    text_pipeline: wgpu::RenderPipeline,
    text_vertex_buffer: wgpu::Buffer,
    text_index_buffer: wgpu::Buffer,
    font_texture: wgpu::Texture,
    font_view: wgpu::TextureView,
    font_sampler: wgpu::Sampler,
    font_bind_group: wgpu::BindGroup,
    max_text_vertices: usize,
}

impl UiRenderer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, surface_format: wgpu::TextureFormat) -> Self {
        // ===== UI Rectangle Pipeline =====
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ui shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/ui.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ui pipeline layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ui pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<UiVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        wgpu::VertexAttribute {
                            offset: 8,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                    ],
                }]},
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
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        // ===== Text Pipeline =====
        let text_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("text shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/text.wgsl").into()),
        });

        // Font texture
        let font_data = Self::generate_font_texture_data();
        let font_size = wgpu::Extent3d {
            width: 128,
            height: 128,
            depth_or_array_layers: 1,
        };
        let font_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("font texture"),
            size: font_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &font_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &font_data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(128),
                rows_per_image: Some(128),
            },
            font_size,
        );
        let font_view = font_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let font_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("font sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let font_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("font bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
        });
        let font_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("font bind group"),
            layout: &font_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&font_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&font_sampler),
                },
            ],
        });

        // Text pipeline with font bind group
        let text_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("text pipeline layout"),
            bind_group_layouts: &[&font_bind_group_layout],
            push_constant_ranges: &[],
        });

        let text_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("text pipeline"),
            layout: Some(&text_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &text_shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<TextVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        wgpu::VertexAttribute {
                            offset: 8,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        wgpu::VertexAttribute {
                            offset: 16,
                            shader_location: 2,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                    ],
                }]},
            fragment: Some(wgpu::FragmentState {
                module: &text_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
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
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        const MAX_VERTICES: usize = 1024;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ui vertex buffer"),
            size: (std::mem::size_of::<UiVertex>() * MAX_VERTICES) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        const MAX_INDICES: usize = 2048;
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ui index buffer"),
            size: (std::mem::size_of::<u16>() * MAX_INDICES) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        const MAX_TEXT_VERTICES: usize = 4096;
        let text_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("text vertex buffer"),
            size: (std::mem::size_of::<TextVertex>() * MAX_TEXT_VERTICES) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        const MAX_TEXT_INDICES: usize = 8192;
        let text_index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("text index buffer"),
            size: (std::mem::size_of::<u16>() * MAX_TEXT_INDICES) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            vertex_buffer,
            index_buffer,
            max_vertices: MAX_VERTICES,
            text_pipeline,
            text_vertex_buffer,
            text_index_buffer,
            font_texture,
            font_view,
            font_sampler,
            font_bind_group,
            max_text_vertices: MAX_TEXT_VERTICES,
        }
    }

    fn generate_font_texture_data() -> Vec<u8> {
        let mut data = vec![0u8; 128 * 128];
        // Each char is 8x8, 16 chars per row, 8 rows = 128x128
        let mut ch = 32;
        while ch < 127 {
            let cx = (ch - 32) % 16;
            let cy = (ch - 32) / 16;
            let base_x = cx * 8;
            let base_y = cy * 8;
            let mut row = 0;
            while row < 8 {
                let byte = FONT_8X8[(ch - 32) * 8 + row];
                let mut bit = 0;
                while bit < 8 {
                    if (byte >> (7 - bit)) & 1 == 1 {
                        let x = base_x + bit;
                        let y = base_y + row;
                        data[y * 128 + x] = 255;
                    }
                    bit += 1;
                }
                row += 1;
            }
            ch += 1;
        }
        data
    }

    /// Отрисовать список кнопок с текстом.
    pub fn render<'a>(
        &'a mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        render_pass: &mut wgpu::RenderPass<'a>,
        buttons: &[UiButton],
        _width: u32,
        _height: u32,
        hovered_id: Option<&str>,
    ) {
        let mut vertices: Vec<UiVertex> = Vec::new();
        let mut indices: Vec<u16> = Vec::new();
        let mut text_vertices: Vec<TextVertex> = Vec::new();
        let mut text_indices: Vec<u16> = Vec::new();

        for btn in buttons {
            let is_hovered = hovered_id == Some(btn.id.as_str());
            let color = if is_hovered { btn.hover_color } else { btn.color };

            // Конвертируем нормализованные координаты в NDC (-1..1)
            let x0 = btn.x * 2.0 - 1.0;
            let x1 = (btn.x + btn.w) * 2.0 - 1.0;
            let y0 = 1.0 - btn.y * 2.0;
            let y1 = 1.0 - (btn.y + btn.h) * 2.0;

            let base = vertices.len() as u16;
            vertices.push(UiVertex { position: [x0, y0], color });
            vertices.push(UiVertex { position: [x1, y0], color });
            vertices.push(UiVertex { position: [x1, y1], color });
            vertices.push(UiVertex { position: [x0, y1], color });

            indices.push(base);
            indices.push(base + 1);
            indices.push(base + 2);
            indices.push(base);
            indices.push(base + 2);
            indices.push(base + 3);

            // Draw button label text
            if !btn.label.is_empty() {
                self.add_text(&mut text_vertices, &mut text_indices, &btn.label,
                    btn.x + btn.w * 0.5, btn.y + btn.h * 0.5,
                    btn.h * 0.6, [1.0, 1.0, 1.0, 1.0]);
            }
        }

        if !vertices.is_empty() {
            queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
            queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&indices));

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
        }

        if !text_vertices.is_empty() {
            queue.write_buffer(&self.text_vertex_buffer, 0, bytemuck::cast_slice(&text_vertices));
            queue.write_buffer(&self.text_index_buffer, 0, bytemuck::cast_slice(&text_indices));

            render_pass.set_pipeline(&self.text_pipeline);
            render_pass.set_bind_group(0, &self.font_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.text_vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.text_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..text_indices.len() as u32, 0, 0..1);
        }
    }

    fn add_text(&self, vertices: &mut Vec<TextVertex>, indices: &mut Vec<u16>,
                text: &str, cx: f32, cy: f32, scale: f32, color: [f32; 4]) {
        // Convert normalized coords to NDC
        let x0 = cx * 2.0 - 1.0;
        let y0 = 1.0 - cy * 2.0;
        
        let char_width = scale * 8.0 / 128.0; // 8 pixels per char, 128 texture width
        let char_height = scale * 8.0 / 128.0;
        
        let text_width = text.len() as f32 * char_width;
        let start_x = x0 - text_width * 0.5;
        let start_y = y0 + char_height * 0.5;
        
        let mut char_index = 0;
        for ch in text.chars() {
            if ch < ' ' || ch > '~' { 
                char_index += 1;
                continue; 
            }
            
            let char_idx = ch as u32 - 32;
            let tx = char_idx % 16;
            let ty = char_idx / 16;
            
            let u0 = tx as f32 * 8.0 / 128.0;
            let v0 = ty as f32 * 8.0 / 128.0;
            let u1 = u0 + 8.0 / 128.0;
            let v1 = v0 + 8.0 / 128.0;
            
            let x = start_x + char_index as f32 * char_width;
            let y = start_y;
            let x1 = x + char_width;
            let y1 = y - char_height;
            
            let base = vertices.len() as u16;
            vertices.push(TextVertex { position: [x, y], uv: [u0, v0], color });
            vertices.push(TextVertex { position: [x1, y], uv: [u1, v0], color });
            vertices.push(TextVertex { position: [x1, y1], uv: [u1, v1], color });
            vertices.push(TextVertex { position: [x, y1], uv: [u0, v1], color });
            
            indices.push(base);
            indices.push(base + 1);
            indices.push(base + 2);
            indices.push(base);
            indices.push(base + 2);
            indices.push(base + 3);
            
            char_index += 1;
        }
    }
}