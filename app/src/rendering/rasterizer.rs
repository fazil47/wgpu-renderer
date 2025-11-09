use std::{collections::HashMap, mem::size_of};
use wesl::include_wesl;
use wgpu::{BindGroup, BindGroupLayout, Buffer, Device, RenderPipeline};

use crate::{
    lighting::probe_lighting::{
        Dimensions, ProbeGrid, ProbeGridConfig,
        updater::ProbeUpdatePipeline,
        visualization::{ProbeUIResult, ProbeVisualization},
    },
    mesh::Vertex,
    rendering::{
        WorldExtractExt,
        extract::{Extract, ExtractionError},
        raytracer::Raytracer,
        wgpu::{CameraBuffers, LightingBuffers, WgpuExt, WgpuResources, render_pass},
    },
};
use ecs::{Entity, World};

pub struct GpuMesh {
    vertex_buffer: Buffer,
    vertex_count: u32,
    index_buffer: Option<(Buffer, u32)>,
    material_entity: Option<Entity>,
}

// Rasterizer vertex type
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuVertex {
    pub position: [f32; 4],
    pub normal: [f32; 4],
}

impl GpuVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

impl From<Vertex> for GpuVertex {
    fn from(val: Vertex) -> Self {
        GpuVertex {
            position: val.position.to_array(),
            normal: val.normal.to_array(),
        }
    }
}

pub struct Rasterizer {
    gpu_meshes: Vec<GpuMesh>,
    camera_buffers: CameraBuffers,
    lighting_buffers: LightingBuffers,
    depth_texture: crate::rendering::wgpu::Texture,
    material_bgl: BindGroupLayout,
    material_bind_groups: HashMap<Entity, BindGroup>,
    material_double_sided: HashMap<Entity, bool>,
    other_bind_group: BindGroup,
    render_pipeline: RenderPipeline,
    render_pipeline_double_sided: RenderPipeline,
    probe_grid: ProbeGrid,
    probe_updater: ProbeUpdatePipeline,
    probe_visualization: ProbeVisualization,
    probe_updater_light_bgl: BindGroupLayout,
}

impl Rasterizer {
    pub fn new(wgpu: &WgpuResources) -> Self {
        let rasterizer_shader = wgpu
            .device
            .shader()
            .label("Rasterizer Main Shader")
            .wesl(include_wesl!("rasterizer-main").into());

        let other_bind_group_layout = wgpu
            .device
            .bind_group_layout()
            .label("Rasterizer Other Bind Group Layout")
            .uniform(0, wgpu::ShaderStages::VERTEX)
            .uniform(1, wgpu::ShaderStages::FRAGMENT)
            .build();
        let material_bgl = wgpu
            .device
            .bind_group_layout()
            .label("Rasterizer Material Bind Group Layout")
            .uniform(0, wgpu::ShaderStages::FRAGMENT)
            .build();

        let probe_grid_config = ProbeGridConfig::default();
        let probe_grid = ProbeGrid::new(&wgpu.device, probe_grid_config);

        let render_pipeline_layout = wgpu
            .device
            .pipeline_layout()
            .label("Rasterizer Render Pipeline Layout")
            .bind_group_layouts(&[
                &other_bind_group_layout,
                probe_grid.bind_group_layout(),
                &material_bgl,
            ])
            .build();

        let swapchain_capabilities = wgpu.surface.get_capabilities(&wgpu.adapter);
        let swapchain_format = swapchain_capabilities.formats[0];

        let render_pipeline = wgpu
            .device
            .render_pipeline()
            .label("Rasterizer Render Pipeline")
            .layout(&render_pipeline_layout)
            .vertex_shader(&rasterizer_shader, "vs_main")
            .fragment_shader(&rasterizer_shader, "fs_main")
            .vertex_buffer(GpuVertex::desc())
            .color_target_alpha_blend(swapchain_format)
            .cull_mode(Some(wgpu::Face::Back))
            .depth_test_less(crate::rendering::wgpu::Texture::DEPTH_FORMAT)
            .build()
            .unwrap();

        let render_pipeline_double_sided = wgpu
            .device
            .render_pipeline()
            .label("Rasterizer Render Pipeline (Double Sided)")
            .layout(&render_pipeline_layout)
            .vertex_shader(&rasterizer_shader, "vs_main")
            .fragment_shader(&rasterizer_shader, "fs_main")
            .vertex_buffer(GpuVertex::desc())
            .color_target_alpha_blend(swapchain_format)
            .cull_mode(None)
            .depth_test_less(crate::rendering::wgpu::Texture::DEPTH_FORMAT)
            .build()
            .unwrap();

        let depth_texture = crate::rendering::wgpu::Texture::create_depth_texture(
            &wgpu.device,
            &wgpu.surface_config,
            "rasterizer_depth_texture",
        );

        let camera_buffers = CameraBuffers::new(&wgpu.device, "Rasterizer");
        let lighting_buffers = LightingBuffers::new(&wgpu.device, "Rasterizer");

        let other_bind_group = wgpu
            .device
            .bind_group(&other_bind_group_layout)
            .label("Rasterizer Other Bind Group")
            .buffer(0, &camera_buffers.view_projection)
            .buffer(1, &lighting_buffers.sun_direction)
            .build();

        let probe_visualization =
            ProbeVisualization::new(wgpu, &probe_grid, &camera_buffers.view_projection);

        let probe_updater_material_bgl = Raytracer::create_material_bind_group_layout(&wgpu.device);
        let probe_updater_mesh_bgl = Raytracer::create_mesh_bind_group_layout(&wgpu.device);
        let probe_updater_light_bgl = wgpu
            .device
            .bind_group_layout()
            .label("Lights Bind Group Layout")
            .uniform(0, wgpu::ShaderStages::COMPUTE)
            .build();

        let probe_updater = ProbeUpdatePipeline::new(
            &wgpu.device,
            &probe_updater_material_bgl,
            &probe_updater_mesh_bgl,
            &probe_updater_light_bgl,
        );

        Self {
            gpu_meshes: Vec::new(),
            camera_buffers,
            lighting_buffers,
            depth_texture,
            material_bgl,
            material_bind_groups: HashMap::new(),
            material_double_sided: HashMap::new(),
            other_bind_group,
            render_pipeline,
            render_pipeline_double_sided,
            probe_grid,
            probe_visualization,
            probe_updater,
            probe_updater_light_bgl,
        }
    }

