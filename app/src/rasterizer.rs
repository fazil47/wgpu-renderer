use glam::Vec3A;
use wgpu::util::DeviceExt;

use crate::{camera::Camera, wgpu::Vertex};

pub fn initialize_rasterizer(
    camera: &Camera,
    color_uniform: &[f32; 4],
    sun_direction_uniform: &Vec3A,
    device: &wgpu::Device,
    surface: &wgpu::Surface,
    adapter: &wgpu::Adapter,
) -> (
    wgpu::Buffer,
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

    let sun_direction_uniform_buffer =
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sun Direction Uniform Buffer"),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&sun_direction_uniform.to_array()),
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
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
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
            wgpu::BindGroupEntry {
                binding: 2,
                resource: sun_direction_uniform_buffer.as_entire_binding(),
            },
        ],
    });

    let rasterizer_render_pipeline_layout =
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Rasterizer Render Pipeline Layout"),
            bind_group_layouts: &[&rasterizer_bind_group_layout],
            push_constant_ranges: &[],
        });

    let swapchain_capabilities = surface.get_capabilities(adapter);
    let swapchain_format = swapchain_capabilities.formats[0];

    let mut primitive = wgpu::PrimitiveState::default();
    primitive.cull_mode = Some(wgpu::Face::Back);

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
            primitive,
            depth_stencil: Some(wgpu::DepthStencilState {
                format: crate::wgpu::Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

    (
        camera_view_proj_uniform_buffer,
        color_uniform_buffer,
        sun_direction_uniform_buffer,
        rasterizer_bind_group,
        rasterizer_render_pipeline,
    )
}

pub fn render_rasterizer(
    render_encoder: &mut wgpu::CommandEncoder,
    surface_texture_view: &wgpu::TextureView,
    depth_texture: &crate::wgpu::Texture,
    vertex_buffer: &wgpu::Buffer,
    index_buffer: &wgpu::Buffer,
    num_indices: u32,
    rasterizer_bind_group: &wgpu::BindGroup,
    rasterizer_render_pipeline: &wgpu::RenderPipeline,
) {
    let mut rasterizer_rpass = render_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Rasterizer Render Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: surface_texture_view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: &depth_texture.view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
    });

    rasterizer_rpass.set_vertex_buffer(0, vertex_buffer.slice(..));
    rasterizer_rpass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
    rasterizer_rpass.set_bind_group(0, rasterizer_bind_group, &[]);
    rasterizer_rpass.set_pipeline(rasterizer_render_pipeline);
    rasterizer_rpass.draw_indexed(0..num_indices, 0, 0..1);
}

pub struct Rasterizer {
    pub depth_texture: crate::wgpu::Texture,
    pub camera_view_proj_uniform: wgpu::Buffer,
    pub color_uniform_buffer: wgpu::Buffer,
    pub sun_direction_uniform_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub render_pipeline: wgpu::RenderPipeline,
}
