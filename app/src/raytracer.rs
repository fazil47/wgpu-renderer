use wgpu::util::DeviceExt;

use crate::{
    camera,
    wgpu::{BufferExt, VERTEX_COLOR_OFFSET, VERTEX_NORMAL_OFFSET, VERTEX_STRIDE},
};

#[cfg(not(target_arch = "wasm32"))]
use wgpu::TextureFormat::Rgba8Unorm as RaytracerTextureFormat;

#[cfg(target_arch = "wasm32")]
use wgpu::TextureFormat::R32Float as RaytracerTextureFormat;

pub struct Raytracer {
    buffers: RaytracerBuffers,
    bind_group_layouts: RaytracerBindGroupLayouts,
    bind_groups: RaytracerBindGroups,
    pipelines: RaytracerPipelines,
}

impl Raytracer {
    pub fn new(
        window_size: &winit::dpi::PhysicalSize<u32>,
        wgpu: &crate::wgpu::RendererWgpu,
        vertex_buffer: &wgpu::Buffer,
        index_buffer: &wgpu::Buffer,
        sun_direction: &maths::Vec3,
        camera: &camera::Camera,
        frame_count: usize,
    ) -> Self {
        let shaders = RaytracerShaders::new(&wgpu.device);
        let buffers = RaytracerBuffers::new(
            &wgpu.device,
            vertex_buffer,
            index_buffer,
            window_size,
            camera,
            &sun_direction,
            frame_count,
        );
        let bind_group_layouts = RaytracerBindGroupLayouts::new(&wgpu.device);
        let bind_groups = RaytracerBindGroups::new(&wgpu.device, &bind_group_layouts, &buffers);
        let pipeline_layouts = RaytracerPipelineLayouts::new(&wgpu.device, &bind_group_layouts);
        let pipelines = RaytracerPipelines::new(wgpu, &shaders, &pipeline_layouts);

        Self {
            buffers,
            bind_group_layouts,
            bind_groups,
            pipelines,
        }
    }

    pub fn resize(
        &mut self,
        new_size: &winit::dpi::PhysicalSize<u32>,
        wgpu: &crate::wgpu::RendererWgpu,
    ) {
        self.buffers.on_resize(&wgpu.device, new_size);
        self.bind_groups
            .on_resize(&wgpu.device, &self.bind_group_layouts, &self.buffers);
    }

    pub fn update_frame_count(&self, queue: &wgpu::Queue, frame_count: u32) {
        self.buffers.update_frame_count(queue, frame_count);
    }

    pub fn update_camera(&self, queue: &wgpu::Queue, camera_controller: &camera::CameraController) {
        self.buffers.update_camera(queue, camera_controller);
    }

    pub fn render(
        &self,
        render_encoder: &mut wgpu::CommandEncoder,
        surface_texture_view: &wgpu::TextureView,
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

        raytracer_rpass.set_bind_group(0, &self.bind_groups.render, &[]);
        raytracer_rpass.set_pipeline(&self.pipelines.render);
        raytracer_rpass.draw(0..3, 0..1);
    }

    pub fn compute(
        &self,
        window_size: &winit::dpi::PhysicalSize<u32>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
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

            raytracer_cpass.set_bind_group(0, &self.bind_groups.compute_mesh, &[]);
            raytracer_cpass.set_bind_group(1, &self.bind_groups.compute_result, &[]);
            raytracer_cpass.set_bind_group(2, &self.bind_groups.compute_light, &[]);
            raytracer_cpass.set_bind_group(3, &self.bind_groups.compute_camera, &[]);

            raytracer_cpass.set_pipeline(&self.pipelines.compute);
            raytracer_cpass.dispatch_workgroups(window_size.width / 8, window_size.height / 8, 1);
        }

        queue.submit(Some(compute_encoder.finish()));
    }
}

struct RaytracerShaders {
    render: wgpu::ShaderModule,
    compute: wgpu::ShaderModule,
}

impl RaytracerShaders {
    fn new(device: &wgpu::Device) -> Self {
        let render_source = include_str!("shaders/raytracer/render.wgsl");
        let compute_source = include_str!("shaders/raytracer/compute.wgsl");

        #[allow(unused_mut, unused_assignments)]
        let mut render = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Raytracer Render Shader"),
            source: wgpu::ShaderSource::Wgsl(render_source.into()),
        });
        #[allow(unused_mut, unused_assignments)]
        let mut compute = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Raytracer Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(compute_source.into()),
        });

        #[cfg(target_arch = "wasm32")]
        {
            // Replace the storage texture format with r32float for WebGPU
            let render_source = render_source.replace(
                "var result: texture_storage_2d<rgba8unorm",
                "var result: texture_storage_2d<r32float",
            );
            let compute_source = compute_source.replace(
                "var result: texture_storage_2d<rgba8unorm",
                "var result: texture_storage_2d<r32float",
            );

            render = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Raytracer Render Shader"),
                source: wgpu::ShaderSource::Wgsl(render_source.into()),
            });
            compute = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Raytracer Compute Shader"),
                source: wgpu::ShaderSource::Wgsl(compute_source.into()),
            });
        }

        Self { render, compute }
    }
}

