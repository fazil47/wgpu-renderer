use std::{
    collections::HashMap,
    mem::{offset_of, size_of},
};
use wgpu::Device;

use crate::{
    material::{DefaultMaterialEntity, Material},
    mesh::Vertex,
    rendering::{
        extract::{Extract, ExtractionError, WorldExtractExt},
        raytracer::bvh::{Aabb, BvhNode, build_blas, build_bvh_debug_lines, build_tlas},
        wgpu::{CameraBuffers, LightingBuffers, WgpuExt, WgpuResources},
    },
};
use ecs::{Entity, World};
use maths::{Mat4, Vec3};

pub mod bvh;

// Raytracer-specific vertex and material types
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RaytracerVertex {
    pub position: [f32; 4],
    pub normal: [f32; 4],
    pub material_index: f32,
}

impl RaytracerVertex {
    pub fn from_vertex(vertex: &Vertex, material_index: usize, transform: Mat4) -> Self {
        let position = transform * vertex.position;
        let normal = transform * vertex.normal;
        Self {
            position: position.to_array(),
            normal: normal.to_array(),
            material_index: material_index as f32,
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

    pub fn from_mesh_material(material: &Material) -> Self {
        Self {
            color: material.color.to_array(),
        }
    }
}

const BVH_INTERIOR_COLOR: [f32; 4] = [0.2, 0.6, 1.0, 1.0];
const BVH_LEAF_COLOR: [f32; 4] = [1.0, 0.4, 0.2, 1.0];

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RaytracerBvhLineVertex {
    pub position: [f32; 4],
    pub color: [f32; 4],
    pub instance_index: u32,
}

impl RaytracerBvhLineVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Float32x4,
        1 => Float32x4,
        2 => Uint32
    ];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }

    pub fn from_vec3(position: Vec3, color: [f32; 4], instance_index: u32) -> Self {
        Self {
            position: [position.x, position.y, position.z, 1.0],
            color,
            instance_index,
        }
    }

    fn zero() -> Self {
        Self {
            position: [0.0, 0.0, 0.0, 1.0],
            color: [0.0, 0.0, 0.0, 0.0],
            instance_index: 0,
        }
    }
}

// Raytracer vertex constants
pub const RAYTRACE_MATERIAL_STRIDE: u32 =
    (size_of::<RaytracerMaterial>() / size_of::<f32>()) as u32;
pub const RAYTRACE_VERTEX_STRIDE: u32 = (size_of::<RaytracerVertex>() / size_of::<f32>()) as u32;
pub const RAYTRACE_VERTEX_NORMAL_OFFSET: u32 =
    (offset_of!(RaytracerVertex, normal) / size_of::<f32>()) as u32;
pub const RAYTRACE_VERTEX_MATERIAL_INDEX_OFFSET: u32 =
    (offset_of!(RaytracerVertex, material_index) / size_of::<f32>()) as u32;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RaytracerInstance {
    pub world_matrix: [[f32; 4]; 4],
    pub blas_index: u32,
    pub _padding: [u32; 3],
}

impl RaytracerInstance {
    pub fn new(world_matrix: Mat4, blas_index: u32) -> Self {
        Self {
            world_matrix: world_matrix.to_cols_array_2d(),
            blas_index,
            _padding: [0; 3],
        }
    }
}

impl Default for RaytracerInstance {
    fn default() -> Self {
        Self {
            world_matrix: Mat4::IDENTITY.to_cols_array_2d(),
            blas_index: 0,
            _padding: [0; 3],
        }
    }
}

// Since there will be one BLAS per unique mesh, we need to store the BLAS info for each of those meshes
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RaytracerBlasInfo {
    pub node_offset: u32,
    pub node_count: u32,
    pub primitive_offset: u32,
    pub primitive_count: u32,
    pub vertex_offset: u32,
    pub index_offset: u32,
    pub _padding: [u32; 2],
}

impl RaytracerBlasInfo {
    pub fn new(
        node_offset: u32,
        node_count: u32,
        primitive_offset: u32,
        primitive_count: u32,
        vertex_offset: u32,
        index_offset: u32,
    ) -> Self {
        Self {
            node_offset,
            node_count,
            primitive_offset,
            primitive_count,
            vertex_offset,
            index_offset,
            _padding: [0; 2],
        }
    }
}

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

