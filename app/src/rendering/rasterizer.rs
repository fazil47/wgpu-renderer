use wesl::include_wesl;

use crate::{
    lighting::probe_lighting::{
        Dimensions, ProbeGrid, ProbeGridConfig,
        updater::ProbeUpdatePipeline,
        visualization::{ProbeUIResult, ProbeVisualization},
    },
    rendering::raytracer::Raytracer,
    ecs::EcsScene,
    utils::buffer_utils::{CameraBuffers, LightingBuffers, create_vertex_buffer, create_index_buffer},
    rendering::wgpu::{RendererWgpu, Vertex},
    wgpu_utils::{WgpuExt, render_pass},
};

pub struct Rasterizer {
    camera_buffers: CameraBuffers,
    lighting_buffers: LightingBuffers,
    other_bind_group: wgpu::BindGroup,
    depth_texture: crate::wgpu::Texture,
    render_pipeline: wgpu::RenderPipeline,
    material_bind_groups: Vec<wgpu::BindGroup>,
    probe_grid: ProbeGrid,
    probe_visualization: ProbeVisualization,
    probe_updater: ProbeUpdatePipeline,
    lights_bind_group_layout: wgpu::BindGroupLayout,
    config_sun_bind_group_layout: wgpu::BindGroupLayout,
}

impl Rasterizer {
    pub fn new(wgpu: &RendererWgpu, scene: &EcsScene) -> Self {
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
            .depth_test_less(crate::rendering::wgpu::Texture::DEPTH_FORMAT)
            .build()
            .unwrap();

        let depth_texture = crate::rendering::wgpu::Texture::create_depth_texture(
            &wgpu.device,
            &wgpu.surface_config,
            "rasterizer_depth_texture",
        );

        let camera_buffers = CameraBuffers::new(&wgpu.device, scene, "Rasterizer");
        let lighting_buffers = LightingBuffers::new(&wgpu.device, scene, "Rasterizer");
        println!("Creating rasterizer other bind group");
        let other_bind_group = wgpu
            .device
            .bind_group(&other_bind_group_layout)
            .label("Rasterizer Other Bind Group")
            .buffer(0, &camera_buffers.view_projection)
            .buffer(1, &lighting_buffers.sun_direction)
            .build();

        let material_bind_groups = scene.create_material_bind_groups(&wgpu.device, &material_bind_group_layout);

        let probe_visualization =
            ProbeVisualization::new(wgpu, &probe_grid, &camera_buffers.view_projection);

        let config_sun_bind_group_layout = wgpu
            .device
            .bind_group_layout()
            .label("Probe Update Config Sun Bind Group Layout")
            .uniform(0, wgpu::ShaderStages::COMPUTE)
            .uniform(1, wgpu::ShaderStages::COMPUTE)
            .build();

        let material_bind_group_layout = Raytracer::create_material_bind_group_layout(&wgpu.device);
        let mesh_bind_group_layout = Raytracer::create_mesh_bind_group_layout(&wgpu.device);
        let lights_bind_group_layout = wgpu.device
            .bind_group_layout()
            .label("Lights Bind Group Layout")
            .uniform(0, wgpu::ShaderStages::COMPUTE)
            .build();

        let probe_updater = ProbeUpdatePipeline::new(
            &wgpu.device,
            probe_grid.get_l0_brick_atlas_view(),
            probe_grid.get_l1x_brick_atlas_view(),
            probe_grid.get_l1y_brick_atlas_view(),
            probe_grid.get_l1z_brick_atlas_view(),
            &material_bind_group_layout,
            &mesh_bind_group_layout,
            &lights_bind_group_layout,
            &config_sun_bind_group_layout,
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
            config_sun_bind_group_layout,
        }
    }

    pub fn resize(&mut self, wgpu: &RendererWgpu) {
        self.depth_texture = crate::rendering::wgpu::Texture::create_depth_texture(
            &wgpu.device,
            &wgpu.surface_config,
            "rasterizer_depth_texture",
        );
    }

    pub fn update_camera(&self, queue: &wgpu::Queue, scene: &EcsScene) {
        self.camera_buffers.update_from_scene(queue, scene);
    }

    pub fn update_light(&self, queue: &wgpu::Queue, scene: &EcsScene) {
        self.lighting_buffers.update_from_scene(queue, scene);
    }

    pub fn get_depth_texture_view(&self) -> &wgpu::TextureView {
        &self.depth_texture.view
    }

    pub fn render(
        &self,
        device: &wgpu::Device,
        render_encoder: &mut wgpu::CommandEncoder,
        surface_texture_view: &wgpu::TextureView,
        scene: &EcsScene,
    ) {
        // Get renderable entities for direct ECS rendering
        let renderable_entities = scene.get_renderable_entities();
        
        // Create a mapping from material entity to material bind group index
        let mut material_entities = scene.get_material_entities();
        material_entities.sort_by_key(|entity| entity.0); // Same ordering as create_material_bind_groups
        let material_entity_to_index: std::collections::HashMap<_, _> = 
            material_entities.iter().enumerate().map(|(i, &entity)| (entity, i)).collect();

        let mut rasterizer_rpass = render_pass(render_encoder)
            .label("Rasterizer Render Pass")
            .color_attachment(surface_texture_view, Some(wgpu::Color::BLACK))
            .depth_attachment(&self.depth_texture.view, Some(1.0))
            .begin();

        rasterizer_rpass.set_pipeline(&self.render_pipeline);
        rasterizer_rpass.set_bind_group(0, &self.other_bind_group, &[]);
        rasterizer_rpass.set_bind_group(1, self.probe_grid.bind_group(), &[]);

        // Group renderable entities by material for efficient rendering
        let mut material_to_entities: std::collections::HashMap<crate::ecs::EntityId, Vec<crate::ecs::EntityId>> = std::collections::HashMap::new();
        
        for entity_id in renderable_entities {
            if let Some(material_ref) = scene.world.get_component::<crate::ecs::MaterialRef>(entity_id) {
                let mat_ref = material_ref.borrow();
                material_to_entities.entry(mat_ref.material_entity)
                    .or_insert_with(Vec::new)
                    .push(entity_id);
            }
        }

        // Render each material group
        for (material_entity, entity_ids) in material_to_entities {
            // Set the appropriate material bind group
            if let Some(&bind_group_index) = material_entity_to_index.get(&material_entity) {
                rasterizer_rpass.set_bind_group(2, &self.material_bind_groups[bind_group_index], &[]);
                
                // Render each mesh with this material
                for entity_id in entity_ids {
                    if let Some(mesh_comp) = scene.world.get_component::<crate::ecs::MeshComponent>(entity_id) {
                        let mesh = mesh_comp.borrow();
                        
                        // Create vertex and index buffers for this specific mesh
                        let vertex_buffer = create_vertex_buffer(
                            device,
                            "Rasterizer Mesh Vertex Buffer",
                            &mesh.vertices,
                        );
                        let index_buffer = create_index_buffer(
                            device,
                            "Rasterizer Mesh Index Buffer",
                            &mesh.indices,
                        );
                        
                        rasterizer_rpass.set_vertex_buffer(0, vertex_buffer.slice(..));
                        rasterizer_rpass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                        rasterizer_rpass.draw_indexed(0..mesh.indices.len() as u32, 0, 0..1);
                    }
                }
            }
        }
    }

    /// Update probe positions based on grid configuration
    pub fn update_probes(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let textures_recreated = self.probe_grid.recreate_textures_if_needed(device);

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
