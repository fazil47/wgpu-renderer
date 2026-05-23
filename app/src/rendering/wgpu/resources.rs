use std::sync::Arc;
use winit::window::Window;

use crate::material::RGBA;

use super::WgpuExt;

pub struct WgpuResources {
    pub instance: wgpu::Instance,
    pub surface: Option<wgpu::Surface<'static>>,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface_config: wgpu::SurfaceConfiguration,
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

        let surface_config = if let Some(ref surface) = surface {
            surface
                .get_default_config(&adapter, width, height)
                .expect("Failed to get default surface configuration")
        } else {
            // TODO: Find a cleaner way to support headless
            wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                width,
                height,
                present_mode: wgpu::PresentMode::Fifo,
                desired_maximum_frame_latency: 2,
                alpha_mode: wgpu::CompositeAlphaMode::Opaque,
                view_formats: vec![],
            }
        };

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

        if let Some(ref surface) = surface {
            surface.configure(&device, &surface_config);
        }

        Self {
            instance,
            surface,
            adapter,
            device,
            queue,
            surface_config,
        }
    }

    pub fn resize(&mut self, new_size: &winit::dpi::PhysicalSize<u32>) {
        self.surface_config.width = new_size.width.max(1);
        self.surface_config.height = new_size.height.max(1);
        if let Some(surface) = &self.surface {
            surface.configure(&self.device, &self.surface_config);
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
        config: &wgpu::SurfaceConfiguration,
        label: &str,
    ) -> Self {
        let texture = device
            .texture()
            .label(label)
            .size_2d(config.width.max(1), config.height.max(1))
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