impl ecs::Resource for Raytracer {}

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
            .label("Raytracer Materials Bind Group Layout")
            .storage(0, wgpu::ShaderStages::COMPUTE, true)
            .build()
    }

    pub fn create_mesh_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device
            .bind_group_layout()
            .label("Raytracer Mesh Bind Group Layout")
            .storage(0, wgpu::ShaderStages::COMPUTE, true)
            .storage(1, wgpu::ShaderStages::COMPUTE, true)
            .storage(2, wgpu::ShaderStages::COMPUTE, true)
            .storage(3, wgpu::ShaderStages::COMPUTE, true)
            .storage(4, wgpu::ShaderStages::COMPUTE, true)
            .storage(5, wgpu::ShaderStages::COMPUTE, true)
            .storage(6, wgpu::ShaderStages::COMPUTE, true)
            .storage(7, wgpu::ShaderStages::COMPUTE, true)
            .build()
    }

    pub fn new(wgpu: &WgpuResources, window_size: &winit::dpi::PhysicalSize<u32>) -> Self {
        let shaders = RaytracerShaders::new(&wgpu.device);
        let buffers = RaytracerBuffers::new(&wgpu.device, window_size);
        let bind_group_layouts = RaytracerBindGroupLayouts::new(&wgpu.device);

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
        let bvh_lines_pipeline_layout = wgpu
            .device
            .pipeline_layout()
            .label("Raytracer BVH Line Pipeline Layout")
            .bind_group_layout(&bind_group_layouts.bvh_lines_camera)
            .build();
        let bvh_lines = wgpu
            .device
            .render_pipeline()
            .label("Raytracer BVH Line Pipeline")
            .layout(&bvh_lines_pipeline_layout)
            .vertex_shader(&shaders.bvh_lines, "vs_main")
            .fragment_shader(&shaders.bvh_lines, "fs_main")
            .vertex_buffer(RaytracerBvhLineVertex::desc())
            .topology(wgpu::PrimitiveTopology::LineList)
            .color_target_alpha_blend(swapchain_format)
            .build()
            .unwrap();

        let pipelines = RaytracerPipelines {
            render,
            compute,
            bvh_lines,
        };
        let bind_groups = RaytracerBindGroups::new(&wgpu.device, &bind_group_layouts, &buffers);

        Self {
            buffers,
            bind_group_layouts,
            bind_groups,
            pipelines,
        }
    }

    pub fn update_render_data(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        world: &World,
        camera_entity: Entity,
        sun_light_entity: Entity,
    ) -> Result<Option<bvh::Bvh>, ExtractionError> {
        let RaytracerExtractedBuffers {
            materials,
            vertices,
            indices,
            bvh_lines,
            bvh_line_vertex_count,
            blas_nodes,
            blas_node_count,
            blas_primitive_indices,
            blas_primitive_count,
            blas_info,
            tlas_nodes,
            tlas_node_count,
            tlas_primitive_indices,
            tlas_primitive_count,
            instances,
            instance_count,
            tlas_bvh,
        } = self.extract(device, world)?;

        self.buffers.materials = materials;
        self.buffers.vertices = vertices;
        self.buffers.indices = indices;
        self.buffers.bvh_lines = bvh_lines;
        self.buffers.bvh_line_vertex_count = bvh_line_vertex_count;
        self.buffers.blas_nodes = blas_nodes;
        self.buffers.blas_node_count = blas_node_count;
        self.buffers.blas_primitive_indices = blas_primitive_indices;
        self.buffers.blas_primitive_count = blas_primitive_count;
        self.buffers.blas_info = blas_info;
        self.buffers.tlas_nodes = tlas_nodes;
        self.buffers.tlas_node_count = tlas_node_count;
        self.buffers.tlas_primitive_indices = tlas_primitive_indices;
        self.buffers.tlas_primitive_count = tlas_primitive_count;
        self.buffers.instances = instances;
        self.buffers.instance_count = instance_count;

        self.buffers
            .lighting
            .update_from_world(queue, world, sun_light_entity);
        self.buffers
            .camera
            .update_from_world(queue, world, camera_entity);

        self.bind_groups =
            RaytracerBindGroups::new(device, &self.bind_group_layouts, &self.buffers);

        Ok(Some(tlas_bvh))
    }

    pub fn resize(&mut self, new_size: &winit::dpi::PhysicalSize<u32>, wgpu: &WgpuResources) {
        self.buffers.on_resize(wgpu, new_size);
        self.bind_groups
            .on_resize(&wgpu.device, &self.bind_group_layouts, &self.buffers);
    }

    pub fn update_frame_count(&self, queue: &wgpu::Queue, frame_count: u32) {
        self.buffers.update_frame_count(queue, frame_count);
    }

    pub fn update_camera(&self, queue: &wgpu::Queue, world: &World, camera_entity: Entity) {
        self.buffers.update_camera(queue, world, camera_entity);
    }

    pub fn update_light(&self, queue: &wgpu::Queue, world: &World, sun_light_entity: Entity) {
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
        show_bvh: bool,
    ) {
        let mut rpass = render_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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

        rpass.set_bind_group(0, &self.bind_groups.render, &[]);
        rpass.set_pipeline(&self.pipelines.render);
        rpass.draw(0..3, 0..1);

        if show_bvh && self.buffers.bvh_line_vertex_count > 0 {
            rpass.set_pipeline(&self.pipelines.bvh_lines);
            rpass.set_bind_group(0, &self.bind_groups.bvh_lines_camera, &[]);
            rpass.set_vertex_buffer(0, self.buffers.bvh_lines.slice(..));
            rpass.draw(0..self.buffers.bvh_line_vertex_count, 0..1);
        }
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
            let mut cpass = compute_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Raytracer Compute Pass"),
                timestamp_writes: None,
            });

            cpass.set_bind_group(0, &self.bind_groups.compute_material, &[]);
            cpass.set_bind_group(1, &self.bind_groups.compute_mesh, &[]);
            cpass.set_bind_group(2, &self.bind_groups.compute_lights, &[]);
            cpass.set_bind_group(3, &self.bind_groups.compute_result_camera, &[]);

            cpass.set_pipeline(&self.pipelines.compute);
            cpass.dispatch_workgroups(window_size.width / 8, window_size.height / 8, 1);
        }

        queue.submit(Some(compute_encoder.finish()));
    }
}

