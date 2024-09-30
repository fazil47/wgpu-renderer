use bytemuck::NoUninit;
use winit::window::Window;

pub type ColsArray = [f32; 16];

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RGBA {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

impl RGBA {
    pub fn new(rgba: [f32; 4]) -> Self {
        Self {
            r: rgba[0],
            g: rgba[1],
            b: rgba[2],
            a: rgba[3],
        }
    }
}

unsafe impl NoUninit for RGBA {}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
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

unsafe impl NoUninit for Resolution {}

pub fn update_buffer<T: NoUninit>(queue: &wgpu::Queue, wgpu_buffer: &wgpu::Buffer, value: T) {
    queue.write_buffer(&wgpu_buffer, 0, bytemuck::cast_slice(&[value]));
}

pub async fn initialize_wgpu<'window>(
    window: &'window Window,
    window_size: &winit::dpi::PhysicalSize<u32>,
) -> (
    wgpu::Instance,
    wgpu::Surface<'window>,
    wgpu::Adapter,
    wgpu::Device,
    wgpu::Queue,
    wgpu::SurfaceConfiguration,
) {
    let instance = wgpu::Instance::default();
    let surface = instance.create_surface(window).unwrap();
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
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Device"),
                required_features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES, // This can be removed when wgpu is upgraded to the next version.
                // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
                required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
            },
            None,
        )
        .await
        .expect("Failed to create device");

    let surface_config = surface
        .get_default_config(&adapter, window_size.width, window_size.height)
        .expect("Failed to get default surface configuration");
    surface.configure(&device, &surface_config);

    (instance, surface, adapter, device, queue, surface_config)
}
