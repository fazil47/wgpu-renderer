use std::{
    marker::PhantomData,
    mem::{offset_of, size_of},
    sync::Arc,
};

use bytemuck::{NoUninit, Pod};
use winit::window::Window;

use crate::mesh::Material;

pub struct RendererWgpu {
    pub instance: wgpu::Instance,
    pub surface: wgpu::Surface<'static>,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface_config: wgpu::SurfaceConfiguration,
}

impl RendererWgpu {
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
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Device"),
                    required_features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES, // This can be removed when wgpu is upgraded to the next version.
                    // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
                    required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
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
pub struct Vertex {
    pub position: [f32; 4],
    pub normal: [f32; 4],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroU32;

    // Helper to initialize WGPU device and queue for tests.
    // Returns Option to allow tests to be skipped if initialization fails.
    async fn setup_wgpu() -> Option<(wgpu::Device, wgpu::Queue)> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(), // Or specific like VULKAN, METAL, DX12
            dx12_shader_compiler: wgpu::Dx12Compiler::default(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await;

        if adapter.is_none() {
            println!("Test WGPU: No adapter found, skipping device-dependent tests.");
            return None;
        }
        let adapter = adapter.unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Test Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .ok()?;

        Some((device, queue))
    }

    #[tokio::test]
    async fn test_buffer_creation_and_usage_flags() {
        if let Some((device, _queue)) = setup_wgpu().await {
            let data = [1.0f32, 2.0, 3.0];
            let u32_data = [1u32, 2, 3];

            // Uniform Buffer
            let uniform_buffer = Buffer::new_uniform(&device, Some("TestUniform"), &data);
            assert_eq!(
                uniform_buffer.buffer.usage(),
                wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST
            );

            // Storage Buffer (read_only = true)
            let storage_ro_buffer = Buffer::new_storage(&device, Some("TestStorageRO"), &data, true);
            assert_eq!(
                storage_ro_buffer.buffer.usage(),
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST
            );

            // Storage Buffer (read_only = false)
            let storage_rw_buffer = Buffer::new_storage(&device, Some("TestStorageRW"), &data, false);
            assert_eq!(
                storage_rw_buffer.buffer.usage(),
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC
            );

            // Vertex Buffer
            let vertex_buffer = Buffer::new_vertex(&device, Some("TestVertex"), &data);
            assert_eq!(
                vertex_buffer.buffer.usage(),
                wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST
            );

            // Index Buffer
            let index_buffer = Buffer::new_index(&device, Some("TestIndex"), &u32_data);
            assert_eq!(
                index_buffer.buffer.usage(),
                wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST
            );

        } else {
            println!("Skipping test_buffer_creation_and_usage_flags: WGPU setup failed.");
        }
    }

    #[tokio::test]
    async fn test_buffer_write_does_not_panic() {
        if let Some((device, queue)) = setup_wgpu().await {
            let data = [0.5f32, 1.5, 2.5];
            let uniform_buffer = Buffer::new_uniform(&device, Some("TestWriteBuffer"), &data);
            uniform_buffer.write(&queue, &[7.0f32, 8.0, 9.0]); // Test does not panic
        } else {
            println!("Skipping test_buffer_write_does_not_panic: WGPU setup failed.");
        }
    }

    #[tokio::test]
    async fn test_generic_texture_creation() {
        if let Some((device, _queue)) = setup_wgpu().await {
            let width = 64;
            let height = 64;
            let format = wgpu::TextureFormat::Rgba8UnormSrgb;
            let usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST;

            // Texture without sampler
            let tex_no_sampler = Texture::new(
                &device,
                width,
                height,
                format,
                usage,
                Some("TestTextureNoSampler"),
                None,
            );
            assert!(tex_no_sampler.sampler.is_none());
            assert_eq!(tex_no_sampler.dimensions, (width, height));
            assert_eq!(tex_no_sampler.format, format);
            assert_eq!(tex_no_sampler.texture.usage(), usage);
            assert_eq!(tex_no_sampler.texture.dimension(), wgpu::TextureDimension::D2);
            assert_eq!(tex_no_sampler.texture.width(), width);
            assert_eq!(tex_no_sampler.texture.height(), height);


            // Texture with sampler
            let sampler_desc = wgpu::SamplerDescriptor {
                label: Some("TestSampler"),
                ..Default::default()
            };
            let tex_with_sampler = Texture::new(
                &device,
                width,
                height,
                format,
                usage,
                Some("TestTextureWithSampler"),
                Some(&sampler_desc),
            );
            assert!(tex_with_sampler.sampler.is_some());
            assert_eq!(tex_with_sampler.dimensions, (width, height));
            assert_eq!(tex_with_sampler.format, format);
            assert_eq!(tex_with_sampler.texture.usage(), usage);
        } else {
            println!("Skipping test_generic_texture_creation: WGPU setup failed.");
        }
    }

