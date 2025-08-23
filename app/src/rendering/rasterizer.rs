use std::mem::size_of;
use wesl::include_wesl;

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

use crate::{
    lighting::probe_lighting::{
        Dimensions, ProbeGrid, ProbeGridConfig,
        updater::ProbeUpdatePipeline,
        visualization::{ProbeUIResult, ProbeVisualization},
    },
    rendering::{
        extract::{
            Extract, ExtractionError, RenderableEntity, extract_entity_components,
            query_renderable_entities,
        },
        raytracer::Raytracer,
        wgpu_utils::{CameraBuffers, LightingBuffers, WgpuExt, WgpuResources, render_pass},
    },
    scene::Scene,
};
use ecs::{EntityId, World};

struct EntityRenderData {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    material_index: usize,
}

pub struct Rasterizer {
    camera_buffers: CameraBuffers,
    lighting_buffers: LightingBuffers,
    other_bind_group: wgpu::BindGroup,
    depth_texture: crate::rendering::wgpu_utils::Texture,
    render_pipeline: wgpu::RenderPipeline,
    material_bind_groups: Vec<wgpu::BindGroup>,
    entity_render_data: Vec<EntityRenderData>,
    probe_grid: ProbeGrid,
    probe_visualization: ProbeVisualization,
    probe_updater: ProbeUpdatePipeline,
    lights_bind_group_layout: wgpu::BindGroupLayout,
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
        let material_bind_group_layout = wgpu
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
                &material_bind_group_layout,
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
            .depth_test_less(crate::rendering::wgpu_utils::Texture::DEPTH_FORMAT)
            .build()
            .unwrap();

        let depth_texture = crate::rendering::wgpu_utils::Texture::create_depth_texture(
            &wgpu.device,
            &wgpu.surface_config,
            "rasterizer_depth_texture",
        );

        let camera_buffers = CameraBuffers::new(&wgpu.device, "Rasterizer");
        let lighting_buffers = LightingBuffers::new(&wgpu.device, "Rasterizer");

        println!("Creating rasterizer other bind group");
        let other_bind_group = wgpu
            .device
            .bind_group(&other_bind_group_layout)
            .label("Rasterizer Other Bind Group")
            .buffer(0, &camera_buffers.view_projection)
            .buffer(1, &lighting_buffers.sun_direction)
            .build();

        let material_bind_groups = Vec::new(); // Empty until scene data loaded

        let dummy_vertices = vec![GpuVertex {
            position: [0.0, 0.0, 0.0, 1.0],
            normal: [0.0, 1.0, 0.0, 0.0],
        }];
        let dummy_indices = vec![0u32];

        let dummy_vertex_buffer = wgpu
            .device
            .buffer()
            .label("Dummy Entity Vertex Buffer")
            .usage(wgpu::BufferUsages::VERTEX)
            .vertex(&dummy_vertices);

        let dummy_index_buffer = wgpu
            .device
            .buffer()
            .label("Dummy Entity Index Buffer")
            .usage(wgpu::BufferUsages::INDEX)
            .index(&dummy_indices);

        let entity_render_data = vec![EntityRenderData {
            vertex_buffer: dummy_vertex_buffer,
            index_buffer: dummy_index_buffer,
            index_count: dummy_indices.len() as u32,
            material_index: 0,
        }];

        let probe_visualization =
            ProbeVisualization::new(wgpu, &probe_grid, &camera_buffers.view_projection);

        let material_bind_group_layout = Raytracer::create_material_bind_group_layout(&wgpu.device);
        let mesh_bind_group_layout = Raytracer::create_mesh_bind_group_layout(&wgpu.device);
        let lights_bind_group_layout = wgpu
            .device
            .bind_group_layout()
            .label("Lights Bind Group Layout")
            .uniform(0, wgpu::ShaderStages::COMPUTE)
            .build();

        let probe_updater = ProbeUpdatePipeline::new(
            &wgpu.device,
            &material_bind_group_layout,
            &mesh_bind_group_layout,
            &lights_bind_group_layout,
        );

