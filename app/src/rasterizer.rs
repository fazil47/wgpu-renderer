use wgpu::util::DeviceExt;

use crate::{camera::Camera, wgpu::Vertex};

pub fn initialize_rasterizer(
    camera: &Camera,
    color_uniform: &[f32; 4],
    device: &wgpu::Device,
    surface: &wgpu::Surface,
    adapter: &wgpu::Adapter,
) -> (
    wgpu::Buffer,
    wgpu::Buffer,
    wgpu::BindGroup,
    wgpu::RenderPipeline,
) {
    // Load the shaders from disk
    let rasterizer_shader =
        device.create_shader_module(wgpu::include_wgsl!("shaders/rasterizer/main.wgsl"));

    let camera_view_proj_uniform_buffer =
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera View Projection Uniform Buffer"),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&[camera.view_projection().to_cols_array_2d()]),
        });

    let color_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Color Uniform Buffer"),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        contents: bytemuck::cast_slice(color_uniform),
    });

    let rasterizer_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Rasterizer Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
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

    let rasterizer_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Rasterizer Bind Group"),
        layout: &rasterizer_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_view_proj_uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: color_uniform_buffer.as_entire_binding(),
            },
        ],
    });

    let rasterizer_render_pipeline_layout =
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Rasterizer Render Pipeline Layout"),
            bind_group_layouts: &[&rasterizer_bind_group_layout],
            push_constant_ranges: &[],
        });

    let swapchain_capabilities = surface.get_capabilities(&adapter);
    let swapchain_format = swapchain_capabilities.formats[0];

    let rasterizer_render_pipeline =
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Rasterizer Render Pipeline"),
            layout: Some(&rasterizer_render_pipeline_layout),
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
        camera_view_proj_uniform_buffer,
        color_uniform_buffer,
        rasterizer_bind_group,
        rasterizer_render_pipeline,
    )
}
