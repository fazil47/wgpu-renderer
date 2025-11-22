use std::borrow::Cow;
use wgpu::{Texture, util::DeviceExt};

// Re-export commonly used types
pub use wgpu::{
    AddressMode, BindGroup, BindGroupLayout, Buffer, CommandEncoder, CompareFunction, ComputePass,
    ComputePipeline, Device, Face, FilterMode, Queue, RenderPass, RenderPipeline, Sampler,
    ShaderModule, ShaderStages, TextureFormat, TextureView,
};

/// Core trait that extends wgpu::Device with fluent builders
pub trait WgpuExt {
    fn texture(&self) -> TextureBuilder<'_>;
    fn sampler(&self) -> SamplerBuilder<'_>;
    fn shader(&self) -> ShaderBuilder<'_>;
    fn buffer(&self) -> BufferBuilder<'_>;
    fn bind_group_layout(&self) -> BindGroupLayoutBuilder<'_>;
    fn bind_group<'a>(&'a self, layout: &'a BindGroupLayout) -> BindGroupBuilder<'a>;
    fn pipeline_layout(&self) -> PipelineLayoutBuilder<'_>;
    fn render_pipeline(&self) -> RenderPipelineBuilder<'_>;
    fn compute_pipeline(&self) -> ComputePipelineBuilder<'_>;
}

impl WgpuExt for Device {
    fn texture(&self) -> TextureBuilder<'_> {
        TextureBuilder::new(self)
    }
    fn sampler(&self) -> SamplerBuilder<'_> {
        SamplerBuilder::new(self)
    }
    fn shader(&self) -> ShaderBuilder<'_> {
        ShaderBuilder::new(self)
    }
    fn buffer(&self) -> BufferBuilder<'_> {
        BufferBuilder::new(self)
    }
    fn bind_group_layout(&self) -> BindGroupLayoutBuilder<'_> {
        BindGroupLayoutBuilder::new(self)
    }
    fn bind_group<'a>(&'a self, layout: &'a BindGroupLayout) -> BindGroupBuilder<'a> {
        BindGroupBuilder::new(self, layout)
    }
    fn pipeline_layout(&self) -> PipelineLayoutBuilder<'_> {
        PipelineLayoutBuilder::new(self)
    }
    fn render_pipeline(&self) -> RenderPipelineBuilder<'_> {
        RenderPipelineBuilder::new(self)
    }
    fn compute_pipeline(&self) -> ComputePipelineBuilder<'_> {
        ComputePipelineBuilder::new(self)
    }
}

/// Create a render pass builder - standalone function to avoid lifetime issues
pub fn render_pass<'a>(encoder: &'a mut CommandEncoder) -> RenderPassBuilder<'a> {
    RenderPassBuilder::new(encoder)
}

/// Create a compute pass builder - standalone function to avoid lifetime issues
pub fn compute_pass<'a>(encoder: &'a mut CommandEncoder) -> ComputePassBuilder<'a> {
    ComputePassBuilder::new(encoder)
}

/// Texture builder with smart defaults
pub struct TextureBuilder<'a> {
    device: &'a Device,
    label: Option<&'a str>,
    size: wgpu::Extent3d,
    format: TextureFormat,
    usage: wgpu::TextureUsages,
    dimension: wgpu::TextureDimension,
    mip_level_count: u32,
    sample_count: u32,
}