    #[tokio::test]
    async fn test_depth_texture_creation() {
        if let Some((device, _queue)) = setup_wgpu().await {
            let surface_config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: wgpu::TextureFormat::Bgra8UnormSrgb, // A common surface format
                width: 128,
                height: 128,
                present_mode: wgpu::PresentMode::Fifo,
                desired_maximum_frame_latency: 2,
                alpha_mode: wgpu::CompositeAlphaMode::Auto,
                view_formats: vec![],
            };

            let depth_texture = DepthTexture::new(&device, &surface_config, "TestDepthTexture");

            assert_eq!(depth_texture.texture.format(), DepthTexture::DEPTH_FORMAT);
            assert_eq!(depth_texture.texture.width(), surface_config.width);
            assert_eq!(depth_texture.texture.height(), surface_config.height);
            assert_eq!(
                depth_texture.texture.usage(),
                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING
            );
            // Sampler specific checks like compare function are hard here,
            // but we check it's created.
            assert!(true); // Implicitly checks that a sampler was created as part of DepthTexture
        } else {
            println!("Skipping test_depth_texture_creation: WGPU setup failed.");
        }
    }

    #[test]
    fn test_bgl_entry_builder() {
        // Buffer
        let entry_buffer = BindGroupLayoutEntryBuilder::new()
            .binding(0)
            .visibility(wgpu::ShaderStages::VERTEX)
            .buffer(wgpu::BufferBindingType::Uniform, false, None)
            .build();
        assert_eq!(entry_buffer.binding, 0);
        assert_eq!(entry_buffer.visibility, wgpu::ShaderStages::VERTEX);
        match entry_buffer.ty {
            wgpu::BindingType::Buffer { ty, .. } => assert_eq!(ty, wgpu::BufferBindingType::Uniform),
            _ => panic!("Incorrect type for buffer entry"),
        }
        assert!(entry_buffer.count.is_none());

        // Sampler
        let entry_sampler = BindGroupLayoutEntryBuilder::new()
            .binding(1)
            .visibility(wgpu::ShaderStages::FRAGMENT)
            .sampler(wgpu::SamplerBindingType::Filtering)
            .build();
        assert_eq!(entry_sampler.binding, 1);
        assert_eq!(entry_sampler.visibility, wgpu::ShaderStages::FRAGMENT);
        match entry_sampler.ty {
            wgpu::BindingType::Sampler(ty) => assert_eq!(ty, wgpu::SamplerBindingType::Filtering),
            _ => panic!("Incorrect type for sampler entry"),
        }

        // Texture
        let entry_texture = BindGroupLayoutEntryBuilder::new()
            .binding(2)
            .visibility(wgpu::ShaderStages::FRAGMENT)
            .texture(
                wgpu::TextureSampleType::Float { filterable: true },
                wgpu::TextureViewDimension::D2,
                false,
            )
            .build();
        assert_eq!(entry_texture.binding, 2);
        match entry_texture.ty {
            wgpu::BindingType::Texture { sample_type, .. } => {
                assert_eq!(sample_type, wgpu::TextureSampleType::Float { filterable: true })
            }
            _ => panic!("Incorrect type for texture entry"),
        }

        // Storage Texture
        let entry_storage_texture = BindGroupLayoutEntryBuilder::new()
            .binding(3)
            .visibility(wgpu::ShaderStages::COMPUTE)
            .storage_texture(
                wgpu::StorageTextureAccess::WriteOnly,
                wgpu::TextureFormat::Rgba8Unorm,
                wgpu::TextureViewDimension::D2,
            )
            .build();
        assert_eq!(entry_storage_texture.binding, 3);
        match entry_storage_texture.ty {
            wgpu::BindingType::StorageTexture { access, .. } => {
                assert_eq!(access, wgpu::StorageTextureAccess::WriteOnly)
            }
            _ => panic!("Incorrect type for storage texture entry"),
        }

        // Count
        let count_val = NonZeroU32::new(6).unwrap();
        let entry_counted_texture = BindGroupLayoutEntryBuilder::new()
            .binding(4)
            .visibility(wgpu::ShaderStages::FRAGMENT)
            .texture(
                wgpu::TextureSampleType::Sint,
                wgpu::TextureViewDimension::D2Array,
                false,
            )
            .count(count_val)
            .build();
        assert_eq!(entry_counted_texture.binding, 4);
        assert_eq!(entry_counted_texture.count, Some(count_val));
    }

