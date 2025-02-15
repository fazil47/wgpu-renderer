use wgpu::util::DeviceExt;

use crate::{
    camera,
    wgpu::{VERTEX_COLOR_OFFSET, VERTEX_NORMAL_OFFSET, VERTEX_STRIDE},
};

#[cfg(not(target_arch = "wasm32"))]
use wgpu::TextureFormat::Rgba8Unorm as RaytracerTextureFormat;

#[cfg(target_arch = "wasm32")]
use wgpu::TextureFormat::R32Float as RaytracerTextureFormat;

pub fn create_raytracer_result_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    // Create storage texture for the raytracer to write to
    let result_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: RaytracerTextureFormat,
        usage: wgpu::TextureUsages::STORAGE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let result_texture_view = result_texture.create_view(&wgpu::TextureViewDescriptor::default());

    (result_texture, result_texture_view)
}

pub fn initialize_raytracer(
    frame_count: usize,
    vertex_buffer: &wgpu::Buffer,
    index_buffer: &wgpu::Buffer,
    camera: &camera::Camera,
    sun_direction: &glam::Vec3A,
    result_texture_view: &wgpu::TextureView,
    device: &wgpu::Device,
    surface: &wgpu::Surface,
    adapter: &wgpu::Adapter,
) -> (
    wgpu::BindGroupLayout,
    wgpu::BindGroup,
    wgpu::RenderPipeline,
    wgpu::Buffer,
    wgpu::Buffer,
    wgpu::Buffer,
    wgpu::Buffer,
    wgpu::Buffer,
    wgpu::Buffer,
    wgpu::Buffer,
    wgpu::BindGroupLayout,
    wgpu::BindGroup,
    wgpu::ComputePipeline,
) {
    let raytracer_render_shader_source = include_str!("shaders/raytracer/render.wgsl");
    let raytracer_compute_shader_source = include_str!("shaders/raytracer/compute.wgsl");

    #[allow(unused_mut, unused_assignments)]
    let mut raytracer_render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Raytracer Render Shader"),
        source: wgpu::ShaderSource::Wgsl(raytracer_render_shader_source.into()),
    });
    #[allow(unused_mut, unused_assignments)]
    let mut raytracer_compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Raytracer Compute Shader"),
        source: wgpu::ShaderSource::Wgsl(raytracer_compute_shader_source.into()),
    });

    #[cfg(target_arch = "wasm32")]
    {
        // Replace the storage texture format with r32float for WebGPU
        let raytracer_render_shader_source = raytracer_render_shader_source.replace(
            "var result: texture_storage_2d<rgba8unorm",
            "var result: texture_storage_2d<r32float",
        );
        let raytracer_compute_shader_source = raytracer_compute_shader_source.replace(
            "var result: texture_storage_2d<rgba8unorm",
            "var result: texture_storage_2d<r32float",
        );

        raytracer_render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Raytracer Render Shader"),
            source: wgpu::ShaderSource::Wgsl(raytracer_render_shader_source.into()),
        });
        raytracer_compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Raytracer Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(raytracer_compute_shader_source.into()),
        });
    }

    let frame_count_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Frame Count Uniform Buffer"),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        contents: bytemuck::cast_slice(&[frame_count as u32]),
    });

    let vertex_stride_uniform_buffer =
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Stride Uniform Buffer"),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&[VERTEX_STRIDE]),
        });

    let vertex_color_offset_uniform_buffer =
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Color Offset Uniform Buffer"),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&[VERTEX_COLOR_OFFSET]),
        });

    let vertex_normal_offset_uniform_buffer =
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Normal Offset Uniform Buffer"),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&[VERTEX_NORMAL_OFFSET]),
        });

    let camera_to_world_uniform_buffer =
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera to World Uniform Buffer"),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&[camera.camera_to_world().to_cols_array_2d()]),
        });

    let camera_inverse_projection_uniform_buffer =
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Inverse Projection Uniform Buffer"),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&[camera
                .camera_inverse_projection()
                .to_cols_array_2d()]),
        });

    let sun_direction_uniform_buffer =
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sun Direction Uniform Buffer"),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&[sun_direction.extend(0.0).to_array()]),
        });

    let raytracer_render_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Raytracer Render Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::StorageTexture {
                    view_dimension: wgpu::TextureViewDimension::D2,
                    format: RaytracerTextureFormat,
                    access: wgpu::StorageTextureAccess::ReadOnly,
                },
                count: None,
            }],
        });
    let raytracer_compute_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Raytracer Compute Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 7,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 8,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 9,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        view_dimension: wgpu::TextureViewDimension::D2,
                        format: RaytracerTextureFormat,
                        access: wgpu::StorageTextureAccess::ReadWrite,
                    },
                    count: None,
                },
            ],
        });

    let (raytracer_render_bind_group, raytracer_compute_bind_group) = create_raytracer_bind_groups(
        result_texture_view,
        device,
        &raytracer_render_bind_group_layout,
        &raytracer_compute_bind_group_layout,
        vertex_buffer,
        index_buffer,
        &frame_count_uniform_buffer,
        &vertex_stride_uniform_buffer,
        &vertex_color_offset_uniform_buffer,
        &vertex_normal_offset_uniform_buffer,
        &camera_to_world_uniform_buffer,
        &camera_inverse_projection_uniform_buffer,
        &sun_direction_uniform_buffer,
    );

    let raytracer_render_pipeline_layout =
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Raytracer Render Pipeline Layout"),
            bind_group_layouts: &[&raytracer_render_bind_group_layout],
            push_constant_ranges: &[],
        });
    let raytracer_compute_pipeline_layout =
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Raytracer Compute Pipeline Layout"),
            bind_group_layouts: &[&raytracer_compute_bind_group_layout],
            push_constant_ranges: &[],
        });

    let swapchain_capabilities = surface.get_capabilities(adapter);
    let swapchain_format = swapchain_capabilities.formats[0];

    let raytracer_render_pipeline =
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Raytracer Render Pipeline"),
            layout: Some(&raytracer_render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &raytracer_render_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &raytracer_render_shader,
                entry_point: Some("fs_main"),
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
            cache: None,
        });
    let raytracer_compute_pipeline =
        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Raytracer Compute Pipeline"),
            layout: Some(&raytracer_compute_pipeline_layout),
            module: &raytracer_compute_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

    (
        raytracer_render_bind_group_layout,
        raytracer_render_bind_group,
        raytracer_render_pipeline,
        frame_count_uniform_buffer,
        vertex_stride_uniform_buffer,
        vertex_color_offset_uniform_buffer,
        vertex_normal_offset_uniform_buffer,
        camera_to_world_uniform_buffer,
        camera_inverse_projection_uniform_buffer,
        sun_direction_uniform_buffer,
        raytracer_compute_bind_group_layout,
        raytracer_compute_bind_group,
        raytracer_compute_pipeline,
    )
}