impl<'a> TextureBuilder<'a> {
    fn new(device: &'a Device) -> Self {
        Self {
            device,
            label: None,
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            format: TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            dimension: wgpu::TextureDimension::D2,
            mip_level_count: 1,
            sample_count: 1,
        }
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    pub fn size_2d(mut self, width: u32, height: u32) -> Self {
        self.size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        self.dimension = wgpu::TextureDimension::D2;
        self
    }

    pub fn size_3d(mut self, size: wgpu::Extent3d) -> Self {
        self.size = size;
        self.dimension = wgpu::TextureDimension::D3;
        self
    }

    pub fn format(mut self, format: TextureFormat) -> Self {
        self.format = format;
        self
    }
    pub fn usage(mut self, usage: wgpu::TextureUsages) -> Self {
        self.usage = usage;
        self
    }
    pub fn mip_levels(mut self, count: u32) -> Self {
        self.mip_level_count = count;
        self
    }
    pub fn multisampled(mut self, count: u32) -> Self {
        self.sample_count = count;
        self
    }

    // Preset configurations
    pub fn render_target(mut self) -> Self {
        self.usage = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
        self
    }
    pub fn depth_buffer(mut self) -> Self {
        self.format = TextureFormat::Depth32Float;
        self.usage = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
        self
    }
    pub fn storage_texture(mut self) -> Self {
        self.usage = wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING;
        self
    }
    pub fn hdr_target(mut self) -> Self {
        self.format = TextureFormat::Rgba16Float;
        self.usage = wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::STORAGE_BINDING;
        self
    }

    pub fn build(self) -> Texture {
        self.device.create_texture(&wgpu::TextureDescriptor {
            label: self.label,
            size: self.size,
            mip_level_count: self.mip_level_count,
            sample_count: self.sample_count,
            dimension: self.dimension,
            format: self.format,
            usage: self.usage,
            view_formats: &[],
        })
    }
}

/// Sampler builder with smart defaults
pub struct SamplerBuilder<'a> {
    device: &'a Device,
    label: Option<&'a str>,
    address_mode_u: AddressMode,
    address_mode_v: AddressMode,
    address_mode_w: AddressMode,
    mag_filter: FilterMode,
    min_filter: FilterMode,
    mipmap_filter: FilterMode,
    compare: Option<CompareFunction>,
    anisotropy_clamp: u16,
}

impl<'a> SamplerBuilder<'a> {
    fn new(device: &'a Device) -> Self {
        Self {
            device,
            label: None,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            compare: None,
            anisotropy_clamp: 1,
        }
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    pub fn address_mode(mut self, mode: AddressMode) -> Self {
        self.address_mode_u = mode;
        self.address_mode_v = mode;
        self.address_mode_w = mode;
        self
    }

    pub fn filter(mut self, filter: FilterMode) -> Self {
        self.mag_filter = filter;
        self.min_filter = filter;
        self
    }

    pub fn mipmap_filter(mut self, filter: FilterMode) -> Self {
        self.mipmap_filter = filter;
        self
    }

    pub fn compare(mut self, compare: CompareFunction) -> Self {
        self.compare = Some(compare);
        self
    }

    pub fn anisotropy(mut self, clamp: u16) -> Self {
        self.anisotropy_clamp = clamp;
        self
    }

    // Presets
    pub fn linear(mut self) -> Self {
        self.mag_filter = FilterMode::Linear;
        self.min_filter = FilterMode::Linear;
        self.mipmap_filter = FilterMode::Linear;
        self
    }

    pub fn nearest(mut self) -> Self {
        self.mag_filter = FilterMode::Nearest;
        self.min_filter = FilterMode::Nearest;
        self.mipmap_filter = FilterMode::Nearest;
        self
    }

    pub fn repeat(self) -> Self {
        self.address_mode(AddressMode::Repeat)
    }

    pub fn clamp(self) -> Self {
        self.address_mode(AddressMode::ClampToEdge)
    }

    pub fn shadow(self) -> Self {
        self.compare(CompareFunction::LessEqual)
    }

    pub fn build(self) -> Sampler {
        self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: self.label,
            address_mode_u: self.address_mode_u,
            address_mode_v: self.address_mode_v,
            address_mode_w: self.address_mode_w,
            mag_filter: self.mag_filter,
            min_filter: self.min_filter,
            mipmap_filter: self.mipmap_filter,
            compare: self.compare,
            anisotropy_clamp: self.anisotropy_clamp,
            ..Default::default()
        })
    }
}

/// Shader builder
pub struct ShaderBuilder<'a> {
    device: &'a Device,
    label: Option<&'a str>,
}

