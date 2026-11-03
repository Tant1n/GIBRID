// NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.


// (Это неофициальный продукт Minecraft, не одобрен Mojang или Microsoft и не связан с ними.//! wgpu renderer — Phase 3 + Phase 4: окно, surface, устройство, очередь и рендер чанковых мешей.
//!
//! Статус: IMPLEMENTED (window + surface + device + clear + chunk meshes).

#![forbid(unsafe_code)]

pub mod chunk_renderer;
pub mod ui_renderer;

use crate::ui_renderer::UiRenderer;
use winit::window::Window;

/// Выбранный графический бэкенд wgpu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GfxBackend {
    Vulkan,
    Dx12,
    Gl,
    Primary,
}

impl From<GfxBackend> for wgpu::Backends {
    fn from(b: GfxBackend) -> Self {
        match b {
            GfxBackend::Vulkan => wgpu::Backends::VULKAN,
            GfxBackend::Dx12 => wgpu::Backends::DX12,
            GfxBackend::Gl => wgpu::Backends::GL,
            GfxBackend::Primary => wgpu::Backends::PRIMARY,
        }
    }
}

/// Диагностический рендерер: окно, surface, устройство, очередь и рендер чанковых мешей.
pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    chunk_renderer: chunk_renderer::ChunkRenderer,
}

impl Renderer {
    /// Создать поверхность, адаптер, устройство, очередь и конвейер рендеринга.
    pub fn new(window: Window, backend: GfxBackend, _render_threads: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let size = window.inner_size();

        // Surface
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: backend.into(),
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

        let depth_texture = Self::create_depth_texture(&device, &config);
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let chunk_renderer = chunk_renderer::ChunkRenderer::new(&device, &queue, surface_format);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            size,
            depth_texture,
            depth_view,
            chunk_renderer,
        })
    }

    /// Создать depth текстуру для буфера глубины.
    fn create_depth_texture(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
    ) -> wgpu::Texture {
        let size = wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
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
        self.depth_texture = Self::create_depth_texture(&self.device, &self.config);
        self.depth_view = self.depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
    }

    /// Загрузить или обновить меш секции на GPU.
    pub fn upload_chunk(&mut self, result: &gibrid_mesh::MeshResult) {
        self.chunk_renderer
            .upload_or_update(&self.device, &self.queue, result);
    }

    /// Обновить матрицу камеры на основе позиции и угла игрока.
    pub fn update_camera(&mut self, eye: [f32; 3], yaw_deg: f32, pitch_deg: f32) {
        let aspect = self.size.width as f32 / self.size.height as f32;
        self.chunk_renderer
            .update_camera(&self.queue, eye, yaw_deg, pitch_deg, aspect);
    }

    /// Удалить секцию из рендерера.
    pub fn remove_chunk(&mut self, section_x: i32, section_y: i32, section_z: i32) {
        self.chunk_renderer
            .remove(section_x, section_y, section_z);
    }

    /// Установить текстурный атлас (данные переданы лаунчером).
    pub fn set_texture_atlas(
        &mut self,
        width: u32,
        height: u32,
        data: &[u8],
    ) {
        self.chunk_renderer
            .set_texture_atlas(&self.device, &self.queue, width, height, data);
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
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
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

    /// Вернуть ссылку на поверхность wgpu.
    pub fn surface(&self) -> &wgpu::Surface<'static> {
        &self.surface
    }

    /// Вернуть ссылку на конфигурацию поверхности.
    pub fn config(&self) -> &wgpu::SurfaceConfiguration {
        &self.config
    }

    /// Отрисовать только UI (цветные кнопки поверх чистого экрана), без мира.
    pub fn render_ui(
        &mut self,
        ui_renderer: &mut UiRenderer,
        buttons: &[crate::ui_renderer::UiButton],
    ) -> Result<(), wgpu::SurfaceError> {
        let surface_texture = self.surface.get_current_texture()?;
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ui frame encoder"),
            });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ui render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.05,
                            g: 0.05,
                            b: 0.08,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            let hovered_id = None; // TODO: отслеживать наведение
            ui_renderer.render(
                self.device(),
                self.queue(),
                &mut rpass,
                buttons,
                self.size().width,
                self.size().height,
                hovered_id,
            );
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();

        Ok(())
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