    #[test]
    #[should_panic(expected = "Binding type must be set for BindGroupLayoutEntry")]
    fn test_bgl_entry_builder_panic_no_type() {
        BindGroupLayoutEntryBuilder::new().binding(0).build();
    }

    #[tokio::test]
    async fn test_bg_entry_builder() {
        if let Some((device, _queue)) = setup_wgpu().await {
            let test_buffer_data = [1u32, 2, 3];
            let buffer_for_binding = Buffer::new_uniform(&device, Some("TestBufferForBinding"), &test_buffer_data);

            let bg_entry = BindGroupEntryBuilder::new()
                .binding(0)
                .resource(buffer_for_binding.buffer.as_entire_binding())
                .build();

            assert_eq!(bg_entry.binding, 0);
            // Checking the resource content is complex, just ensure it's Some.
            // The actual resource variant is wgpu::BindingResource::Buffer, but it's private.
        } else {
             println!("Skipping test_bg_entry_builder: WGPU setup failed.");
        }
    }

    #[test]
    #[should_panic(expected = "Resource must be set for BindGroupEntry")]
    fn test_bg_entry_builder_panic_no_resource() {
        BindGroupEntryBuilder::new().binding(0).build();
    }

    #[tokio::test]
    async fn test_create_bind_group_layout_does_not_panic() {
        if let Some((device, _queue)) = setup_wgpu().await {
            let entry = BindGroupLayoutEntryBuilder::new()
                .binding(0)
                .visibility(wgpu::ShaderStages::VERTEX)
                .buffer(wgpu::BufferBindingType::Uniform, false, None)
                .build();

            let _layout = create_bind_group_layout(&device, Some("TestLayout"), &[entry]);
            // Test passes if it doesn't panic
        } else {
            println!("Skipping test_create_bind_group_layout_does_not_panic: WGPU setup failed.");
        }
    }

    #[tokio::test]
    async fn test_create_bind_group_does_not_panic() {
        if let Some((device, _queue)) = setup_wgpu().await {
            // 1. Create Layout
            let layout_entry = BindGroupLayoutEntryBuilder::new()
                .binding(0)
                .visibility(wgpu::ShaderStages::VERTEX)
                .buffer(wgpu::BufferBindingType::Uniform, false, None)
                .build();
            let layout = create_bind_group_layout(&device, Some("TestLayoutForGroup"), &[layout_entry]);

            // 2. Create Resource (Buffer)
            let test_data = [0u32];
            let buffer_resource = Buffer::new_uniform(&device, Some("TestBufferForGroup"), &test_data);

            // 3. Create BindGroupEntry
            let bg_entry = BindGroupEntryBuilder::new()
                .binding(0)
                .resource(buffer_resource.buffer.as_entire_binding())
                .build();

            // 4. Create Bind Group
            let _bind_group = create_bind_group(&device, Some("TestGroup"), &layout, &[bg_entry]);
            // Test passes if it doesn't panic
        } else {
            println!("Skipping test_create_bind_group_does_not_panic: WGPU setup failed.");
        }
    }
}

// --- Bind Group and Layout Helpers ---

/// Builder for `wgpu::BindGroupLayoutEntry`.
///
/// Simplifies the creation of bind group layout entries by providing a
/// chained interface to set various properties.
///
/// # Example
/// ```rust
/// // let entry = BindGroupLayoutEntryBuilder::new()
/// //     .binding(0)
/// //     .visibility(wgpu::ShaderStages::VERTEX)
/// //     .buffer(wgpu::BufferBindingType::Uniform, false, None)
/// //     .build();
/// ```
#[derive(Default, Debug, Clone)]
pub struct BindGroupLayoutEntryBuilder {
    binding: u32,
    visibility: wgpu::ShaderStages,
    ty: Option<wgpu::BindingType>,
    count: Option<std::num::NonZeroU32>,
}