struct RaytracerShaders {
    render: wgpu::ShaderModule,
    compute: wgpu::ShaderModule,
    bvh_lines: wgpu::ShaderModule,
}

impl RaytracerShaders {
    fn new(device: &wgpu::Device) -> Self {
        let render = device
            .shader()
            .label("Raytracer Render Shader")
            .define_u32("MATERIAL_STRIDE", RAYTRACE_MATERIAL_STRIDE)
            .wesl_runtime("package::raytracer::render");
        let compute = device
            .shader()
            .label("Raytracer Compute Shader")
            .define_u32("MATERIAL_STRIDE", RAYTRACE_MATERIAL_STRIDE)
            .define_u32("VERTEX_STRIDE", RAYTRACE_VERTEX_STRIDE)
            .define_u32("VERTEX_NORMAL_OFFSET", RAYTRACE_VERTEX_NORMAL_OFFSET)
            .define_u32(
                "VERTEX_MATERIAL_OFFSET",
                RAYTRACE_VERTEX_MATERIAL_INDEX_OFFSET,
            )
            .wesl_runtime("package::raytracer::compute");
        let bvh_lines = device
            .shader()
            .label("Raytracer BVH Line Shader")
            .wesl_runtime("package::raytracer::bvh_lines");

        Self {
            render,
            compute,
            bvh_lines,
        }
    }
}

pub struct RaytracerBuffers {
    // Material buffers
    pub materials: wgpu::Buffer,

    // Mesh buffers
    pub vertices: wgpu::Buffer,
    pub indices: wgpu::Buffer,
    pub bvh_lines: wgpu::Buffer,
    pub bvh_line_vertex_count: u32,
    pub blas_nodes: wgpu::Buffer,
    pub blas_node_count: u32,
    pub blas_primitive_indices: wgpu::Buffer,
    pub blas_primitive_count: u32,
    pub blas_info: wgpu::Buffer,
    pub tlas_nodes: wgpu::Buffer,
    pub tlas_node_count: u32,
    pub tlas_primitive_indices: wgpu::Buffer,
    pub tlas_primitive_count: u32,
    pub instances: wgpu::Buffer,
    pub instance_count: u32,

