use wesl::include_wesl;

use crate::{
    mesh::Material,
    probe_lighting::{
        Dimensions, ProbeGrid, ProbeGridConfig,
        updater::ProbeUpdatePipeline,
        visualization::{ProbeUIResult, ProbeVisualization},
    },
    raytracer::Raytracer,
    scene::Scene,
    wgpu::{RendererWgpu, Vertex},
    wgpu_utils::{QueueExt, WgpuExt, render_pass},
};

pub struct Rasterizer {
    other_buffers: RasterizerOtherBuffers,
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
    pub fn new(wgpu: &RendererWgpu, scene: &Scene) -> Self {
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
            .depth_test_less(crate::wgpu::Texture::DEPTH_FORMAT)
            .build()
            .unwrap();

        let depth_texture = crate::wgpu::Texture::create_depth_texture(
            &wgpu.device,
            &wgpu.surface_config,
            "rasterizer_depth_texture",
        );

        let other_buffers = RasterizerOtherBuffers::new(&wgpu.device, scene);
        println!("Creating rasterizer other bind group");
        let other_bind_group = wgpu
            .device
            .bind_group(&other_bind_group_layout)
            .label("Rasterizer Other Bind Group")
            .buffer(0, &other_buffers.camera_view_proj)
            .buffer(1, &other_buffers.sun_direction)
            .build();

        let material_bind_groups =
            Rasterizer::get_material_bind_groups(&wgpu.device, &material_bind_group_layout, scene);

        let probe_visualization =
            ProbeVisualization::new(wgpu, &probe_grid, &other_buffers.camera_view_proj);

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
            other_buffers,
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

    pub fn get_depth_texture_view(&self) -> &wgpu::TextureView {
        &self.depth_texture.view
    }

    pub fn render(
        &self,
        device: &wgpu::Device,
        render_encoder: &mut wgpu::CommandEncoder,
        surface_texture_view: &wgpu::TextureView,
        materials: &[Material],
    ) {
        let mut rasterizer_rpass = render_pass(render_encoder)
            .label("Rasterizer Render Pass")
            .color_attachment(surface_texture_view, Some(wgpu::Color::BLACK))
            .depth_attachment(&self.depth_texture.view, Some(1.0))
            .begin();

        rasterizer_rpass.set_pipeline(&self.render_pipeline);
        rasterizer_rpass.set_bind_group(0, &self.other_bind_group, &[]);
        rasterizer_rpass.set_bind_group(1, self.probe_grid.bind_group(), &[]);

        for (i, material) in materials.iter().enumerate() {
            rasterizer_rpass.set_bind_group(2, &self.material_bind_groups[i], &[]);

            let vertex_buffer = material.create_vertex_buffer(device);
            let index_buffer = material.create_index_buffer(device);
            let num_indices = material.get_index_count();

            rasterizer_rpass.set_vertex_buffer(0, vertex_buffer.slice(..));
            rasterizer_rpass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            rasterizer_rpass.draw_indexed(0..num_indices, 0, 0..1);
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
        &self.other_buffers.camera_view_proj
    }

    pub fn get_sun_direction_buffer(&self) -> &wgpu::Buffer {
        &self.other_buffers.sun_direction
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
            .buffer(0, &self.other_buffers.sun_direction)
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

    fn get_material_bind_groups(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        scene: &Scene,
    ) -> Vec<wgpu::BindGroup> {
        scene
            .materials
            .iter()
            .map(|material| create_material_bind_group(device, bind_group_layout, material))
            .collect()
    }
}

struct RasterizerMaterialBuffers {
    color: wgpu::Buffer,
}

impl RasterizerMaterialBuffers {
    fn new(device: &wgpu::Device, material: &Material) -> Self {
        let color = device
            .buffer()
            .label("Rasterizer Material Color Buffer")
            .uniform(&material.color.to_array());
        Self { color }
    }
}

struct RasterizerOtherBuffers {
    camera_view_proj: wgpu::Buffer,
    sun_direction: wgpu::Buffer,
}

impl RasterizerOtherBuffers {
    fn new(device: &wgpu::Device, scene: &Scene) -> Self {
        let camera_view_proj = device
            .buffer()
            .label("Rasterizer Camera View Projection Uniform Buffer")
            .uniform(&scene.camera.view_projection().to_cols_array_2d());
        let sun_direction = device
            .buffer()
            .label("Rasterizer Sun Direction Uniform Buffer")
            .uniform(&scene.sun_light.direction.to_array());

        Self {
            camera_view_proj,
            sun_direction,
        }
    }

    fn update_camera(&self, queue: &wgpu::Queue, scene: &Scene) {
        queue.write_buffer_data(
            &self.camera_view_proj,
            0,
            &scene.camera.view_projection().to_cols_array_2d(),
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

fn create_material_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    material: &Material,
) -> wgpu::BindGroup {
    let buffers = RasterizerMaterialBuffers::new(device, material);
    device
        .bind_group(layout)
        .label("Rasterizer Material Bind Group")
        .buffer(0, &buffers.color)
        .build()
}