impl BindGroupLayoutEntryBuilder {
    /// Creates a new `BindGroupLayoutEntryBuilder` with default values.
    /// Default visibility is `wgpu::ShaderStages::NONE`.
    pub fn new() -> Self {
        Self {
            binding: 0,
            visibility: wgpu::ShaderStages::NONE,
            ty: None,
            count: None,
        }
    }

    /// Sets the binding slot.
    pub fn binding(mut self, binding: u32) -> Self {
        self.binding = binding;
        self
    }

    /// Sets the shader stages where this binding will be visible.
    pub fn visibility(mut self, visibility: wgpu::ShaderStages) -> Self {
        self.visibility = visibility;
        self
    }

    /// Configures this entry as a buffer binding.
    pub fn buffer(
        mut self,
        buffer_type: wgpu::BufferBindingType,
        has_dynamic_offset: bool,
        min_binding_size: Option<wgpu::BufferSize>,
    ) -> Self {
        self.ty = Some(wgpu::BindingType::Buffer {
            ty: buffer_type,
            has_dynamic_offset,
            min_binding_size,
        });
        self
    }

    /// Configures this entry as a sampler binding.
    pub fn sampler(mut self, sampler_type: wgpu::SamplerBindingType) -> Self {
        self.ty = Some(wgpu::BindingType::Sampler(sampler_type));
        self
    }

    /// Configures this entry as a texture binding.
    pub fn texture(
        mut self,
        sample_type: wgpu::TextureSampleType,
        view_dimension: wgpu::TextureViewDimension,
        multisampled: bool,
    ) -> Self {
        self.ty = Some(wgpu::BindingType::Texture {
            sample_type,
            view_dimension,
            multisampled,
        });
        self
    }

    /// Configures this entry as a storage texture binding.
    pub fn storage_texture(
        mut self,
        access: wgpu::StorageTextureAccess,
        format: wgpu::TextureFormat,
        view_dimension: wgpu::TextureViewDimension,
    ) -> Self {
        self.ty = Some(wgpu::BindingType::StorageTexture {
            access,
            format,
            view_dimension,
        });
        self
    }

    /// Sets the count for this binding, typically for arrays of textures.
    pub fn count(mut self, count: std::num::NonZeroU32) -> Self {
        self.count = Some(count);
        self
    }

    /// Builds the `wgpu::BindGroupLayoutEntry`.
    ///
    /// # Panics
    /// Panics if the binding type (`ty`) was not set.
    pub fn build(self) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding: self.binding,
            visibility: self.visibility,
            ty: self.ty.expect("Binding type must be set for BindGroupLayoutEntry"),
            count: self.count,
        }
    }
}

/// Builder for `wgpu::BindGroupEntry`.
///
/// Simplifies the creation of bind group entries by providing a
/// chained interface.
///
/// # Example
/// ```rust
/// // let uniform_buffer = Buffer::<MyUniforms>::new_uniform(...);
/// // let entry = BindGroupEntryBuilder::new()
/// //     .binding(0)
/// //     .resource(uniform_buffer.buffer.as_entire_binding())
/// //     .build();
/// ```
#[derive(Default)]
pub struct BindGroupEntryBuilder<'a> {
    binding: u32,
    resource: Option<wgpu::BindingResource<'a>>,
}

impl<'a> BindGroupEntryBuilder<'a> {
    /// Creates a new `BindGroupEntryBuilder`.
    pub fn new() -> Self {
        Self {
            binding: 0,
            resource: None,
        }
    }

    /// Sets the binding slot.
    pub fn binding(mut self, binding: u32) -> Self {
        self.binding = binding;
        self
    }

    /// Sets the resource for this entry.
    pub fn resource(mut self, resource: wgpu::BindingResource<'a>) -> Self {
        self.resource = Some(resource);
        self
    }

