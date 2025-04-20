use wesl::include_wesl;
use wgpu::util::DeviceExt;

use crate::{
    mesh::Material,
    scene::Scene,
    wgpu::{BufferExt, RendererWgpu, Vertex},
};

pub struct Rasterizer {
    other_buffers: RasterizerOtherBuffers,
    other_bind_group: wgpu::BindGroup,
    depth_texture: crate::wgpu::Texture,
    render_pipeline: wgpu::RenderPipeline,
    material_bind_groups: Vec<wgpu::BindGroup>,
}

impl Rasterizer {
    pub fn new(wgpu: &RendererWgpu, scene: &Scene) -> Self {
        let rasterizer_shader = wgpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Rasterizer Main Shader"),
                source: wgpu::ShaderSource::Wgsl(include_wesl!("rasterizer-main").into()),
            });

        let other_buffers = RasterizerOtherBuffers::new(&wgpu.device, scene);
        let bind_group_layouts = RasterizerBindGroupLayouts::new(&wgpu.device);
        let other_bind_group =
            create_other_bind_group(&wgpu.device, &other_buffers, &bind_group_layouts);

        let render_pipeline_layout =
            wgpu.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Rasterizer Render Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layouts.other, &bind_group_layouts.material],
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

        let material_bind_groups =
            Rasterizer::get_material_bind_groups(&wgpu.device, &bind_group_layouts, scene);

        Self {
            other_buffers,
            other_bind_group,
            depth_texture,
            render_pipeline,
            material_bind_groups,
        }
    }

    pub fn resize(&mut self, wgpu: &RendererWgpu) {
        self.depth_texture = crate::wgpu::Texture::create_depth_texture(
            &wgpu.device,
            &wgpu.surface_config,
            "rasterizer_depth_texture",
        );
    }

    pub fn update_camera(&self, queue: &wgpu::Queue, scene: &Scene) {
        self.other_buffers.update_camera(queue, scene);
    }

    pub fn update_light(&self, queue: &wgpu::Queue, scene: &Scene) {
        self.other_buffers.update_light(queue, scene);
    }

    pub fn render(
        &self,
        device: &wgpu::Device,
        render_encoder: &mut wgpu::CommandEncoder,
        surface_texture_view: &wgpu::TextureView,
        materials: &Vec<Material>,
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

        rasterizer_rpass.set_pipeline(&self.render_pipeline);
        rasterizer_rpass.set_bind_group(0, &self.other_bind_group, &[]);

        for i in 0..materials.len() {
            let material = &materials[i];
            rasterizer_rpass.set_bind_group(1, &self.material_bind_groups[i], &[]);

            let vertex_buffer = material.create_vertex_buffer(device);
            let index_buffer = material.create_index_buffer(device);
            let num_indices = material.get_index_count();

            rasterizer_rpass.set_vertex_buffer(0, vertex_buffer.slice(..));
            rasterizer_rpass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            rasterizer_rpass.draw_indexed(0..num_indices, 0, 0..1);
        }
    }

    fn get_material_bind_groups(
        device: &wgpu::Device,
        bind_group_layouts: &RasterizerBindGroupLayouts,
        scene: &Scene,
    ) -> Vec<wgpu::BindGroup> {
        scene
            .materials
            .iter()
            .map(|material| create_material_bind_group(device, bind_group_layouts, material))
            .collect()
    }
}

struct RasterizerMaterialBuffers {
    color: wgpu::Buffer,
}

impl RasterizerMaterialBuffers {
    fn new(device: &wgpu::Device, material: &Material) -> Self {
        let color = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Rasterizer Material Color Buffer"),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&material.color.to_array()),
        });

        Self { color }
    }
}

struct RasterizerOtherBuffers {
    camera_view_proj: wgpu::Buffer,
    sun_direction: wgpu::Buffer,
}

impl RasterizerOtherBuffers {
    fn new(device: &wgpu::Device, scene: &Scene) -> Self {
        let camera_view_proj = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Rasterizer Camera View Projection Uniform Buffer"),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&[scene.camera.view_projection().to_cols_array_2d()]),
        });

        let sun_direction = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Rasterizer Sun Direction Uniform Buffer"),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&scene.sun_light.direction.to_array()),
        });

        Self {
            camera_view_proj,
            sun_direction,
        }
    }

    fn update_camera(&self, queue: &wgpu::Queue, scene: &Scene) {
        self.camera_view_proj
            .write(queue, &[scene.camera.view_projection().to_cols_array_2d()]);
    }

    fn update_light(&self, queue: &wgpu::Queue, scene: &Scene) {
        self.sun_direction
            .write(queue, &[scene.sun_light.direction.to_array()]);
    }
}

struct RasterizerBindGroupLayouts {
    other: wgpu::BindGroupLayout,
    material: wgpu::BindGroupLayout,
}

impl RasterizerBindGroupLayouts {
    fn new(device: &wgpu::Device) -> Self {
        let other = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Rasterizer Other Bind Group Layout"),
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

        let material = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Rasterizer Material Bind Group Layout"),
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

        Self { other, material }
    }
}

fn create_other_bind_group(
    device: &wgpu::Device,
    buffers: &RasterizerOtherBuffers,
    layouts: &RasterizerBindGroupLayouts,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Rasterizer Other Bind Group"),
        layout: &layouts.other,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: buffers.camera_view_proj.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: buffers.sun_direction.as_entire_binding(),
            },
        ],
    })
}

fn create_material_bind_group(
    device: &wgpu::Device,
    layouts: &RasterizerBindGroupLayouts,
    material: &Material,
) -> wgpu::BindGroup {
    let buffers = RasterizerMaterialBuffers::new(device, material);

    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Rasterizer Material Bind Group"),
        layout: &layouts.material,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buffers.color.as_entire_binding(),
        }],
    })
}