pub fn create_raytracer_bind_groups(
    result_texture_view: &wgpu::TextureView,
    device: &wgpu::Device,
    raytracer_render_bind_group_layout: &wgpu::BindGroupLayout,
    raytracer_compute_bind_group_layout: &wgpu::BindGroupLayout,
    vertex_buffer: &wgpu::Buffer,
    index_buffer: &wgpu::Buffer,
    frame_count_uniform_buffer: &wgpu::Buffer,
    vertex_stride_uniform_buffer: &wgpu::Buffer,
    vertex_color_offset_uniform_buffer: &wgpu::Buffer,
    vertex_normal_offset_uniform_buffer: &wgpu::Buffer,
    camera_to_world_uniform_buffer: &wgpu::Buffer,
    camera_inverse_projection_uniform_buffer: &wgpu::Buffer,
    sun_direction_uniform_buffer: &wgpu::Buffer,
) -> (wgpu::BindGroup, wgpu::BindGroup) {
    let raytracer_render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Raytracer Render Bind Group"),
        layout: raytracer_render_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(result_texture_view),
        }],
    });
    let raytracer_compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Raytracer Compute Bind Group"),
        layout: raytracer_compute_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: vertex_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: index_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: frame_count_uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: vertex_stride_uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: vertex_color_offset_uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: vertex_normal_offset_uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 6,
                resource: camera_to_world_uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 7,
                resource: camera_inverse_projection_uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 8,
                resource: sun_direction_uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 9,
                resource: wgpu::BindingResource::TextureView(result_texture_view),
            },
        ],
    });

    (raytracer_render_bind_group, raytracer_compute_bind_group)
}

pub fn render_raytracer(
    render_encoder: &mut wgpu::CommandEncoder,
    surface_texture_view: &wgpu::TextureView,
    raytracer_render_bind_group: &wgpu::BindGroup,
    raytracer_render_pipeline: &wgpu::RenderPipeline,
) {
    let mut raytracer_rpass = render_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Raytracer Render Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: surface_texture_view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    });

    raytracer_rpass.set_bind_group(0, raytracer_render_bind_group, &[]);
    raytracer_rpass.set_pipeline(raytracer_render_pipeline);
    raytracer_rpass.draw(0..3, 0..1);
}

pub fn run_raytracer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    window_size: winit::dpi::PhysicalSize<u32>,
    raytracer_compute_bind_group: &wgpu::BindGroup,
    raytracer_compute_pipeline: &wgpu::ComputePipeline,
) {
    let mut compute_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Compute Command Encoder"),
    });

    {
        let mut raytracer_cpass =
            compute_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Raytracer Compute Pass"),
                timestamp_writes: None,
            });

        raytracer_cpass.set_bind_group(0, raytracer_compute_bind_group, &[]);
        raytracer_cpass.set_pipeline(raytracer_compute_pipeline);
        raytracer_cpass.dispatch_workgroups(window_size.width / 8, window_size.height / 8, 1);
    }

    queue.submit(Some(compute_encoder.finish()));
}

pub struct Raytracer {
    pub result_texture: wgpu::Texture,
    pub result_texture_view: wgpu::TextureView,
    pub render_bind_group_layout: wgpu::BindGroupLayout,
    pub render_bind_group: wgpu::BindGroup,
    pub render_pipeline: wgpu::RenderPipeline,
    pub frame_count_uniform_buffer: wgpu::Buffer,
    pub vertex_stride_uniform_buffer: wgpu::Buffer,
    pub vertex_color_offset_uniform_buffer: wgpu::Buffer,
    pub vertex_normal_offset_uniform_buffer: wgpu::Buffer,
    pub camera_to_world_uniform_buffer: wgpu::Buffer,
    pub camera_inverse_projection_uniform_buffer: wgpu::Buffer,
    pub sun_direction_uniform_buffer: wgpu::Buffer,
    pub compute_bind_group_layout: wgpu::BindGroupLayout,
    pub compute_bind_group: wgpu::BindGroup,
    pub compute_pipeline: wgpu::ComputePipeline,
}
