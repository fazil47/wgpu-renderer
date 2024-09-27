use crate::wgpu::{update_buffer, Resolution, Vertex, RGBA};

pub fn initialize_raytracer_shader(
    color_uniform: [f32; 4],
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    surface: &wgpu::Surface,
    adapter: &wgpu::Adapter,
) -> (
    wgpu::ShaderModule,
    wgpu::Buffer,
    wgpu::Buffer,
    wgpu::BindGroup,
    wgpu::PipelineLayout,
    wgpu::RenderPipeline,
) {
    // Load the shaders from disk
    let raytracer_shader =
        device.create_shader_module(wgpu::include_wgsl!("shaders/raytracer/main.wgsl"));

    let color_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Color Uniform Buffer"),
        size: std::mem::size_of::<RGBA>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    // Update the color buffer with the initial color
    update_buffer(queue, &color_uniform_buffer, RGBA::new(color_uniform));

    let resolution_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Resolution Uniform Buffer"),
        size: std::mem::size_of::<Resolution>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let color_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Raytracer Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

    let raytracer_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Raytracer Bind Group"),
        layout: &color_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: color_uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: resolution_uniform_buffer.as_entire_binding(),
            },
        ],
    });

    let raytracer_render_pipeline_layout =
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&color_bind_group_layout],
            push_constant_ranges: &[],
        });

    let swapchain_capabilities = surface.get_capabilities(&adapter);
    let swapchain_format = swapchain_capabilities.formats[0];

    let raytracer_render_pipeline =
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Raytracer Render Pipeline"),
            layout: Some(&raytracer_render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &raytracer_shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &raytracer_shader,
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
        raytracer_shader,
        color_uniform_buffer,
        resolution_uniform_buffer,
        raytracer_bind_group,
        raytracer_render_pipeline_layout,
        raytracer_render_pipeline,
    )
}
