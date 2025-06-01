use wesl::include_wesl;
use wgpu::util::DeviceExt;

use crate::{
    camera,
    mesh::{Material, RaytracerExt},
    scene::Scene,
    wgpu_utils::{QueueExt, WgpuExt},
};

#[cfg(target_arch = "wasm32")]
use wgpu::TextureFormat::R32Float as RaytracerTextureFormat;
#[cfg(not(target_arch = "wasm32"))]
use wgpu::TextureFormat::Rgba8Unorm as RaytracerTextureFormat;

pub struct Raytracer {
    pub buffers: RaytracerBuffers,
    pub bind_group_layouts: RaytracerBindGroupLayouts,
    bind_groups: RaytracerBindGroups,
    pipelines: RaytracerPipelines,
}

impl Raytracer {
    pub fn create_scene_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device
            .bind_group_layout()
            .label("Raytracer Scene Bind Group Layout")
            .storage(0, wgpu::ShaderStages::COMPUTE, true)
            .uniform(1, wgpu::ShaderStages::COMPUTE)
            .storage(2, wgpu::ShaderStages::COMPUTE, true)
            .uniform(3, wgpu::ShaderStages::COMPUTE)
            .uniform(4, wgpu::ShaderStages::COMPUTE)
            .uniform(5, wgpu::ShaderStages::COMPUTE)
            .storage(6, wgpu::ShaderStages::COMPUTE, true)
            .build()
    }

    pub fn create_material_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device
            .bind_group_layout()
            .label("Raytracer Compute Materials Bind Group Layout")
            .storage(0, wgpu::ShaderStages::COMPUTE, true)
            .uniform(1, wgpu::ShaderStages::COMPUTE)
            .build()
    }

    pub fn create_mesh_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device
            .bind_group_layout()
            .label("Raytracer Compute Meshes Bind Group Layout")
            .storage(0, wgpu::ShaderStages::COMPUTE, true)
            .uniform(1, wgpu::ShaderStages::COMPUTE)
            .uniform(2, wgpu::ShaderStages::COMPUTE)
            .uniform(3, wgpu::ShaderStages::COMPUTE)
            .storage(4, wgpu::ShaderStages::COMPUTE, true)
            .build()
    }

    pub fn new(
        wgpu: &crate::wgpu::RendererWgpu,
        window_size: &winit::dpi::PhysicalSize<u32>,
        scene: &Scene,
    ) -> Self {
        let shaders = RaytracerShaders::new(&wgpu.device);
        let buffers = RaytracerBuffers::new(&wgpu.device, window_size, scene);
        let bind_group_layouts = RaytracerBindGroupLayouts::new(&wgpu.device);

        // Use builder API for pipeline layouts
        let render_pipeline_layout = wgpu
            .device
            .pipeline_layout()
            .label("Raytracer Render Pipeline Layout")
            .bind_group_layout(&bind_group_layouts.render)
            .build();
        let compute_pipeline_layout = wgpu
            .device
            .pipeline_layout()
            .label("Raytracer Compute Pipeline Layout")
            .bind_group_layouts(&[
                &bind_group_layouts.compute_materials,
                &bind_group_layouts.compute_meshes,
                &bind_group_layouts.compute_lights,
                &bind_group_layouts.compute_result_camera,
            ])
            .build();

        // Use builder API for render pipeline
        let swapchain_capabilities = wgpu.surface.get_capabilities(&wgpu.adapter);
        let swapchain_format = swapchain_capabilities.formats[0];
        let render = wgpu
            .device
            .render_pipeline()
            .label("Raytracer Render Pipeline")
            .layout(&render_pipeline_layout)
            .vertex_shader(&shaders.render, "vs_main")
            .fragment_shader(&shaders.render, "fs_main")
            .color_target_alpha_blend(swapchain_format)
            .build()
            .unwrap();
        let compute = wgpu
            .device
            .compute_pipeline()
            .label("Raytracer Compute Pipeline")
            .layout(&compute_pipeline_layout)
            .shader(&shaders.compute, "main")
            .build()
            .unwrap();

        let pipelines = RaytracerPipelines { render, compute };
        let bind_groups = RaytracerBindGroups::new(&wgpu.device, &bind_group_layouts, &buffers);

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
        self.buffers.on_resize(wgpu, new_size);
        self.bind_groups
            .on_resize(&wgpu.device, &self.bind_group_layouts, &self.buffers);
    }

    pub fn update_frame_count(&self, queue: &wgpu::Queue, frame_count: u32) {
        self.buffers.update_frame_count(queue, frame_count);
    }

    pub fn update_camera(&self, queue: &wgpu::Queue, scene: &Scene) {
        self.buffers.update_camera(queue, scene);
    }

    pub fn update_light(&self, queue: &wgpu::Queue, scene: &Scene) {
        self.buffers.update_light(queue, scene);
    }

    pub fn get_material_bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_groups.compute_material
    }

    pub fn get_mesh_bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_groups.compute_mesh
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
            label: Some("Raytracer Compute Command Encoder"),
        });

        {
            let mut raytracer_cpass =
                compute_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Raytracer Compute Pass"),
                    timestamp_writes: None,
                });

            raytracer_cpass.set_bind_group(0, &self.bind_groups.compute_material, &[]);
            raytracer_cpass.set_bind_group(1, &self.bind_groups.compute_mesh, &[]);
            raytracer_cpass.set_bind_group(2, &self.bind_groups.compute_lights, &[]);
            raytracer_cpass.set_bind_group(3, &self.bind_groups.compute_result_camera, &[]);

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
        let render = device
            .shader()
            .label("Raytracer Render Shader")
            .wesl(include_wesl!("raytracer-render").into());
        let compute = device
            .shader()
            .label("Raytracer Compute Shader")
            .wesl(include_wesl!("raytracer-compute").into());

        Self { render, compute }
    }
}