impl<'a> ShaderBuilder<'a> {
    fn new(device: &'a Device) -> Self {
        Self {
            device,
            label: None,
        }
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    pub fn wgsl(self, source: &str) -> ShaderModule {
        self.device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: self.label,
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(source)),
            })
    }

    pub fn wesl(self, source: Cow<'a, str>) -> ShaderModule {
        self.device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: self.label,
                source: wgpu::ShaderSource::Wgsl(source),
            })
    }
}

/// Buffer builder with convenience methods for common patterns
pub struct BufferBuilder<'a> {
    device: &'a Device,
    label: Option<&'a str>,
    usage: wgpu::BufferUsages,
    mapped_at_creation: bool,
}

impl<'a> BufferBuilder<'a> {
    fn new(device: &'a Device) -> Self {
        Self {
            device,
            label: None,
            usage: wgpu::BufferUsages::empty(),
            mapped_at_creation: false,
        }
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }
    pub fn usage(mut self, usage: wgpu::BufferUsages) -> Self {
        self.usage = usage;
        self
    }
    pub fn mapped(mut self) -> Self {
        self.mapped_at_creation = true;
        self
    }

    pub fn uniform<T: bytemuck::Pod>(self, data: &T) -> Buffer {
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: self.label,
                contents: bytemuck::cast_slice(std::slice::from_ref(data)),
                usage: self.usage | wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            })
    }

    pub fn storage<T: bytemuck::Pod>(self, data: &[T]) -> Buffer {
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: self.label,
                contents: bytemuck::cast_slice(data),
                usage: self.usage | wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            })
    }

    pub fn vertex<T: bytemuck::Pod>(self, data: &[T]) -> Buffer {
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: self.label,
                contents: bytemuck::cast_slice(data),
                usage: self.usage | wgpu::BufferUsages::VERTEX,
            })
    }

    pub fn index(self, data: &[u32]) -> Buffer {
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: self.label,
                contents: bytemuck::cast_slice(data),
                usage: self.usage | wgpu::BufferUsages::INDEX,
            })
    }

    pub fn empty(self, size: u64) -> Buffer {
        self.device.create_buffer(&wgpu::BufferDescriptor {
            label: self.label,
            size,
            usage: self.usage,
            mapped_at_creation: self.mapped_at_creation,
        })
    }
}

/// Bind group layout builder
pub struct BindGroupLayoutBuilder<'a> {
    device: &'a Device,
    label: Option<&'a str>,
    entries: Vec<wgpu::BindGroupLayoutEntry>,
}

impl<'a> BindGroupLayoutBuilder<'a> {
    fn new(device: &'a Device) -> Self {
        Self {
            device,
            label: None,
            entries: Vec::new(),
        }
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    pub fn uniform(mut self, binding: u32, visibility: ShaderStages) -> Self {
        self.entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });
        self
    }

    pub fn storage(mut self, binding: u32, visibility: ShaderStages, read_only: bool) -> Self {
        self.entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });
        self
    }

    pub fn texture_2d(mut self, binding: u32, visibility: ShaderStages) -> Self {
        self.entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        });
        self
    }

    pub fn texture_3d(mut self, binding: u32, visibility: ShaderStages) -> Self {
        self.entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D3,
                multisampled: false,
            },
            count: None,
        });
        self
    }

    pub fn storage_texture_2d(
        mut self,
        binding: u32,
        visibility: ShaderStages,
        access: wgpu::StorageTextureAccess,
        format: TextureFormat,
    ) -> Self {
        self.entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::StorageTexture {
                access,
                format,
                view_dimension: wgpu::TextureViewDimension::D2,
            },
            count: None,
        });
        self
    }

    pub fn storage_texture_3d(
        mut self,
        binding: u32,
        visibility: ShaderStages,
        access: wgpu::StorageTextureAccess,
        format: TextureFormat,
    ) -> Self {
        self.entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::StorageTexture {
                access,
                format,
                view_dimension: wgpu::TextureViewDimension::D3,
            },
            count: None,
        });
        self
    }

    pub fn sampler(mut self, binding: u32, visibility: ShaderStages) -> Self {
        self.entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        });
        self
    }

    pub fn comparison_sampler(mut self, binding: u32, visibility: ShaderStages) -> Self {
        self.entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
            count: None,
        });
        self
    }

    pub fn build(self) -> BindGroupLayout {
        self.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: self.label,
                entries: &self.entries,
            })
    }
}