        Self {
            camera_buffers,
            lighting_buffers,
            other_bind_group,
            depth_texture,
            render_pipeline,
            material_bind_groups,
            entity_render_data,
            probe_grid,
            probe_visualization,
            probe_updater,
            lights_bind_group_layout,
        }
    }

    pub fn update_scene_data(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        world: &World,
        camera_entity: EntityId,
        sun_light_entity: EntityId,
    ) -> Result<(), ExtractionError> {
        let scene = world.get_resource::<Scene>().unwrap();

        // Update camera and lighting buffers
        self.camera_buffers
            .update_from_world(queue, world, camera_entity);
        self.lighting_buffers
            .update_from_world(queue, world, sun_light_entity);

        // Create material bind groups
        let material_bind_group_layout = device
            .bind_group_layout()
            .label("Rasterizer Material Bind Group Layout")
            .uniform(0, wgpu::ShaderStages::FRAGMENT)
            .build();
        self.material_bind_groups =
            scene.create_material_bind_groups(device, &material_bind_group_layout);

        let extracted_entities = self.extract(world)?;
        let mut entity_render_data = Vec::new();

        for renderable_entity in extracted_entities {
            // Convert mesh data to rasterizer vertex format
            let vertices: Vec<GpuVertex> = renderable_entity
                .mesh
                .vertices()
                .iter()
                .map(|v| {
                    let transform = renderable_entity.transform.get_matrix();
                    let position = transform * v.position;
                    let normal = transform * v.normal;
                    GpuVertex {
                        position: position.to_array(),
                        normal: normal.to_array(),
                    }
                })
                .collect();

            // Create vertex and index buffers for this entity's mesh
            let vertex_buffer = device
                .buffer()
                .label("Entity Mesh Vertex Buffer")
                .usage(wgpu::BufferUsages::VERTEX)
                .vertex(&vertices);

            let index_buffer = device
                .buffer()
                .label("Entity Mesh Index Buffer")
                .usage(wgpu::BufferUsages::INDEX)
                .index(renderable_entity.mesh.indices());

            entity_render_data.push(EntityRenderData {
                vertex_buffer,
                index_buffer,
                index_count: renderable_entity.mesh.indices().len() as u32,
                material_index: renderable_entity.material_index,
            });
        }

        self.entity_render_data = entity_render_data;
        Ok(())
    }

    pub fn resize(&mut self, wgpu: &WgpuResources) {
        self.depth_texture = crate::rendering::wgpu_utils::Texture::create_depth_texture(
            &wgpu.device,
            &wgpu.surface_config,
            "rasterizer_depth_texture",
        );
    }

    pub fn update_camera(&self, queue: &wgpu::Queue, world: &World, camera_entity: EntityId) {
        self.camera_buffers
            .update_from_world(queue, world, camera_entity);
    }

    pub fn update_light(&self, queue: &wgpu::Queue, world: &World, sun_light_entity: EntityId) {
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
        _camera_entity: EntityId,
    ) {
        let mut rasterizer_rpass = render_pass(render_encoder)
            .label("Rasterizer Render Pass")
            .color_attachment(surface_texture_view, Some(wgpu::Color::BLACK))
            .depth_attachment(&self.depth_texture.view, Some(1.0))
            .begin();

        rasterizer_rpass.set_pipeline(&self.render_pipeline);
        rasterizer_rpass.set_bind_group(0, &self.other_bind_group, &[]);
        rasterizer_rpass.set_bind_group(1, self.probe_grid.bind_group(), &[]);

        // Render each cached entity
        for entity_data in &self.entity_render_data {
            // Set material bind group using pre-calculated material index
            if entity_data.material_index < self.material_bind_groups.len() {
                rasterizer_rpass.set_bind_group(
                    2,
                    &self.material_bind_groups[entity_data.material_index],
                    &[],
                );
            } else {
                // Fallback to first material if index is invalid
                if !self.material_bind_groups.is_empty() {
                    rasterizer_rpass.set_bind_group(2, &self.material_bind_groups[0], &[]);
                }
            }

            // Use pre-created buffers
            rasterizer_rpass.set_vertex_buffer(0, entity_data.vertex_buffer.slice(..));
            rasterizer_rpass.set_index_buffer(
                entity_data.index_buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            rasterizer_rpass.draw_indexed(0..entity_data.index_count, 0, 0..1);
        }
    }

    /// Update probe positions based on grid configuration
    pub fn update_probes(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.probe_grid.recreate_textures_if_needed(device);

        self.probe_grid.update_config_buffer(queue);
        self.probe_visualization
            .update_probes(device, queue, &self.probe_grid);
    }

    /// Render probe visualization spheres
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

    /// Run UI for probe grid configuration and visualization
    pub fn run_probe_ui(&mut self, ui: &mut egui::Ui) -> ProbeUIResult {
        self.probe_visualization.run_ui(ui, &mut self.probe_grid)
    }

    /// Check if probe grid is dirty and needs updates
    pub fn is_probe_dirty(&self) -> bool {
        self.probe_grid.is_dirty()
    }

    /// Clear probe dirty flag
    pub fn clear_probe_dirty(&mut self) {
        self.probe_grid.clear_dirty();
    }

    /// Check if probe visualization should be rendered
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

        // Create lights bind group
        let lights_bind_group = device
            .bind_group(&self.lights_bind_group_layout)
            .label("Probe Update Lights Bind Group")
            .buffer(0, &self.lighting_buffers.sun_direction)
            .build();

        // Create probe bind group with textures and config
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
    type ExtractedData = Vec<RenderableEntity>;

    fn extract(&self, world: &World) -> Result<Self::ExtractedData, ExtractionError> {
        let scene = world.get_resource::<Scene>().unwrap();

        let entity_ids = query_renderable_entities(world);
        let mut extracted_entities = Vec::new();

        for entity_id in entity_ids {
            let (transform, mesh, material_entity) = extract_entity_components(world, entity_id)?;

            // Get material index from scene
            let material_index = scene
                .get_material_index_for_entity(material_entity)
                .ok_or(ExtractionError::InvalidMaterialReference(entity_id))?;

            extracted_entities.push(RenderableEntity {
                entity_id,
                transform,
                mesh,
                material_entity,
                material_index,
            });
        }

        Ok(extracted_entities)
    }
}