    pub fn update_render_data(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        world: &World,
        camera_entity: Entity,
        sun_light_entity: Entity,
    ) -> Result<(), ExtractionError> {
        self.camera_buffers
            .update_from_world(queue, world, camera_entity);
        self.lighting_buffers
            .update_from_world(queue, world, sun_light_entity);
        let (gpu_meshes, material_bind_groups, material_double_sided) =
            self.extract(device, world)?;
        self.gpu_meshes = gpu_meshes;
        self.material_bind_groups = material_bind_groups;
        self.material_double_sided = material_double_sided;

        Ok(())
    }

    pub fn resize(&mut self, wgpu: &WgpuResources) {
        self.depth_texture = crate::rendering::wgpu::Texture::create_depth_texture(
            &wgpu.device,
            &wgpu.surface_config,
            "rasterizer_depth_texture",
        );
    }

    pub fn update_camera(&self, queue: &wgpu::Queue, world: &World, camera_entity: Entity) {
        self.camera_buffers
            .update_from_world(queue, world, camera_entity);
    }

    pub fn update_light(&self, queue: &wgpu::Queue, world: &World, sun_light_entity: Entity) {
        self.lighting_buffers
            .update_from_world(queue, world, sun_light_entity);
    }

    pub fn get_depth_texture_view(&self) -> &wgpu::TextureView {
        &self.depth_texture.view
    }

    pub fn render(
        &self,
        render_encoder: &mut wgpu::CommandEncoder,
        surface_texture_view: &wgpu::TextureView,
        default_material_entity: Entity,
    ) {
        let mut rpass = render_pass(render_encoder)
            .label("Rasterizer Render Pass")
            .color_attachment(surface_texture_view, Some(wgpu::Color::BLACK))
            .depth_attachment(&self.depth_texture.view, Some(1.0))
            .begin();

        rpass.set_bind_group(0, &self.other_bind_group, &[]);
        rpass.set_bind_group(1, self.probe_grid.bind_group(), &[]);

        for mesh in &self.gpu_meshes {
            let material_entity = mesh.material_entity.unwrap_or(default_material_entity);
            let material_bind_group = self
                .material_bind_groups
                .get(&material_entity)
                .or_else(|| self.material_bind_groups.get(&default_material_entity))
                .expect("Material bind group not found");
            let double_sided = self
                .material_double_sided
                .get(&material_entity)
                .copied()
                .or_else(|| {
                    self.material_double_sided
                        .get(&default_material_entity)
                        .copied()
                })
                .unwrap_or(false);

            if double_sided {
                rpass.set_pipeline(&self.render_pipeline_double_sided);
            } else {
                rpass.set_pipeline(&self.render_pipeline);
            }

            rpass.set_bind_group(2, material_bind_group, &[]);
            rpass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));