pub struct RaytracerMaterialBuffers {
    pub materials: wgpu::Buffer,
    pub material_stride: wgpu::Buffer,
}

impl RaytracerMaterialBuffers {
    fn new(device: &wgpu::Device, materials: &Vec<Material>) -> Self {
        Self {
            materials: materials.create_materials_buffer(device),
            material_stride: materials.create_material_stride_buffer(device),
        }
    }
}

pub struct RaytracerMeshBuffers {
    pub vertices: wgpu::Buffer,
    pub vertex_stride: wgpu::Buffer,
    pub vertex_normal_offset: wgpu::Buffer,
    pub vertex_material_offset: wgpu::Buffer,
    pub indices: wgpu::Buffer,
}

impl RaytracerMeshBuffers {
    fn new(device: &wgpu::Device, materials: &Vec<Material>) -> Self {
        Self {
            vertices: materials.create_vertices_buffer(device),
            vertex_stride: materials.create_vertex_stride_buffer(device),
            vertex_normal_offset: materials.create_vertex_normal_buffer(device),
            vertex_material_offset: materials.create_vertex_material_buffer(device),
            indices: materials.create_indices_buffer(device),
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
            label: Some("Raytracer Camera to World Uniform Buffer"),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&[camera.camera_to_world().to_cols_array_2d()]),
        });
        let camera_inverse_projection =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Raytracer Camera Inverse Projection Uniform Buffer"),
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

pub struct RaytracerOtherBuffers {
    sun_direction: wgpu::Buffer,
    camera: RaytracerCameraBuffers,
    frame_count: wgpu::Buffer,
}

impl RaytracerOtherBuffers {
    fn new(device: &wgpu::Device, scene: &Scene) -> Self {
        let camera = RaytracerCameraBuffers::new(device, &scene.camera);
        let sun_direction = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Raytracer Sun Direction Uniform Buffer"),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&[scene.sun_light.direction.to_array()]),
        });
        let frame_count = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Raytracer Frame Count Uniform Buffer"),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&[0]),
        });
        Self {
            sun_direction,
            camera,
            frame_count,
        }
    }

    fn update_frame_count(&self, queue: &wgpu::Queue, frame_count: u32) {
        queue.write_buffer_data(&self.frame_count, 0, &frame_count);
    }

    fn update_camera(&self, queue: &wgpu::Queue, scene: &Scene) {
        queue.write_buffer_data(
            &self.camera.camera_to_world,
            0,
            &scene.camera.camera_to_world().to_cols_array_2d(),
        );
        queue.write_buffer_data(
            &self.camera.camera_inverse_projection,
            0,
            &scene.camera.camera_inverse_projection().to_cols_array_2d(),
        );
    }

    fn update_light(&self, queue: &wgpu::Queue, scene: &Scene) {
        queue.write_buffer_data(
            &self.sun_direction,
            0,
            &scene.sun_light.direction.to_array(),
        );
    }
}

pub struct RaytracerBuffers {
    pub materials: RaytracerMaterialBuffers,
    pub meshes: RaytracerMeshBuffers,
    pub result: wgpu::TextureView,
    pub other: RaytracerOtherBuffers,
}

