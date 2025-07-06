use std::mem::size_of;
use wesl::include_wesl;

// Rasterizer vertex type
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 4],
    pub normal: [f32; 4],
}

impl Vertex {
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
    rendering::raytracer::Raytracer,
    rendering::wgpu_utils::{CameraBuffers, LightingBuffers, WgpuResources},
    rendering::wgpu_utils::{WgpuExt, render_pass},
    // scene::EcsScene, // Removed - using World directly
};
use ecs::{EntityId, World};

pub struct Rasterizer {
    camera_buffers: CameraBuffers,
    lighting_buffers: LightingBuffers,
    other_bind_group: wgpu::BindGroup,
    depth_texture: crate::rendering::wgpu_utils::Texture,
    render_pipeline: wgpu::RenderPipeline,
    material_bind_groups: Vec<wgpu::BindGroup>,
    probe_grid: ProbeGrid,
    probe_visualization: ProbeVisualization,
    probe_updater: ProbeUpdatePipeline,
    lights_bind_group_layout: wgpu::BindGroupLayout,
}

impl Rasterizer {
    pub fn new(
        wgpu: &WgpuResources,
        scene: &crate::scene::Scene,
        world: &World,
        camera_entity: EntityId,
        sun_light_entity: EntityId,
    ) -> Self {
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
            .vertex_buffer(Vertex::desc())
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

        let camera_buffers = CameraBuffers::new(&wgpu.device, world, camera_entity, "Rasterizer");
        let lighting_buffers =
            LightingBuffers::new(&wgpu.device, world, sun_light_entity, "Rasterizer");
        println!("Creating rasterizer other bind group");
        let other_bind_group = wgpu
            .device
            .bind_group(&other_bind_group_layout)
            .label("Rasterizer Other Bind Group")
            .buffer(0, &camera_buffers.view_projection)
            .buffer(1, &lighting_buffers.sun_direction)
            .build();

        let material_bind_groups =
            scene.create_material_bind_groups(&wgpu.device, &material_bind_group_layout);

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
            probe_grid,
            probe_visualization,
            probe_updater,
            lights_bind_group_layout,
        }
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
        device: &wgpu::Device,
        render_encoder: &mut wgpu::CommandEncoder,
        surface_texture_view: &wgpu::TextureView,
        scene: &crate::scene::Scene,
        world: &World,
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

        // Query ECS for renderable entities
        let renderable_entities = world.get_entities_with_3::<crate::rendering::Transform, crate::mesh::Mesh, crate::rendering::MaterialRef>()
            .into_iter()
            .filter(|&entity_id| world.has_component::<crate::rendering::Renderable>(entity_id))
            .collect::<Vec<_>>();

        // No more hacky material entity mapping - use centralized Scene indexing

        // Render each ECS entity
        for entity_id in renderable_entities.iter() {
            // Get components
            if let (Some(mesh_component), Some(material_ref_component)) = (
                world.get_component::<crate::mesh::Mesh>(*entity_id),
                world.get_component::<crate::rendering::MaterialRef>(*entity_id),
            ) {
                let mesh = mesh_component.borrow();
                let material_ref = material_ref_component.borrow();

                // Use centralized Scene material indexing for consistent colors
                if let Some(material_index) =
                    scene.get_material_index_for_entity(material_ref.material_entity)
                {
                    if material_index < self.material_bind_groups.len() {
                        rasterizer_rpass.set_bind_group(
                            2,
                            &self.material_bind_groups[material_index],
                            &[],
                        );
                    }
                } else {
                    // Fallback to first material if mapping fails
                    if !self.material_bind_groups.is_empty() {
                        rasterizer_rpass.set_bind_group(2, &self.material_bind_groups[0], &[]);
                    }
                }

                // Convert mesh data to rasterizer vertex format
                let vertices: Vec<Vertex> = mesh
                    .vertices()
                    .iter()
                    .map(|v| Vertex {
                        position: [v.position[0], v.position[1], v.position[2], 1.0],
                        normal: [v.normal[0], v.normal[1], v.normal[2], 0.0],
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
                    .index(mesh.indices());

                rasterizer_rpass.set_vertex_buffer(0, vertex_buffer.slice(..));
                rasterizer_rpass
                    .set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                rasterizer_rpass.draw_indexed(0..mesh.indices().len() as u32, 0, 0..1);
            }
        }
    }

    /// Update probe positions based on grid configuration
    pub fn update_probes(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.probe_grid.recreate_textures_if_needed(device);

        // Texture views are now created dynamically during dispatch

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

    // Removed get_material_bind_groups - now using scene.create_material_bind_groups() directly
}

// Removed RasterizerMaterialBuffers and RasterizerOtherBuffers - now using unified buffer utilities

// Removed create_material_bind_group - now using ECS material bind groups