struct RaytracerMeshBuffers {
    vertex: wgpu::Buffer,
    vertex_stride: wgpu::Buffer,
    vertex_color_offset: wgpu::Buffer,
    vertex_normal_offset: wgpu::Buffer,
    index: wgpu::Buffer,
}

impl RaytracerMeshBuffers {
    fn new(
        device: &wgpu::Device,
        vertex_buffer: &wgpu::Buffer,
        index_buffer: &wgpu::Buffer,
    ) -> Self {
        let vertex_stride = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Stride Uniform Buffer"),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&[VERTEX_STRIDE]),
        });
        let vertex_color_offset = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Color Offset Uniform Buffer"),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&[VERTEX_COLOR_OFFSET]),
        });
        let vertex_normal_offset = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Normal Offset Uniform Buffer"),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&[VERTEX_NORMAL_OFFSET]),
        });

        Self {
            vertex: vertex_buffer.clone(),
            vertex_stride,
            vertex_color_offset,
            vertex_normal_offset,
            index: index_buffer.clone(),
        }
    }
}

struct RaytracerCameraBuffers {
    camera_to_world: wgpu::Buffer,
    camera_inverse_projection: wgpu::Buffer,
}

impl RaytracerCameraBuffers {
    fn new(device: &wgpu::Device, camera: &camera::Camera) -> Self {
        let camera_to_world = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera to World Uniform Buffer"),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&[camera.camera_to_world().to_cols_array_2d()]),
        });
        let camera_inverse_projection =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Camera Inverse Projection Uniform Buffer"),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                contents: bytemuck::cast_slice(&[camera
                    .camera_inverse_projection()
                    .to_cols_array_2d()]),
            });

        Self {
            camera_to_world,
            camera_inverse_projection,
        }
    }
}

struct RaytracerBuffers {
    mesh: RaytracerMeshBuffers,
    result: wgpu::TextureView,
    sun_direction: wgpu::Buffer,
    camera: RaytracerCameraBuffers,
    frame_count: wgpu::Buffer,
}

impl RaytracerBuffers {
    fn new(
        device: &wgpu::Device,
        vertex_buffer: &wgpu::Buffer,
        index_buffer: &wgpu::Buffer,
        window_size: &winit::dpi::PhysicalSize<u32>,
        camera: &camera::Camera,
        sun_direction: &maths::Vec3,
        frame_count: usize,
    ) -> Self {
        let mesh = RaytracerMeshBuffers::new(device, vertex_buffer, index_buffer);
        let result = Self::create_result_texture(device, window_size);
        let camera = RaytracerCameraBuffers::new(device, camera);
        let sun_direction = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sun Direction Uniform Buffer"),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&[sun_direction.to_array()]),
        });
        let frame_count = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Frame Count Uniform Buffer"),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&[frame_count as u32]),
        });

        Self {
            mesh,
            result,
            sun_direction,
            camera,
            frame_count,
        }
    }

    fn on_resize(&mut self, device: &wgpu::Device, new_size: &winit::dpi::PhysicalSize<u32>) {
        // Recreate the result texture with the new size
        self.result = Self::create_result_texture(device, new_size);
    }

    fn update_frame_count(&self, queue: &wgpu::Queue, frame_count: u32) {
        self.frame_count.write(queue, &[frame_count]);
    }

    fn update_camera(&self, queue: &wgpu::Queue, camera_controller: &camera::CameraController) {
        self.camera.camera_to_world.write(
            queue,
            &[camera_controller
                .camera
                .camera_to_world()
                .to_cols_array_2d()],
        );

        self.camera.camera_inverse_projection.write(
            queue,
            &[camera_controller
                .camera
                .camera_inverse_projection()
                .to_cols_array_2d()],
        );
    }

    fn create_result_texture(
        device: &wgpu::Device,
        window_size: &winit::dpi::PhysicalSize<u32>,
    ) -> wgpu::TextureView {
        // Create storage texture for the raytracer compute shader to write to
        let result_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: window_size.width,
                height: window_size.height,
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

        result_texture.create_view(&wgpu::TextureViewDescriptor::default())
    }
}

struct RaytracerBindGroupLayouts {
    render: wgpu::BindGroupLayout,
    compute_mesh: wgpu::BindGroupLayout,
    compute_result: wgpu::BindGroupLayout,
    compute_light: wgpu::BindGroupLayout,
    compute_camera: wgpu::BindGroupLayout,
}

impl RaytracerBindGroupLayouts {
    fn new(device: &wgpu::Device) -> Self {
        let render = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let compute_mesh = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Raytracer Compute Mesh Bind Group Layout"),
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
                        ty: wgpu::BufferBindingType::Uniform,
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
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let compute_result = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Raytracer Compute Result Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::StorageTexture {
                    view_dimension: wgpu::TextureViewDimension::D2,
                    format: RaytracerTextureFormat,
                    access: wgpu::StorageTextureAccess::ReadWrite,
                },
                count: None,
            }],
        });
        let compute_light = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Raytracer Compute Light Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let compute_camera = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Raytracer Compute Camera Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
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
            ],
        });

        Self {
            render,
            compute_mesh,
            compute_result,
            compute_light,
            compute_camera,
        }
    }
}

