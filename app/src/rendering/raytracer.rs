use std::mem::{offset_of, size_of};
use wesl::include_wesl;

use crate::{
    rendering::{
        material::Material,
        rasterizer::Vertex,
        wgpu_utils::{CameraBuffers, LightingBuffers, WgpuExt},
    },
    // scene::crate::scene::Scene, // Removed - using World directly
};
use ecs::{EntityId, World};

// Raytracer-specific vertex and material types
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RaytracerVertex {
    pub position: [f32; 4],
    pub normal: [f32; 4],
    pub material_id: f32,
}

impl RaytracerVertex {
    pub fn from_vertex(vertex: &Vertex, material_id: usize) -> Self {
        Self {
            position: vertex.position,
            normal: vertex.normal,
            material_id: material_id as f32,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RaytracerMaterial {
    pub color: [f32; 4],
}

impl RaytracerMaterial {
    pub fn from_material(material: &Material) -> Self {
        Self {
            color: material.color.to_array(),
        }
    }

    pub fn from_mesh_material(material: &crate::mesh::Material) -> Self {
        Self {
            color: material.color.to_array(),
        }
    }
}

// Raytracer vertex constants
pub const RAYTRACE_MATERIAL_STRIDE: u32 =
    (size_of::<RaytracerMaterial>() / size_of::<f32>()) as u32;
pub const RAYTRACE_VERTEX_STRIDE: u32 = (size_of::<RaytracerVertex>() / size_of::<f32>()) as u32;
pub const RAYTRACE_VERTEX_NORMAL_OFFSET: u32 =
    (offset_of!(RaytracerVertex, normal) / size_of::<f32>()) as u32;
pub const RAYTRACE_VERTEX_MATERIAL_ID_OFFSET: u32 =
    (offset_of!(RaytracerVertex, material_id) / size_of::<f32>()) as u32;

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
        wgpu: &crate::rendering::wgpu_utils::WgpuResources,
        window_size: &winit::dpi::PhysicalSize<u32>,
        scene: &crate::scene::Scene,
        world: &World,
        camera_entity: EntityId,
        sun_light_entity: EntityId,
    ) -> Self {
        let shaders = RaytracerShaders::new(&wgpu.device);
        let buffers = RaytracerBuffers::new(
            &wgpu.device,
            window_size,
            scene,
            world,
            camera_entity,
            sun_light_entity,
        );
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
        wgpu: &crate::rendering::wgpu_utils::WgpuResources,
    ) {
        self.buffers.on_resize(wgpu, new_size);
        self.bind_groups
            .on_resize(&wgpu.device, &self.bind_group_layouts, &self.buffers);
    }

    pub fn update_frame_count(&self, queue: &wgpu::Queue, frame_count: u32) {
        self.buffers.update_frame_count(queue, frame_count);
    }

    pub fn update_camera(&self, queue: &wgpu::Queue, world: &World, camera_entity: EntityId) {
        self.buffers.update_camera(queue, world, camera_entity);
    }

    pub fn update_light(&self, queue: &wgpu::Queue, world: &World, sun_light_entity: EntityId) {
        self.buffers.update_light(queue, world, sun_light_entity);
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

// Removed RaytracerCameraBuffers - now using unified CameraBuffers from buffer_utils

pub struct RaytracerBuffers {
    // Material buffers
    pub materials: wgpu::Buffer,
    pub material_stride: wgpu::Buffer,

    // Mesh buffers
    pub vertices: wgpu::Buffer,
    pub vertex_stride: wgpu::Buffer,
    pub vertex_normal_offset: wgpu::Buffer,
    pub vertex_material_offset: wgpu::Buffer,
    pub indices: wgpu::Buffer,

    // Other buffers
    pub lighting: LightingBuffers,
    pub camera: CameraBuffers,
    pub frame_count: wgpu::Buffer,
    pub result: wgpu::TextureView,
}

impl RaytracerBuffers {
    fn new(
        device: &wgpu::Device,
        window_size: &winit::dpi::PhysicalSize<u32>,
        scene: &crate::scene::Scene,
        world: &World,
        camera_entity: EntityId,
        sun_light_entity: EntityId,
    ) -> Self {
        // Create material buffers - sort by ID to ensure consistent ordering
        let mut sorted_materials: Vec<_> = scene.materials().iter().collect();
        sorted_materials.sort_by_key(|(id, _)| id.0);

        let materials: Vec<RaytracerMaterial> = sorted_materials
            .iter()
            .map(|(_, material)| RaytracerMaterial::from_material(material))
            .collect();

        let materials_data = if materials.is_empty() {
            vec![RaytracerMaterial {
                color: [1.0, 1.0, 1.0, 1.0],
            }]
        } else {
            materials
        };

        let materials_buffer = device
            .buffer()
            .label("Raytracer Materials Buffer")
            .storage(&materials_data);

        let material_stride = device
            .buffer()
            .label("Raytracer Material Stride Buffer")
            .uniform(&RAYTRACE_MATERIAL_STRIDE);

        // Create mesh buffers
        let mut all_vertices = Vec::new();
        let mut all_indices = Vec::new();

        // Get mesh data from ECS entities
        let renderable_entities = world.get_entities_with_3::<crate::rendering::Transform, crate::mesh::Mesh, crate::rendering::MaterialRef>()
            .into_iter()
            .filter(|&entity_id| world.has_component::<crate::rendering::Renderable>(entity_id))
            .collect::<Vec<_>>();

        // Use centralized Scene material indexing for consistency with rasterizer

        let mut vertex_offset = 0u32;
        for entity_id in &renderable_entities {
            if let (Some(mesh_component), Some(material_ref_component)) = (
                world.get_component::<crate::mesh::Mesh>(*entity_id),
                world.get_component::<crate::rendering::MaterialRef>(*entity_id),
            ) {
                let mesh = mesh_component.borrow();
                let material_ref = material_ref_component.borrow();

                // Get material index using centralized Scene indexing for consistency
                let material_index = scene
                    .get_material_index_for_entity(material_ref.material_entity)
                    .unwrap_or(0);

                // Convert mesh vertices to raytracer format
                for vertex in mesh.vertices() {
                    all_vertices.push(RaytracerVertex::from_vertex(vertex, material_index));
                }

                // Add indices with vertex offset
                for &index in mesh.indices() {
                    all_indices.push(index + vertex_offset);
                }

                vertex_offset += mesh.vertices().len() as u32;
            }
        }

        // Fallback if no entities found
        if all_vertices.is_empty() {
            all_vertices.push(RaytracerVertex {
                position: [0.0, 0.0, 0.0, 1.0],
                normal: [0.0, 1.0, 0.0, 0.0],
                material_id: 0.0,
            });
            all_indices.push(0);
        }

        let vertices = device
            .buffer()
            .label("Raytracer Vertices Buffer")
            .storage(&all_vertices);

        let vertex_stride = device
            .buffer()
            .label("Raytracer Vertex Stride Buffer")
            .uniform(&RAYTRACE_VERTEX_STRIDE);

        let vertex_normal_offset = device
            .buffer()
            .label("Raytracer Vertex Normal Offset Buffer")
            .uniform(&RAYTRACE_VERTEX_NORMAL_OFFSET);

        let vertex_material_offset = device
            .buffer()
            .label("Raytracer Vertex Material Offset Buffer")
            .uniform(&RAYTRACE_VERTEX_MATERIAL_ID_OFFSET);

        let indices = device
            .buffer()
            .label("Raytracer Indices Buffer")
            .storage(&all_indices);

        // Create other buffers
        let lighting = LightingBuffers::new(device, world, sun_light_entity, "Raytracer");
        let camera = CameraBuffers::new(device, world, camera_entity, "Raytracer");
        let frame_count = device
            .buffer()
            .label("Raytracer Frame Count Buffer")
            .uniform(&0u32);

        let result = Self::create_result_texture(device, window_size);

        Self {
            materials: materials_buffer,
            material_stride,
            vertices,
            vertex_stride,
            vertex_normal_offset,
            vertex_material_offset,
            indices,
            lighting,
            camera,
            frame_count,
            result,
        }
    }
    fn on_resize(
        &mut self,
        wgpu: &crate::rendering::wgpu_utils::WgpuResources,
        new_size: &winit::dpi::PhysicalSize<u32>,
    ) {
        self.update_frame_count(&wgpu.queue, 0);
        self.result = Self::create_result_texture(&wgpu.device, new_size);
    }
    fn update_frame_count(&self, queue: &wgpu::Queue, frame_count: u32) {
        queue.write_buffer(&self.frame_count, 0, bytemuck::cast_slice(&[frame_count]));
    }
    fn update_camera(&self, queue: &wgpu::Queue, world: &World, camera_entity: EntityId) {
        self.camera.update_from_world(queue, world, camera_entity);
    }
    fn update_light(&self, queue: &wgpu::Queue, world: &World, sun_light_entity: EntityId) {
        self.lighting
            .update_from_world(queue, world, sun_light_entity);
    }
    fn create_result_texture(
        device: &wgpu::Device,
        window_size: &winit::dpi::PhysicalSize<u32>,
    ) -> wgpu::TextureView {
        let result_texture = device
            .texture()
            .label("Raytracer Result Texture")
            .size_2d(window_size.width, window_size.height)
            .format(RaytracerTextureFormat)
            .usage(
                wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::COPY_SRC,
            )
            .build();
        result_texture.create_view(&wgpu::TextureViewDescriptor::default())
    }
}

pub struct RaytracerBindGroupLayouts {
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
            .buffer(0, &buffers.materials)
            .buffer(1, &buffers.material_stride)
            .build();
        let compute_mesh = device
            .bind_group(&bgl.compute_meshes)
            .label("Raytracer Compute Mesh Bind Group")
            .buffer(0, &buffers.vertices)
            .buffer(1, &buffers.vertex_stride)
            .buffer(2, &buffers.vertex_normal_offset)
            .buffer(3, &buffers.vertex_material_offset)
            .buffer(4, &buffers.indices)
            .build();
        let compute_lights = device
            .bind_group(&bgl.compute_lights)
            .label("Raytracer Compute Lights Bind Group")
            .buffer(0, &buffers.lighting.sun_direction)
            .build();
        let compute_result_camera = device
            .bind_group(&bgl.compute_result_camera)
            .label("Raytracer Compute Result Camera Bind Group")
            .texture(0, &buffers.result)
            .buffer(1, &buffers.camera.camera_to_world)
            .buffer(2, &buffers.camera.camera_inverse_projection)
            .buffer(3, &buffers.frame_count)
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
            .buffer(1, &buffers.camera.camera_to_world)
            .buffer(2, &buffers.camera.camera_inverse_projection)
            .buffer(3, &buffers.frame_count)
            .build();
    }
}

// Add RaytracerPipelines struct
struct RaytracerPipelines {
    render: wgpu::RenderPipeline,
    compute: wgpu::ComputePipeline,
}
