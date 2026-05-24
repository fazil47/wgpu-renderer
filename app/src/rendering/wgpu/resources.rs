use std::sync::Arc;
use winit::window::Window;

use crate::material::RGBA;

use super::WgpuExt;

pub enum RenderTarget {
    Surface {
        surface: wgpu::Surface<'static>,
        config: wgpu::SurfaceConfiguration,
    },
    Offscreen {
        texture: wgpu::Texture,
        view: wgpu::TextureView,
    },
}

impl RenderTarget {
    pub fn format(&self) -> wgpu::TextureFormat {
        match self {
            RenderTarget::Surface { config, .. } => config.format,
            RenderTarget::Offscreen { texture, .. } => texture.format(),
        }
    }

    pub fn width(&self) -> u32 {
        match self {
            RenderTarget::Surface { config, .. } => config.width,
            RenderTarget::Offscreen { texture, .. } => texture.width(),
        }
    }

    pub fn height(&self) -> u32 {
        match self {
            RenderTarget::Surface { config, .. } => config.height,
            RenderTarget::Offscreen { texture, .. } => texture.height(),
        }
    }
}

pub struct WgpuResources {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub target: RenderTarget,
}

impl ecs::Resource for WgpuResources {}

impl WgpuResources {
    pub async fn new(window: Arc<Window>, window_size: &winit::dpi::PhysicalSize<u32>) -> Self {
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window.clone()).unwrap();
        Self::build(
            instance,
            Some(surface),
            window_size.width,
            window_size.height,
        )
        .await
    }

    pub async fn new_headless() -> Self {
        let instance = wgpu::Instance::default();
        Self::build(instance, None, 800, 600).await
    }

    /// Shared device creation and initialization.
    async fn build(
        instance: wgpu::Instance,
        surface: Option<wgpu::Surface<'static>>,
        width: u32,
        height: u32,
    ) -> Self {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: surface.as_ref(),
            })
            .await
            .expect("Failed to find an appropriate adapter");

        // TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES is native only
        let required_features = wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
            | wgpu::Features::FLOAT32_FILTERABLE;

        // Create the logical device and command queue
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Device"),
                required_features,
                required_limits: wgpu::Limits {
                    max_storage_buffers_per_shader_stage: 10,
                    ..wgpu::Limits::default().using_resolution(adapter.limits())
                },
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::default(),
            })
            .await
            .expect("Failed to create device");

        let target = if let Some(surface) = surface {
            let config = surface
                .get_default_config(&adapter, width, height)
                .expect("Failed to get default surface configuration");
            surface.configure(&device, &config);
            RenderTarget::Surface { surface, config }
        } else {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Offscreen Render Target"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            RenderTarget::Offscreen { texture, view }
        };

        Self {
            instance,
            adapter,
            device,
            queue,
            target,
        }
    }

    pub fn resize(&mut self, new_size: &winit::dpi::PhysicalSize<u32>) {
        match &mut self.target {
            RenderTarget::Surface { surface, config } => {
                config.width = new_size.width.max(1);
                config.height = new_size.height.max(1);
                surface.configure(&self.device, config);
            }
            RenderTarget::Offscreen { texture, view } => {
                let format = texture.format();
                let usage = texture.usage();
                *texture = self.device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("Offscreen Render Target"),
                    size: wgpu::Extent3d {
                        width: new_size.width.max(1),
                        height: new_size.height.max(1),
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage,
                    view_formats: &[],
                });
                *view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            }
        }
    }
}

pub type Index = u32;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuRGBA {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

impl GpuRGBA {
    pub fn new(rgba: RGBA) -> Self {
        Self {
            r: rgba.r,
            g: rgba.g,
            b: rgba.b,
            a: rgba.a,
        }
    }

    pub fn to_array(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Resolution {
    width: f32,
    height: f32,
}

impl Resolution {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width: width as f32,
            height: height as f32,
        }
    }
}

pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl Texture {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn create_depth_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        label: &str,
    ) -> Self {
        let texture = device
            .texture()
            .label(label)
            .size_2d(width.max(1), height.max(1))
            .format(Self::DEPTH_FORMAT)
            .usage(wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING)
            .build();
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device
            .sampler()
            .label("Depth Sampler")
            .clamp()
            .filter(wgpu::FilterMode::Linear)
            .mipmap_filter(wgpu::FilterMode::Nearest)
            .compare(wgpu::CompareFunction::LessEqual)
            .build();
        Self {
            texture,
            view,
            sampler,
        }
    }
}