struct RaytracerBindGroups {
    render: wgpu::BindGroup,
    compute_mesh: wgpu::BindGroup,
    compute_result: wgpu::BindGroup,
    compute_light: wgpu::BindGroup,
    compute_camera: wgpu::BindGroup,
}

impl RaytracerBindGroups {
    fn new(
        device: &wgpu::Device,
        bind_group_layouts: &RaytracerBindGroupLayouts,
        buffers: &RaytracerBuffers,
    ) -> Self {
        let render = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Raytracer Render Bind Group"),
            layout: &bind_group_layouts.render,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&buffers.result),
            }],
        });

        let compute_mesh = Self::create_compute_mesh_bind_group(
            device,
            &bind_group_layouts.compute_mesh,
            &buffers.mesh,
        );
        let compute_result = Self::create_compute_result_bind_group(
            device,
            &bind_group_layouts.compute_result,
            &buffers.result,
        );
        let compute_light = Self::create_compute_light_bind_group(
            device,
            &bind_group_layouts.compute_light,
            &buffers.sun_direction,
        );
        let compute_camera = Self::create_compute_camera_bind_group(
            device,
            &bind_group_layouts.compute_camera,
            &buffers.camera,
            &buffers.frame_count,
        );

        Self {
            render,
            compute_mesh,
            compute_result,
            compute_light,
            compute_camera,
        }
    }

    fn on_resize(
        &mut self,
        device: &wgpu::Device,
        bind_group_layouts: &RaytracerBindGroupLayouts,
        buffers: &RaytracerBuffers,
    ) {
        // Recreate the compute result bind group with the new texture view
        self.compute_result = Self::create_compute_result_bind_group(
            device,
            &bind_group_layouts.compute_result,
            &buffers.result,
        );
    }

    fn create_compute_mesh_bind_group(
        device: &wgpu::Device,
        compute_mesh_bind_group_layout: &wgpu::BindGroupLayout,
        mesh_buffers: &RaytracerMeshBuffers,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Raytracer Compute Mesh Bind Group"),
            layout: compute_mesh_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: mesh_buffers.vertex.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: mesh_buffers.vertex_stride.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: mesh_buffers.vertex_color_offset.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: mesh_buffers.vertex_normal_offset.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: mesh_buffers.index.as_entire_binding(),
                },
            ],
        })
    }

    fn create_compute_result_bind_group(
        device: &wgpu::Device,
        compute_result_bind_group_layout: &wgpu::BindGroupLayout,
        result_texture_view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Raytracer Compute Result Bind Group"),
            layout: compute_result_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(result_texture_view),
            }],
        })
    }

    fn create_compute_light_bind_group(
        device: &wgpu::Device,
        compute_light_bind_group_layout: &wgpu::BindGroupLayout,
        sun_direction_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Raytracer Compute Light Bind Group"),
            layout: compute_light_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: sun_direction_buffer.as_entire_binding(),
            }],
        })
    }

    fn create_compute_camera_bind_group(
        device: &wgpu::Device,
        compute_camera_bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffers: &RaytracerCameraBuffers,
        frame_count_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Raytracer Compute Camera Bind Group"),
            layout: compute_camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffers.camera_to_world.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: camera_buffers.camera_inverse_projection.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: frame_count_buffer.as_entire_binding(),
                },
            ],
        })
    }
}

struct RaytracerPipelineLayouts {
    render: wgpu::PipelineLayout,
    compute: wgpu::PipelineLayout,
}

impl RaytracerPipelineLayouts {
    fn new(device: &wgpu::Device, bind_group_layouts: &RaytracerBindGroupLayouts) -> Self {
        let render = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Raytracer Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layouts.render],
            push_constant_ranges: &[],
        });
        let compute = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Raytracer Compute Pipeline Layout"),
            bind_group_layouts: &[
                &bind_group_layouts.compute_mesh,
                &bind_group_layouts.compute_result,
                &bind_group_layouts.compute_light,
                &bind_group_layouts.compute_camera,
            ],
            push_constant_ranges: &[],
        });

        Self { render, compute }
    }
}

struct RaytracerPipelines {
    render: wgpu::RenderPipeline,
    compute: wgpu::ComputePipeline,
}

impl RaytracerPipelines {
    fn new(
        wgpu: &crate::wgpu::RendererWgpu,
        shaders: &RaytracerShaders,
        pipeline_layouts: &RaytracerPipelineLayouts,
    ) -> Self {
        let swapchain_capabilities = wgpu.surface.get_capabilities(&wgpu.adapter);
        let swapchain_format = swapchain_capabilities.formats[0];

        let render = wgpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Raytracer Render Pipeline"),
                layout: Some(&pipeline_layouts.render),
                vertex: wgpu::VertexState {
                    module: &shaders.render,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shaders.render,
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
        let compute = wgpu
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Raytracer Compute Pipeline"),
                layout: Some(&pipeline_layouts.compute),
                module: &shaders.compute,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });

        Self { render, compute }
    }
}