/// Bind group builder
pub struct BindGroupBuilder<'a> {
    device: &'a Device,
    layout: &'a BindGroupLayout,
    label: Option<&'a str>,
    entries: Vec<wgpu::BindGroupEntry<'a>>,
}

impl<'a> BindGroupBuilder<'a> {
    fn new(device: &'a Device, layout: &'a BindGroupLayout) -> Self {
        Self {
            device,
            layout,
            label: None,
            entries: Vec::new(),
        }
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    pub fn buffer(mut self, binding: u32, buffer: &'a Buffer) -> Self {
        self.entries.push(wgpu::BindGroupEntry {
            binding,
            resource: buffer.as_entire_binding(),
        });
        self
    }

    pub fn buffer_range(
        mut self,
        binding: u32,
        buffer: &'a Buffer,
        offset: u64,
        size: Option<u64>,
    ) -> Self {
        self.entries.push(wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer,
                offset,
                size: size.map(|s| std::num::NonZeroU64::new(s).unwrap()),
            }),
        });
        self
    }

    pub fn texture(mut self, binding: u32, view: &'a TextureView) -> Self {
        self.entries.push(wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(view),
        });
        self
    }

    pub fn sampler(mut self, binding: u32, sampler: &'a Sampler) -> Self {
        self.entries.push(wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::Sampler(sampler),
        });
        self
    }

    pub fn build(self) -> BindGroup {
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: self.label,
            layout: self.layout,
            entries: &self.entries,
        })
    }
}

/// Pipeline layout builder
pub struct PipelineLayoutBuilder<'a> {
    device: &'a Device,
    label: Option<&'a str>,
    bind_group_layouts: Vec<&'a BindGroupLayout>,
    push_constant_ranges: Vec<wgpu::PushConstantRange>,
}

impl<'a> PipelineLayoutBuilder<'a> {
    fn new(device: &'a Device) -> Self {
        Self {
            device,
            label: None,
            bind_group_layouts: Vec::new(),
            push_constant_ranges: Vec::new(),
        }
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    pub fn bind_group_layout(mut self, layout: &'a BindGroupLayout) -> Self {
        self.bind_group_layouts.push(layout);
        self
    }

    pub fn bind_group_layouts(mut self, layouts: &[&'a BindGroupLayout]) -> Self {
        self.bind_group_layouts.extend_from_slice(layouts);
        self
    }

    pub fn push_constants(mut self, stages: ShaderStages, range: std::ops::Range<u32>) -> Self {
        self.push_constant_ranges
            .push(wgpu::PushConstantRange { stages, range });
        self
    }

    pub fn build(self) -> wgpu::PipelineLayout {
        self.device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: self.label,
                bind_group_layouts: &self.bind_group_layouts,
                push_constant_ranges: &self.push_constant_ranges,
            })
    }
}

/// Render pipeline builder with smart defaults
pub struct RenderPipelineBuilder<'a> {
    device: &'a Device,
    label: Option<&'a str>,
    layout: Option<&'a wgpu::PipelineLayout>,
    vertex_shader: Option<(&'a ShaderModule, &'a str)>,
    fragment_shader: Option<(&'a ShaderModule, &'a str)>,
    vertex_buffers: Vec<wgpu::VertexBufferLayout<'a>>,
    color_targets: Vec<Option<wgpu::ColorTargetState>>,
    depth_stencil: Option<wgpu::DepthStencilState>,
    primitive: wgpu::PrimitiveState,
    multisample: wgpu::MultisampleState,
}