impl RaytracerBuffers {
    fn new(
        device: &wgpu::Device,
        window_size: &winit::dpi::PhysicalSize<u32>,
        scene: &Scene,
    ) -> Self {
        let materials = RaytracerMaterialBuffers::new(device, &scene.materials);
        let meshes = RaytracerMeshBuffers::new(device, &scene.materials);
        let result = Self::create_result_texture(device, window_size);
        let other = RaytracerOtherBuffers::new(device, scene);
        Self {
            materials,
            meshes,
            result,
            other,
        }
    }
    fn on_resize(
        &mut self,
        wgpu: &crate::wgpu::RendererWgpu,
        new_size: &winit::dpi::PhysicalSize<u32>,
    ) {
        self.update_frame_count(&wgpu.queue, 0);
        self.result = Self::create_result_texture(&wgpu.device, new_size);
    }
    fn update_frame_count(&self, queue: &wgpu::Queue, frame_count: u32) {
        self.other.update_frame_count(queue, frame_count);
    }
    fn update_camera(&self, queue: &wgpu::Queue, scene: &Scene) {
        self.other.update_camera(queue, scene);
    }
    fn update_light(&self, queue: &wgpu::Queue, scene: &Scene) {
        self.other.update_light(queue, scene);
    }
    fn create_result_texture(
        device: &wgpu::Device,
        window_size: &winit::dpi::PhysicalSize<u32>,
    ) -> wgpu::TextureView {
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
    compute_materials: wgpu::BindGroupLayout,
    compute_meshes: wgpu::BindGroupLayout,
    compute_lights: wgpu::BindGroupLayout,
    compute_result_camera: wgpu::BindGroupLayout,
}

impl RaytracerBindGroupLayouts {
    fn new(device: &wgpu::Device) -> Self {
        let render = device
            .bind_group_layout()
            .label("Raytracer Render Bind Group Layout")
            .storage_texture_2d(
                0,
                wgpu::ShaderStages::FRAGMENT,
                wgpu::StorageTextureAccess::ReadOnly,
                RaytracerTextureFormat,
            )
            .build();
        let compute_materials = Raytracer::create_material_bind_group_layout(device);
        let compute_meshes = Raytracer::create_mesh_bind_group_layout(device);
        let compute_lights = device
            .bind_group_layout()
            .label("Raytracer Compute Lights Bind Group Layout")
            .uniform(0, wgpu::ShaderStages::COMPUTE)
            .build();
        let compute_result_camera = device
            .bind_group_layout()
            .label("Raytracer Compute Result Camera Bind Group Layout")
            .storage_texture_2d(
                0,
                wgpu::ShaderStages::COMPUTE,
                wgpu::StorageTextureAccess::ReadWrite,
                RaytracerTextureFormat,
            )
            .uniform(1, wgpu::ShaderStages::COMPUTE)
            .uniform(2, wgpu::ShaderStages::COMPUTE)
            .uniform(3, wgpu::ShaderStages::COMPUTE)
            .build();
        Self {
            render,
            compute_materials,
            compute_meshes,
            compute_lights,
            compute_result_camera,
        }
    }
}

struct RaytracerBindGroups {
    render: wgpu::BindGroup,
    compute_material: wgpu::BindGroup,
    compute_mesh: wgpu::BindGroup,
    compute_lights: wgpu::BindGroup,
    compute_result_camera: wgpu::BindGroup,
}

impl RaytracerBindGroups {
    fn new(
        device: &wgpu::Device,
        bgl: &RaytracerBindGroupLayouts,
        buffers: &RaytracerBuffers,
    ) -> Self {
        let render = device
            .bind_group(&bgl.render)
            .label("Raytracer Render Bind Group")
            .texture(0, &buffers.result)
            .build();
        let compute_material = device
            .bind_group(&bgl.compute_materials)
            .label("Raytracer Compute Material Bind Group")
            .buffer(0, &buffers.materials.materials)
            .buffer(1, &buffers.materials.material_stride)
            .build();
        let compute_mesh = device
            .bind_group(&bgl.compute_meshes)
            .label("Raytracer Compute Mesh Bind Group")
            .buffer(0, &buffers.meshes.vertices)
            .buffer(1, &buffers.meshes.vertex_stride)
            .buffer(2, &buffers.meshes.vertex_normal_offset)
            .buffer(3, &buffers.meshes.vertex_material_offset)
            .buffer(4, &buffers.meshes.indices)
            .build();
        let compute_lights = device
            .bind_group(&bgl.compute_lights)
            .label("Raytracer Compute Lights Bind Group")
            .buffer(0, &buffers.other.sun_direction)
            .build();
        let compute_result_camera = device
            .bind_group(&bgl.compute_result_camera)
            .label("Raytracer Compute Result Camera Bind Group")
            .texture(0, &buffers.result)
            .buffer(1, &buffers.other.camera.camera_to_world)
            .buffer(2, &buffers.other.camera.camera_inverse_projection)
            .buffer(3, &buffers.other.frame_count)
            .build();
        Self {
            render,
            compute_material,
            compute_mesh,
            compute_lights,
            compute_result_camera,
        }
    }
    fn on_resize(
        &mut self,
        device: &wgpu::Device,
        bgl: &RaytracerBindGroupLayouts,
        buffers: &RaytracerBuffers,
    ) {
        self.render = device
            .bind_group(&bgl.render)
            .label("Raytracer Render Bind Group")
            .texture(0, &buffers.result)
            .build();
        self.compute_result_camera = device
            .bind_group(&bgl.compute_result_camera)
            .label("Raytracer Compute Result Camera Bind Group")
            .texture(0, &buffers.result)
            .buffer(1, &buffers.other.camera.camera_to_world)
            .buffer(2, &buffers.other.camera.camera_inverse_projection)
            .buffer(3, &buffers.other.frame_count)
            .build();
    }
}

// Add RaytracerPipelines struct
struct RaytracerPipelines {
    render: wgpu::RenderPipeline,
    compute: wgpu::ComputePipeline,
}