    // Other buffers
    pub lighting: LightingBuffers,
    pub camera: CameraBuffers,
    pub frame_count: wgpu::Buffer,
    pub result: wgpu::TextureView,
}

impl RaytracerBuffers {
    fn new(device: &wgpu::Device, window_size: &winit::dpi::PhysicalSize<u32>) -> Self {
        let initial_materials = vec![RaytracerMaterial {
            color: [1.0, 1.0, 1.0, 1.0],
        }];

        let materials_buffer = device
            .buffer()
            .label("Raytracer Materials Buffer")
            .storage(&initial_materials);

        let initial_vertices = vec![RaytracerVertex {
            position: [0.0, 0.0, 0.0, 1.0],
            normal: [0.0, 1.0, 0.0, 0.0],
            material_index: 0.0,
        }];
        let initial_indices = vec![0u32];

        let vertices = device
            .buffer()
            .label("Raytracer Vertices Buffer")
            .storage(&initial_vertices);

        let indices = device
            .buffer()
            .label("Raytracer Indices Buffer")
            .storage(&initial_indices);

        let initial_bvh_lines = [
            RaytracerBvhLineVertex::zero(),
            RaytracerBvhLineVertex::zero(),
        ];
        let bvh_lines = device
            .buffer()
            .label("Raytracer BVH Line Buffer")
            .vertex(&initial_bvh_lines);

        let initial_bvh_node = [BvhNode::default()];
        let blas_nodes = device
            .buffer()
            .label("Raytracer BLAS Node Buffer")
            .storage(&initial_bvh_node);

        let initial_bvh_primitive_indices = [0u32];
        let blas_primitive_indices = device
            .buffer()
            .label("Raytracer BLAS Primitive Indices Buffer")
            .storage(&initial_bvh_primitive_indices);

        let blas_info = device
            .buffer()
            .label("Raytracer BLAS Info Buffer")
            .storage(&[RaytracerBlasInfo::default()]);

        let tlas_nodes = device
            .buffer()
            .label("Raytracer TLAS Node Buffer")
            .storage(&[BvhNode::default()]);

        let tlas_primitive_indices = device
            .buffer()
            .label("Raytracer TLAS Primitive Indices Buffer")
            .storage(&[0u32]);

        let instances = device
            .buffer()
            .label("Raytracer TLAS Instance Buffer")
            .storage(&[RaytracerInstance::default()]);

        let lighting = LightingBuffers::new(device, "Raytracer");
        let camera = CameraBuffers::new(device, "Raytracer");
        let frame_count = device
            .buffer()
            .label("Raytracer Frame Count Buffer")
            .uniform(&0u32);

        let result = Self::create_result_texture(device, window_size);

        Self {
            materials: materials_buffer,
            vertices,
            indices,
            bvh_lines,
            bvh_line_vertex_count: 0,
            blas_nodes,
            blas_node_count: 0,
            blas_primitive_indices,
            blas_primitive_count: 0,
            blas_info,
            tlas_nodes,
            tlas_node_count: 0,
            tlas_primitive_indices,
            tlas_primitive_count: 0,
            instances,
            instance_count: 0,
            lighting,
            camera,
            frame_count,
            result,
        }
    }
    fn on_resize(&mut self, wgpu: &WgpuResources, new_size: &winit::dpi::PhysicalSize<u32>) {
        self.update_frame_count(&wgpu.queue, 0);
        self.result = Self::create_result_texture(&wgpu.device, new_size);
    }
    fn update_frame_count(&self, queue: &wgpu::Queue, frame_count: u32) {
        queue.write_buffer(&self.frame_count, 0, bytemuck::cast_slice(&[frame_count]));
    }
    fn update_camera(&self, queue: &wgpu::Queue, world: &World, camera_entity: Entity) {
        self.camera.update_from_world(queue, world, camera_entity);
    }
    fn update_light(&self, queue: &wgpu::Queue, world: &World, sun_light_entity: Entity) {
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
    bvh_lines_camera: wgpu::BindGroupLayout,
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
        let bvh_lines_camera = device
            .bind_group_layout()
            .label("Raytracer BVH Line Camera Bind Group Layout")
            .uniform(0, wgpu::ShaderStages::VERTEX)
            .storage(1, wgpu::ShaderStages::VERTEX, true)
            .build();
        Self {
            render,
            compute_materials,
            compute_meshes,
            compute_lights,
            compute_result_camera,
            bvh_lines_camera,
        }
    }
}

