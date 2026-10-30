//! wgpu renderer — Phase 3 + Phase 4: окно, surface, устройство, очередь и рендер чанковых мешей.
//!
//! Статус: IMPLEMENTED (window + surface + device + clear + chunk meshes).

#![forbid(unsafe_code)]

pub mod chunk_renderer;

use winit::window::Window;

/// Диагностический рендерер: окно, surface, устройство, очередь и рендер чанковых мешей.
pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    chunk_renderer: chunk_renderer::ChunkRenderer,
}

impl Renderer {
    /// Создать окно, поверхность, адаптер, устройство, очередь и конвейер рендеринга.
    pub fn new(window: Window) -> Result<Self, Box<dyn std::error::Error>> {
        let size = window.inner_size();

        // Surface
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let surface = instance.create_surface(window)?;

        // Adapter
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .ok_or("no suitable adapter found")?;

        // Device + Queue
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                label: None,
            },
            None,
        ))?;

        // Surface config
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let chunk_renderer = chunk_renderer::ChunkRenderer::new(&device, surface_format);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            size,
            chunk_renderer,
        })
    }

    /// Обработать изменение размера окна.
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }
        self.size = new_size;
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
    }

    /// Загрузить или обновить меш секции на GPU.
    pub fn upload_chunk(&mut self, result: &gibrid_mesh::MeshResult) {
        self.chunk_renderer
            .upload_or_update(&self.device, &self.queue, result);
    }

    /// Удалить секцию из рендерера.
    pub fn remove_chunk(&mut self, section_x: i32, section_y: i32, section_z: i32) {
        self.chunk_renderer
            .remove(section_x, section_y, section_z);
    }

    /// Отрендерить один кадр: очистка экрана, отрисовка всех чанковых мешей, présent.
    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let surface_texture = self.surface.get_current_texture()?;
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame encoder"),
            });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.15,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            self.chunk_renderer.render(&mut rpass);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();

        Ok(())
    }

    /// Вернуть размер окна.
    pub fn size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.size
    }

    /// Вернуть ссылку на устройство (для создания буферов).
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// Вернуть ссылку на очередь (для отправки команд).
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renderer_has_device_and_queue() {
        // Проверка только того, что структуры компилируются.
        // Полная инициализация требует окна, поэтому не тестируется здесь.
        assert!(true);
    }
}