impl<'a> RenderPipelineBuilder<'a> {
    fn new(device: &'a Device) -> Self {
        Self {
            device,
            label: None,
            layout: None,
            vertex_shader: None,
            fragment_shader: None,
            vertex_buffers: Vec::new(),
            color_targets: Vec::new(),
            depth_stencil: None,
            primitive: wgpu::PrimitiveState::default(),
            multisample: wgpu::MultisampleState::default(),
        }
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }
    pub fn layout(mut self, layout: &'a wgpu::PipelineLayout) -> Self {
        self.layout = Some(layout);
        self
    }

    pub fn vertex_shader(mut self, shader: &'a ShaderModule, entry_point: &'a str) -> Self {
        self.vertex_shader = Some((shader, entry_point));
        self
    }

    pub fn fragment_shader(mut self, shader: &'a ShaderModule, entry_point: &'a str) -> Self {
        self.fragment_shader = Some((shader, entry_point));
        self
    }

    pub fn vertex_buffer(mut self, layout: wgpu::VertexBufferLayout<'a>) -> Self {
        self.vertex_buffers.push(layout);
        self
    }

    pub fn color_target(mut self, target: wgpu::ColorTargetState) -> Self {
        self.color_targets.push(Some(target));
        self
    }

    pub fn color_target_replace(mut self, format: TextureFormat) -> Self {
        self.color_targets.push(Some(wgpu::ColorTargetState {
            format,
            blend: Some(wgpu::BlendState::REPLACE),
            write_mask: wgpu::ColorWrites::ALL,
        }));
        self
    }

    pub fn color_target_alpha_blend(mut self, format: TextureFormat) -> Self {
        self.color_targets.push(Some(wgpu::ColorTargetState {
            format,
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
        }));
        self
    }

    pub fn depth_test(mut self, format: TextureFormat, compare: CompareFunction) -> Self {
        self.depth_stencil = Some(wgpu::DepthStencilState {
            format,
            depth_write_enabled: true,
            depth_compare: compare,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        });
        self
    }

    pub fn depth_test_less(self, format: TextureFormat) -> Self {
        self.depth_test(format, CompareFunction::Less)
    }

    pub fn cull_mode(mut self, cull_mode: Option<Face>) -> Self {
        self.primitive.cull_mode = cull_mode;
        self
    }

    pub fn topology(mut self, topology: wgpu::PrimitiveTopology) -> Self {
        self.primitive.topology = topology;
        self
    }

    pub fn strip_index_format(mut self, format: Option<wgpu::IndexFormat>) -> Self {
        self.primitive.strip_index_format = format;
        self
    }

    pub fn build(self) -> Result<RenderPipeline, &'static str> {
        let layout = self.layout.ok_or("Pipeline layout is required")?;
        let (vs_module, vs_entry) = self.vertex_shader.ok_or("Vertex shader is required")?;

        let fragment = if let Some((fs_module, fs_entry)) = self.fragment_shader {
            Some(wgpu::FragmentState {
                module: fs_module,
                entry_point: Some(fs_entry),
                targets: &self.color_targets,
                compilation_options: Default::default(),
            })
        } else {
            None
        };

        Ok(self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: self.label,
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module: vs_module,
                    entry_point: Some(vs_entry),
                    buffers: &self.vertex_buffers,
                    compilation_options: Default::default(),
                },
                fragment,
                primitive: self.primitive,
                depth_stencil: self.depth_stencil,
                multisample: self.multisample,
                multiview: None,
                cache: None,
            }))
    }
}

/// Compute pipeline builder
pub struct ComputePipelineBuilder<'a> {
    device: &'a Device,
    label: Option<&'a str>,
    layout: Option<&'a wgpu::PipelineLayout>,
    shader: Option<(&'a ShaderModule, &'a str)>,
}

