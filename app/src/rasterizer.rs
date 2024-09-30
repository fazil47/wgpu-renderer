use crate::wgpu::{update_buffer, Vertex, RGBA};

pub fn initialize_rasterizer(
    color_uniform: [f32; 4],
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    surface: &wgpu::Surface,
    adapter: &wgpu::Adapter,
) -> (wgpu::Buffer, wgpu::BindGroup, wgpu::RenderPipeline) {
    // Load the shaders from disk
    let rasterizer_shader =
        device.create_shader_module(wgpu::include_wgsl!("shaders/rasterizer/main.wgsl"));

    let color_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Color Uniform Buffer"),
        size: std::mem::size_of::<RGBA>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    // Update the color buffer with the initial color
    update_buffer(queue, &color_uniform_buffer, RGBA::new(color_uniform));

    let color_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Rasterizer Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

    let color_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Rasterizer Bind Group"),
        layout: &color_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: &color_uniform_buffer,
                offset: 0,
                size: None,
            }),
        }],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Pipeline Layout"),
        bind_group_layouts: &[&color_bind_group_layout],
        push_constant_ranges: &[],
    });

    let swapchain_capabilities = surface.get_capabilities(&adapter);
    let swapchain_format = swapchain_capabilities.formats[0];

    let rasterizer_render_pipeline =
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Rasterizer Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &rasterizer_shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &rasterizer_shader,
                entry_point: "fs_main",
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: swapchain_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

    (
        color_uniform_buffer,
        color_bind_group,
        rasterizer_render_pipeline,
    )
}