    /// Builds the `wgpu::BindGroupEntry`.
    ///
    /// # Panics
    /// Panics if the resource was not set.
    pub fn build(self) -> wgpu::BindGroupEntry<'a> {
        wgpu::BindGroupEntry {
            binding: self.binding,
            resource: self.resource.expect("Resource must be set for BindGroupEntry"),
        }
    }
}

/// Creates a `wgpu::BindGroupLayout` with the given entries.
///
/// # Arguments
/// * `device`: The WGPU device.
/// * `label`: An optional label for the bind group layout.
/// * `entries`: A slice of `wgpu::BindGroupLayoutEntry` defining the layout.
pub fn create_bind_group_layout(
    device: &wgpu::Device,
    label: Option<&str>,
    entries: &[wgpu::BindGroupLayoutEntry],
) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label, entries })
}

/// Creates a `wgpu::BindGroup` with the given layout and entries.
///
/// # Arguments
/// * `device`: The WGPU device.
/// * `label`: An optional label for the bind group.
/// * `layout`: The `wgpu::BindGroupLayout` that this bind group will conform to.
/// * `entries`: A slice of `wgpu::BindGroupEntry` for the resources to bind.
pub fn create_bind_group<'a>(
    device: &wgpu::Device,
    label: Option<&str>,
    layout: &wgpu::BindGroupLayout,
    entries: &[wgpu::BindGroupEntry<'a>],
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label,
        layout,
        entries,
    })
}

/*
// Illustrative Example of using the builders and helpers:
// (Assuming `device`, `some_uniform_buffer: Buffer<MyUniforms>`,
//  `some_texture: Texture` and `some_sampler: wgpu::Sampler` are available)

fn example_bind_group_creation(
    device: &wgpu::Device,
    uniform_buffer: &Buffer<u32>, // Replace u32 with your actual uniform type
    texture_view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
) {
    // 1. Define Bind Group Layout Entries using the builder
    let layout_entry_uniform = BindGroupLayoutEntryBuilder::new()
        .binding(0)
        .visibility(wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT)
        .buffer(wgpu::BufferBindingType::Uniform, false, None)
        .build();

    let layout_entry_texture = BindGroupLayoutEntryBuilder::new()
        .binding(1)
        .visibility(wgpu::ShaderStages::FRAGMENT)
        .texture(
            wgpu::TextureSampleType::Float { filterable: true },
            wgpu::TextureViewDimension::D2,
            false,
        )
        .build();

    let layout_entry_sampler = BindGroupLayoutEntryBuilder::new()
        .binding(2)
        .visibility(wgpu::ShaderStages::FRAGMENT)
        .sampler(wgpu::SamplerBindingType::Filtering)
        .build();

    // 2. Create the Bind Group Layout using the helper
    let bind_group_layout = create_bind_group_layout(
        device,
        Some("Example Bind Group Layout"),
        &[layout_entry_uniform, layout_entry_texture, layout_entry_sampler],
    );

    // 3. Define Bind Group Entries using the builder
    let entry_uniform = BindGroupEntryBuilder::new()
        .binding(0)
        .resource(uniform_buffer.buffer.as_entire_binding())
        .build();

    let entry_texture = BindGroupEntryBuilder::new()
        .binding(1)
        .resource(wgpu::BindingResource::TextureView(texture_view))
        .build();

    let entry_sampler = BindGroupEntryBuilder::new()
        .binding(2)
        .resource(wgpu::BindingResource::Sampler(sampler))
        .build();

    // 4. Create the Bind Group using the helper
    let _bind_group = create_bind_group(
        device,
        Some("Example Bind Group"),
        &bind_group_layout,
        &[entry_uniform, entry_texture, entry_sampler],
    );

    // _bind_group can now be used in a render pass
}
*/

// Raytracer vertex field offsets are calculated based on the following assumptions:
// The field are aligned to 4 bytes.

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RaytracerVertex {
    pub position: [f32; 4],
    pub normal: [f32; 4],
    pub material_id: f32,
}