            if let Some((buffer, count)) = mesh.index_buffer.as_ref() {
                rpass.set_index_buffer(buffer.slice(..), wgpu::IndexFormat::Uint32);
                rpass.draw_indexed(0..*count, 0, 0..1);
            } else {
                rpass.draw(0..mesh.vertex_count, 0..1);
            }
        }
    }

    pub fn update_probes(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.probe_grid.recreate_textures_if_needed(device);

        self.probe_grid.update_config_buffer(queue);
        self.probe_visualization
            .update_probes(device, queue, &self.probe_grid);
    }

    pub fn render_probe_visualization(
        &self,
        render_encoder: &mut wgpu::CommandEncoder,
        surface_texture_view: &wgpu::TextureView,
    ) {
        self.probe_visualization.render(
            render_encoder,
            surface_texture_view,
            &self.depth_texture.view,
            self.probe_grid.bind_group(),
        );
    }

    // EGUI setup for probes
    pub fn run_probe_ui(&mut self, ui: &mut egui::Ui) -> ProbeUIResult {
        self.probe_visualization.run_ui(ui, &mut self.probe_grid)
    }

    pub fn is_probe_dirty(&self) -> bool {
        self.probe_grid.is_dirty()
    }

    pub fn clear_probe_dirty(&mut self) {
        self.probe_grid.clear_dirty();
    }

    pub fn should_render_probe_visualization(&self) -> bool {
        self.probe_visualization.show_probes
    }

    pub fn get_camera_buffer(&self) -> &wgpu::Buffer {
        &self.camera_buffers.view_projection
    }

    pub fn get_sun_direction_buffer(&self) -> &wgpu::Buffer {
        &self.lighting_buffers.sun_direction
    }

    pub fn bake_probes(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        material_bind_group: &wgpu::BindGroup,
        mesh_bind_group: &wgpu::BindGroup,
    ) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Probe Baking Command Encoder"),
        });

        let lights_bind_group = device
            .bind_group(&self.probe_updater_light_bgl)
            .label("Probe Update Lights Bind Group")
            .buffer(0, &self.lighting_buffers.sun_direction)
            .build();

        let probe_bind_group = device
            .bind_group(&self.probe_updater.probe_bind_group_layout)
            .label("Probe Update Probe Bind Group")
            .texture(0, self.probe_grid.get_l0_brick_atlas_view())
            .texture(1, self.probe_grid.get_l1x_brick_atlas_view())
            .texture(2, self.probe_grid.get_l1y_brick_atlas_view())
            .texture(3, self.probe_grid.get_l1z_brick_atlas_view())
            .buffer(4, self.probe_grid.get_config_buffer())
            .build();

        let Dimensions { x, y, z } = self.probe_grid.config.dimensions;
        let probe_count = x * y * z;

        self.probe_updater.dispatch(
            &mut encoder,
            material_bind_group,
            mesh_bind_group,
            &lights_bind_group,
            &probe_bind_group,
            probe_count,
        );

        queue.submit(Some(encoder.finish()));
    }
}

impl Extract for Rasterizer {
    type ExtractedData = (
        Vec<GpuMesh>,
        HashMap<Entity, BindGroup>,
        HashMap<Entity, bool>,
    );

    fn extract(
        &self,
        device: &Device,
        world: &World,
    ) -> Result<Self::ExtractedData, ExtractionError> {
        let materials: Vec<Entity> = world.get_materials();
        let mut material_bind_groups: HashMap<Entity, BindGroup> = HashMap::new();
        let mut material_double_sided: HashMap<Entity, bool> = HashMap::new();

        for entity in materials {
            let material = world.extract_material_component(entity)?;
            let color_array = material.color.to_array();

            let material_buffer = device
                .buffer()
                .label("Material Buffer")
                .usage(wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST)
                .uniform(&color_array);
            let material_bind_group = device
                .bind_group(&self.material_bgl)
                .label("Material Bind Group")
                .buffer(0, &material_buffer)
                .build();

            material_bind_groups
                .entry(entity)
                .or_insert(material_bind_group);
            material_double_sided
                .entry(entity)
                .or_insert(material.double_sided);
        }

        let renderables: Vec<Entity> = world.get_renderables();
        let mut gpu_meshes: Vec<GpuMesh> = Vec::new();

        for entity in renderables {
            let transform = world.extract_transform_component(entity)?;
            let mesh = world.extract_mesh_component(entity)?;

            let vertices: Vec<GpuVertex> = mesh
                .vertices()
                .iter()
                .map(|v| {
                    let transform = transform.get_matrix();
                    let position = transform * v.position;
                    let normal = transform * v.normal;

                    GpuVertex {
                        position: position.to_array(),
                        normal: normal.to_array(),
                    }
                })
                .collect();

            let vertex_buffer: Buffer = device
                .buffer()
                .label("Entity Mesh Vertex Buffer")
                .usage(wgpu::BufferUsages::VERTEX)
                .vertex(&vertices);
            let vertex_count = mesh.vertices().len() as u32;

            let index_buffer = mesh.indices().map(|mesh_indices| {
                (
                    device
                        .buffer()
                        .label("Entity Mesh Index Buffer")
                        .usage(wgpu::BufferUsages::INDEX)
                        .index(mesh_indices),
                    mesh_indices.len() as u32,
                )
            });

            gpu_meshes.push(GpuMesh {
                vertex_buffer,
                vertex_count,
                index_buffer,
                material_entity: mesh.material_entity,
            });
        }

        Ok((gpu_meshes, material_bind_groups, material_double_sided))
    }
}
