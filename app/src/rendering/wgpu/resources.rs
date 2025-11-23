use std::sync::Arc;
use winit::window::Window;

use crate::material::RGBA;

use super::WgpuExt;

pub struct WgpuResources {
    pub instance: wgpu::Instance,
    pub surface: wgpu::Surface<'static>,
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
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                // Request an adapter which can render to our surface
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find an appropriate adapter");

        // Create the logical device and command queue
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Device"),
                required_features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
                    | wgpu::Features::FLOAT32_FILTERABLE,
                // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
                required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::default(),
            })
            .await
            .expect("Failed to create device");

        let surface_config = surface
            .get_default_config(&adapter, window_size.width, window_size.height)
            .expect("Failed to get default surface configuration");
        surface.configure(&device, &surface_config);

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
        self.surface.configure(&self.device, &self.surface_config);
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