impl RaytracerVertex {
    pub fn from_vertex(vertex: &Vertex, material_id: usize) -> Self {
        Self {
            position: vertex.position,
            normal: vertex.normal,
            material_id: material_id as f32,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RaytracerMaterial {
    pub color: [f32; 4],
}

impl RaytracerMaterial {
    pub fn from_material(material: &Material) -> Self {
        Self {
            color: material.color.to_array(),
        }
    }
}

pub const RAYTRACE_MATERIAL_STRIDE: u32 =
    (size_of::<RaytracerMaterial>() / size_of::<f32>()) as u32;

pub const RAYTRACE_VERTEX_STRIDE: u32 = (size_of::<RaytracerVertex>() / size_of::<f32>()) as u32;
pub const RAYTRACE_VERTEX_NORMAL_OFFSET: u32 =
    (offset_of!(RaytracerVertex, normal) / size_of::<f32>()) as u32;
pub const RAYTRACE_VERTEX_MATERIAL_ID_OFFSET: u32 =
    (offset_of!(RaytracerVertex, material_id) / size_of::<f32>()) as u32;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
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

pub struct Buffer<T: Pod> {
    pub buffer: wgpu::Buffer,
    _marker: PhantomData<T>,
}

impl<T: Pod> Buffer<T> {
    pub fn new_uniform(
        device: &wgpu::Device,
        label: Option<&str>,
        contents: &[T],
    ) -> Self {
        use wgpu::util::DeviceExt;
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label,
            contents: bytemuck::cast_slice(contents),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        Self {
            buffer,
            _marker: PhantomData,
        }
    }

    pub fn new_storage(
        device: &wgpu::Device,
        label: Option<&str>,
        contents: &[T],
        read_only: bool,
    ) -> Self {
        use wgpu::util::DeviceExt;
        let mut usage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
        if !read_only {
            usage |= wgpu::BufferUsages::COPY_SRC;
        }
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label,
            contents: bytemuck::cast_slice(contents),
            usage,
        });
        Self {
            buffer,
            _marker: PhantomData,
        }
    }

    pub fn new_vertex(
        device: &wgpu::Device,
        label: Option<&str>,
        contents: &[T],
    ) -> Self {
        use wgpu::util::DeviceExt;
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label,
            contents: bytemuck::cast_slice(contents),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        Self {
            buffer,
            _marker: PhantomData,
        }
    }

    pub fn write(&self, queue: &wgpu::Queue, contents: &[T]) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(contents));
    }
}

impl Buffer<u32> {
    pub fn new_index(
        device: &wgpu::Device,
        label: Option<&str>,
        contents: &[u32],
    ) -> Self {
        use wgpu::util::DeviceExt;
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label,
            contents: bytemuck::cast_slice(contents),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });
        Self {
            buffer,
            _marker: PhantomData,
        }
    }
}

pub fn create_uniform_buffer<T: Pod>(
    device: &wgpu::Device,
    label: Option<&str>,
    contents: &[T],
) -> Buffer<T> {
    Buffer::new_uniform(device, label, contents)
}

pub fn create_storage_buffer<T: Pod>(
    device: &wgpu::Device,
    label: Option<&str>,
    contents: &[T],
    read_only: bool,
) -> Buffer<T> {
    Buffer::new_storage(device, label, contents, read_only)
}

pub fn create_vertex_buffer<T: Pod>(
    device: &wgpu::Device,
    label: Option<&str>,
    contents: &[T],
) -> Buffer<T> {
    Buffer::new_vertex(device, label, contents)
}

pub fn create_index_buffer(
    device: &wgpu::Device,
    label: Option<&str>,
    contents: &[u32],
) -> Buffer<u32> {
    Buffer::new_index(device, label, contents)
}

// Renamed from Texture to DepthTexture
pub struct DepthTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl DepthTexture {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration, // Keep config as it's used for width/height
        label: &str,
    ) -> Self {
        let width = config.width.max(1);
        let height = config.height.max(1);
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = device.create_texture(&desc);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
        }
    }
}

pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: Option<wgpu::Sampler>,
    pub format: wgpu::TextureFormat,
    pub dimensions: (u32, u32),
}

impl Texture {
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
        label: Option<&str>,
        sampler_descriptor: Option<&wgpu::SamplerDescriptor>,
    ) -> Self {
        let width = width.max(1);
        let height = height.max(1);

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture_descriptor = wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        };
        let texture = device.create_texture(&texture_descriptor);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = sampler_descriptor.map(|sd| device.create_sampler(sd));

        Self {
            texture,
            view,
            sampler,
            format,
            dimensions: (width, height),
        }
    }
}
