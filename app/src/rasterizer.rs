use wgpu::util::DeviceExt;

use crate::{
    mesh::Mesh,
    scene::Scene,
    wgpu::{BufferExt, RendererWgpu, Vertex},
};

pub struct Rasterizer {
    pub render_pipeline: wgpu::RenderPipeline,
    pub bind_group: wgpu::BindGroup,
    pub camera_view_proj: wgpu::Buffer,
    pub sun_direction: wgpu::Buffer,
    pub depth_texture: crate::wgpu::Texture,
}

impl Rasterizer {
    pub fn new(wgpu: &RendererWgpu, scene: &Scene) -> Self {
        let rasterizer_shader = wgpu
            .device
            .create_shader_module(wgpu::include_wgsl!("shaders/rasterizer/main.wgsl"));

        let camera_view_proj = wgpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Camera View Projection Uniform Buffer"),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                contents: bytemuck::cast_slice(&[scene
                    .camera
                    .view_projection()
                    .to_cols_array_2d()]),
            });

        let sun_direction = wgpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Sun Direction Uniform Buffer"),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                contents: bytemuck::cast_slice(&scene.sun_light.direction.to_array()),
            });

        let bind_group_layout =
            wgpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let bind_group = wgpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Rasterizer Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_view_proj.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: sun_direction.as_entire_binding(),
                },
            ],
        });

        let render_pipeline_layout =
            wgpu.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Rasterizer Render Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let swapchain_capabilities = wgpu.surface.get_capabilities(&wgpu.adapter);
        let swapchain_format = swapchain_capabilities.formats[0];

        let mut primitive = wgpu::PrimitiveState::default();
        primitive.cull_mode = Some(wgpu::Face::Back);

        let render_pipeline = wgpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Rasterizer Render Pipeline"),
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &rasterizer_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Vertex::desc()],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &rasterizer_shader,
                    entry_point: Some("fs_main"),
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

        let depth_texture = crate::wgpu::Texture::create_depth_texture(
            &wgpu.device,
            &wgpu.surface_config,
            "rasterizer_depth_texture",
        );

        Self {
            camera_view_proj,
            sun_direction,
            bind_group,
            render_pipeline,
            depth_texture,
        }
    }

    pub fn resize(&mut self, wgpu: &RendererWgpu) {
        self.depth_texture = crate::wgpu::Texture::create_depth_texture(
            &wgpu.device,
            &wgpu.surface_config,
            "depth_texture",
        );
    }

    pub fn update_camera(&self, queue: &wgpu::Queue, scene: &Scene) {
        self.camera_view_proj
            .write(queue, &[scene.camera.view_projection().to_cols_array_2d()]);
    }

    pub fn update_light(&self, queue: &wgpu::Queue, scene: &Scene) {
        self.sun_direction
            .write(queue, &[scene.sun_light.direction.to_array()]);
    }

    pub fn render(
        &self,
        device: &wgpu::Device,
        render_encoder: &mut wgpu::CommandEncoder,
        surface_texture_view: &wgpu::TextureView,
        meshes: &Vec<Box<dyn Mesh>>,
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
                view: &self.depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        rasterizer_rpass.set_bind_group(0, &self.bind_group, &[]);
        rasterizer_rpass.set_pipeline(&self.render_pipeline);

        for mesh in meshes {
            let vertex_buffer = mesh.create_vertex_buffer(device);
            let index_buffer = mesh.create_index_buffer(device);
            let num_indices = mesh.get_index_count();

            rasterizer_rpass.set_vertex_buffer(0, vertex_buffer.slice(..));
            rasterizer_rpass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            rasterizer_rpass.draw_indexed(0..num_indices, 0, 0..1);
        }
    }
}