impl<'a> ComputePipelineBuilder<'a> {
    fn new(device: &'a Device) -> Self {
        Self {
            device,
            label: None,
            layout: None,
            shader: None,
        }
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }
    pub fn layout(mut self, layout: &'a wgpu::PipelineLayout) -> Self {
        self.layout = Some(layout);
        self
    }

    pub fn shader(mut self, shader: &'a ShaderModule, entry_point: &'a str) -> Self {
        self.shader = Some((shader, entry_point));
        self
    }

    pub fn build(self) -> Result<ComputePipeline, &'static str> {
        let layout = self.layout.ok_or("Pipeline layout is required")?;
        let (shader_module, entry_point) = self.shader.ok_or("Compute shader is required")?;

        Ok(self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: self.label,
                layout: Some(layout),
                module: shader_module,
                entry_point: Some(entry_point),
                compilation_options: Default::default(),
                cache: None,
            }))
    }
}

/// Render pass builder for simplified render pass creation
pub struct RenderPassBuilder<'a> {
    encoder: &'a mut CommandEncoder,
    label: Option<&'a str>,
    color_attachments: Vec<Option<wgpu::RenderPassColorAttachment<'a>>>,
    depth_stencil_attachment: Option<wgpu::RenderPassDepthStencilAttachment<'a>>,
}

impl<'a> RenderPassBuilder<'a> {
    fn new(encoder: &'a mut CommandEncoder) -> Self {
        Self {
            encoder,
            label: None,
            color_attachments: Vec::new(),
            depth_stencil_attachment: None,
        }
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    pub fn color_attachment(
        mut self,
        view: &'a TextureView,
        clear_color: Option<wgpu::Color>,
    ) -> Self {
        self.color_attachments
            .push(Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: if let Some(color) = clear_color {
                        wgpu::LoadOp::Clear(color)
                    } else {
                        wgpu::LoadOp::Load
                    },
                    store: wgpu::StoreOp::Store,
                },
            }));
        self
    }

    pub fn depth_attachment(mut self, view: &'a TextureView, clear_depth: Option<f32>) -> Self {
        self.depth_stencil_attachment = Some(wgpu::RenderPassDepthStencilAttachment {
            view,
            depth_ops: Some(wgpu::Operations {
                load: if let Some(depth) = clear_depth {
                    wgpu::LoadOp::Clear(depth)
                } else {
                    wgpu::LoadOp::Load
                },
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        });
        self
    }

    pub fn begin(self) -> RenderPass<'a> {
        self.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: self.label,
            color_attachments: &self.color_attachments,
            depth_stencil_attachment: self.depth_stencil_attachment,
            timestamp_writes: None,
            occlusion_query_set: None,
        })
    }
}

/// Compute pass builder for simplified compute pass creation
pub struct ComputePassBuilder<'a> {
    encoder: &'a mut CommandEncoder,
    label: Option<&'a str>,
}

impl<'a> ComputePassBuilder<'a> {
    fn new(encoder: &'a mut CommandEncoder) -> Self {
        Self {
            encoder,
            label: None,
        }
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    pub fn begin(self) -> ComputePass<'a> {
        self.encoder
            .begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: self.label,
                timestamp_writes: None,
            })
    }
}

/// Extension trait for Queue operations
pub trait QueueExt {
    fn write_buffer_data<T: bytemuck::Pod>(&self, buffer: &Buffer, offset: u64, data: &T);
    fn write_buffer_slice<T: bytemuck::Pod>(&self, buffer: &Buffer, offset: u64, data: &[T]);
}

impl QueueExt for Queue {
    fn write_buffer_data<T: bytemuck::Pod>(&self, buffer: &Buffer, offset: u64, data: &T) {
        self.write_buffer(
            buffer,
            offset,
            bytemuck::cast_slice(std::slice::from_ref(data)),
        );
    }

    fn write_buffer_slice<T: bytemuck::Pod>(&self, buffer: &Buffer, offset: u64, data: &[T]) {
        self.write_buffer(buffer, offset, bytemuck::cast_slice(data));
    }
}