struct RaytracerBindGroups {
    render: wgpu::BindGroup,
    compute_material: wgpu::BindGroup,
    compute_mesh: wgpu::BindGroup,
    compute_lights: wgpu::BindGroup,
    compute_result_camera: wgpu::BindGroup,
    bvh_lines_camera: wgpu::BindGroup,
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
            .build();
        let compute_mesh = device
            .bind_group(&bgl.compute_meshes)
            .label("Raytracer Compute Mesh Bind Group")
            .buffer(0, &buffers.vertices)
            .buffer(1, &buffers.indices)
            .buffer(2, &buffers.blas_nodes)
            .buffer(3, &buffers.blas_primitive_indices)
            .buffer(4, &buffers.blas_info)
            .buffer(5, &buffers.tlas_nodes)
            .buffer(6, &buffers.instances)
            .buffer(7, &buffers.tlas_primitive_indices)
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
        let bvh_lines_camera = device
            .bind_group(&bgl.bvh_lines_camera)
            .label("Raytracer BVH Line Camera Bind Group")
            .buffer(0, &buffers.camera.view_projection)
            .buffer(1, &buffers.instances)
            .build();
        Self {
            render,
            compute_material,
            compute_mesh,
            compute_lights,
            compute_result_camera,
            bvh_lines_camera,
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

struct RaytracerPipelines {
    render: wgpu::RenderPipeline,
    compute: wgpu::ComputePipeline,
    bvh_lines: wgpu::RenderPipeline,
}

pub struct RaytracerExtractedBuffers {
    pub materials: wgpu::Buffer,
    pub vertices: wgpu::Buffer,
    pub indices: wgpu::Buffer,
    pub bvh_lines: wgpu::Buffer,
    pub bvh_line_vertex_count: u32,
    pub blas_nodes: wgpu::Buffer,
    pub blas_node_count: u32,
    pub blas_primitive_indices: wgpu::Buffer,
    pub blas_primitive_count: u32,
    pub blas_info: wgpu::Buffer,
    pub tlas_nodes: wgpu::Buffer,
    pub tlas_node_count: u32,
    pub tlas_primitive_indices: wgpu::Buffer,
    pub tlas_primitive_count: u32,
    pub instances: wgpu::Buffer,
    pub instance_count: u32,
    pub tlas_bvh: bvh::Bvh,
}

impl Extract for Raytracer {
    type ExtractedData = RaytracerExtractedBuffers;

    fn extract(
        &self,
        device: &Device,
        world: &World,
    ) -> Result<Self::ExtractedData, ExtractionError> {
        let material_entities = world.get_materials();
        let mut materials = Vec::new();
        let mut material_entity_to_index = HashMap::new();
        let default_material_entity = world.get_resource::<DefaultMaterialEntity>().unwrap().0;
        let mut default_material_index = None;

        for entity in material_entities {
            let material = world.extract_material_component(entity)?;
            let material_index = materials.len();
            materials.push(RaytracerMaterial::from_material(&material));
            material_entity_to_index.insert(entity, material_index);

            if entity == default_material_entity {
                default_material_index = Some(material_index);
            }
        }

        let default_material_index = default_material_index.unwrap();

        let renderables = world.get_renderables();

        // Aggregated buffers
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut blas_nodes = Vec::new();
        let mut blas_primitive_indices = Vec::new();
        let mut blas_infos = Vec::new();
        let mut instances = Vec::new();

        // Debug lines
        let mut blas_lines_per_instance: Vec<(u32, Vec<bvh::BvhDebugLine>)> = Vec::new();

        // Build per-entity BLAS and TLAS leaves (one BLAS per renderable for simplicity)
        for entity in renderables {
            let global_transform = world.extract_global_transform_component(entity)?;
            let mesh = world.extract_mesh_component(entity)?;
            let material_index = if let Some(mat_entity) = mesh.material_entity {
                *material_entity_to_index
                    .get(&mat_entity)
                    .expect("Material entity not found for mesh")
            } else {
                default_material_index
            };

            let mesh_vertices: Vec<RaytracerVertex> = mesh
                .vertices()
                .iter()
                .map(|vertex| RaytracerVertex::from_vertex(vertex, material_index, Mat4::IDENTITY))
                .collect();

            let mesh_indices: Vec<u32> = match mesh.indices() {
                Some(mesh_indices) => mesh_indices.to_vec(),
                None => (0..mesh.vertices().len() as u32).collect(),
            };

            let blas = build_blas(&mesh_vertices, &mesh_indices);
            let blas_index = blas_infos.len() as u32;
            let node_offset = blas_nodes.len() as u32;
            let node_count = blas.nodes.len() as u32;
            let primitive_offset = blas_primitive_indices.len() as u32;
            let primitive_count = blas.primitive_indices.len() as u32;
            blas_nodes.extend_from_slice(&blas.nodes);
            blas_primitive_indices.extend_from_slice(&blas.primitive_indices);

            let vertex_offset = vertices.len() as u32;
            vertices.extend_from_slice(&mesh_vertices);
            let index_offset = indices.len() as u32;
            indices.extend_from_slice(&mesh_indices);

            blas_infos.push(RaytracerBlasInfo::new(
                node_offset,
                node_count,
                primitive_offset,
                primitive_count,
                vertex_offset,
                index_offset,
            ));

            let transform_matrix = global_transform.matrix;
            instances.push(RaytracerInstance::new(transform_matrix, blas_index));

            let instance_index = (instances.len() as u32).saturating_sub(1);
            let blas_lines = build_bvh_debug_lines(&blas);
            blas_lines_per_instance.push((instance_index, blas_lines));
        }

        // Build TLAS using world-space bounds of each instance
        let mut instance_bounds: Vec<(Vec3, Vec3, u32)> = Vec::new();
        for (instance_index, instance) in instances.iter().enumerate() {
            let blas_index = instance.blas_index as usize;
            let blas_info = &blas_infos[blas_index];
            let bounds = if blas_info.node_count == 0 {
                (Vec3::ZERO, Vec3::ZERO)
            } else {
                let node = &blas_nodes[blas_info.node_offset as usize];
                (
                    Vec3::new(node.bounds_min[0], node.bounds_min[1], node.bounds_min[2]),
                    Vec3::new(node.bounds_max[0], node.bounds_max[1], node.bounds_max[2]),
                )
            };
            let world_matrix = Mat4::from_cols_array_2d(instance.world_matrix);
            let aabb = Aabb::new(bounds.0, bounds.1).transform(world_matrix);
            instance_bounds.push((aabb.min, aabb.max, instance_index as u32));
        }

        let tlas_bvh = build_tlas(&instance_bounds);
        let tlas_node_count = tlas_bvh.nodes.len() as u32;
        let tlas_nodes_buffer = if tlas_bvh.nodes.is_empty() {
            device
                .buffer()
                .label("Raytracer TLAS Node Buffer")
                .storage(&[BvhNode::default()])
        } else {
            device
                .buffer()
                .label("Raytracer TLAS Node Buffer")
                .storage(&tlas_bvh.nodes)
        };
        let tlas_primitive_count = tlas_bvh.primitive_indices.len() as u32;
        let tlas_primitive_indices_buffer = if tlas_bvh.primitive_indices.is_empty() {
            device
                .buffer()
                .label("Raytracer TLAS Primitive Indices Buffer")
                .storage(&[0u32])
        } else {
            device
                .buffer()
                .label("Raytracer TLAS Primitive Indices Buffer")
                .storage(&tlas_bvh.primitive_indices)
        };

        // TLAS debug lines (world space, no transform)
        let tlas_debug_lines = build_bvh_debug_lines(&tlas_bvh);
        let mut line_vertices = Vec::new();
        for line in tlas_debug_lines {
            let color = if line.is_leaf {
                BVH_LEAF_COLOR
            } else {
                BVH_INTERIOR_COLOR
            };
            line_vertices.push(RaytracerBvhLineVertex::from_vec3(
                line.start,
                color,
                u32::MAX,
            ));
            line_vertices.push(RaytracerBvhLineVertex::from_vec3(line.end, color, u32::MAX));
        }

        // BLAS debug lines (object space, transformed on GPU per instance)
        for (instance_index, lines) in blas_lines_per_instance {
            for line in lines {
                let color = if line.is_leaf {
                    BVH_LEAF_COLOR
                } else {
                    BVH_INTERIOR_COLOR
                };
                line_vertices.push(RaytracerBvhLineVertex::from_vec3(
                    line.start,
                    color,
                    instance_index,
                ));
                line_vertices.push(RaytracerBvhLineVertex::from_vec3(
                    line.end,
                    color,
                    instance_index,
                ));
            }
        }

        let (bvh_line_buffer, bvh_line_vertex_count) = if line_vertices.is_empty() {
            (
                device.buffer().label("Raytracer BVH Line Buffer").vertex(&[
                    RaytracerBvhLineVertex::zero(),
                    RaytracerBvhLineVertex::zero(),
                ]),
                0,
            )
        } else {
            create_bvh_lines_vertex_buffer(device, &line_vertices)
        };

        let material_buffer = device
            .buffer()
            .label("Raytracer Materials Buffer")
            .storage(&materials);
        let vertices_buffer = device
            .buffer()
            .label("Raytracer Vertices Buffer")
            .storage(&vertices);
        let indices_buffer = device
            .buffer()
            .label("Raytracer Indices Buffer")
            .storage(&indices);

        let blas_nodes_buffer = if blas_nodes.is_empty() {
            device
                .buffer()
                .label("Raytracer BLAS Node Buffer")
                .storage(&[BvhNode::default()])
        } else {
            device
                .buffer()
                .label("Raytracer BLAS Node Buffer")
                .storage(&blas_nodes)
        };

        let blas_primitives_buffer = if blas_primitive_indices.is_empty() {
            device
                .buffer()
                .label("Raytracer BLAS Primitive Indices Buffer")
                .storage(&[0u32])
        } else {
            device
                .buffer()
                .label("Raytracer BLAS Primitive Indices Buffer")
                .storage(&blas_primitive_indices)
        };

        let blas_info_buffer = if blas_infos.is_empty() {
            device
                .buffer()
                .label("Raytracer BLAS Info Buffer")
                .storage(&[RaytracerBlasInfo::default()])
        } else {
            device
                .buffer()
                .label("Raytracer BLAS Info Buffer")
                .storage(&blas_infos)
        };

        let instances_buffer = if instances.is_empty() {
            device
                .buffer()
                .label("Raytracer TLAS Instance Buffer")
                .storage(&[RaytracerInstance::default()])
        } else {
            device
                .buffer()
                .label("Raytracer TLAS Instance Buffer")
                .storage(&instances)
        };

        Ok(RaytracerExtractedBuffers {
            materials: material_buffer,
            vertices: vertices_buffer,
            indices: indices_buffer,
            bvh_lines: bvh_line_buffer,
            bvh_line_vertex_count,
            blas_nodes: blas_nodes_buffer,
            blas_node_count: blas_nodes.len() as u32,
            blas_primitive_indices: blas_primitives_buffer,
            blas_primitive_count: blas_primitive_indices.len() as u32,
            blas_info: blas_info_buffer,
            tlas_nodes: tlas_nodes_buffer,
            tlas_node_count,
            tlas_primitive_indices: tlas_primitive_indices_buffer,
            tlas_primitive_count,
            instances: instances_buffer,
            instance_count: instances.len() as u32,
            tlas_bvh,
        })
    }
}

fn create_bvh_lines_vertex_buffer(
    device: &wgpu::Device,
    vertices: &[RaytracerBvhLineVertex],
) -> (wgpu::Buffer, u32) {
    let byte_len = std::mem::size_of_val(vertices) as u64;
    let max_size = device.limits().max_buffer_size;

    if byte_len > max_size {
        log::warn!(
            "Skipping BVH debug lines: buffer would be {:.2} MiB, exceeds device limit {:.2} MiB",
            byte_len as f64 / (1024.0 * 1024.0),
            max_size as f64 / (1024.0 * 1024.0)
        );

        let fallback = device.buffer().label("Raytracer BVH Line Buffer").vertex(&[
            RaytracerBvhLineVertex::zero(),
            RaytracerBvhLineVertex::zero(),
        ]);

        return (fallback, 0);
    }

    let buffer = device
        .buffer()
        .label("Raytracer BVH Line Buffer")
        .usage(wgpu::BufferUsages::COPY_DST)
        .vertex(vertices);

    (buffer, vertices.len() as u32)
}
