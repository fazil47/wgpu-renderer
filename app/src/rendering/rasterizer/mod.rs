mod blit_to_screen;
pub mod shadow;

use std::collections::HashMap;
use wgpu::{BindGroup, BindGroupLayout, Device, RenderPipeline};

use crate::{
    lighting::{
        DirectionalLight,
        directional_light::CASCADED_SHADOW_BLEND_REGION,
        probe_lighting::{
            Dimensions, ProbeGrid, ProbeGridConfig,
            updater::ProbeUpdatePipeline,
            visualization::{ProbeUIResult, ProbeVisualization},
        },
    },
    rendering::{
        GpuVertex, WorldExtractExt,
        extract::{Extract, ExtractionError},
        mesh::{InstanceTransform, MeshBuffers},
        rasterizer::{blit_to_screen::BlitToScreen, shadow::ShadowRenderTexture},
        raytracer::Raytracer,
        wgpu::{CameraBuffers, LightingBuffers, WgpuExt, WgpuResources, render_pass},
    },
};
use ecs::{Entity, World};

pub struct Rasterizer {
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
    shadow_render_texture: ShadowRenderTexture,
    blit_to_screen: BlitToScreen, // For debugging render textures
    debug_shadow_maps: bool,
}

impl ecs::Resource for Rasterizer {}

impl Rasterizer {
    pub fn new(wgpu: &WgpuResources) -> Self {
        let rasterizer_shader = wgpu
            .device
            .shader()
            .label("Rasterizer Main Shader")
            .define_u32("SHADOW_MAP_SIZE", shadow::SHADOW_MAP_SIZE)
            .define_f32("CASCADED_SHADOW_BLEND_REGION", CASCADED_SHADOW_BLEND_REGION)
            .wesl_runtime("package::rasterizer::main");

        let other_bind_group_layout = wgpu
            .device
            .bind_group_layout()
            .label("Rasterizer Other Bind Group Layout")
            .uniform(0, wgpu::ShaderStages::VERTEX)
            .uniform(1, wgpu::ShaderStages::FRAGMENT)
            .comparison_sampler(2, wgpu::ShaderStages::FRAGMENT)
            .texture_depth_2d_array(3, wgpu::ShaderStages::FRAGMENT)
            .uniform(4, wgpu::ShaderStages::FRAGMENT)
            .build();
        let material_bgl = wgpu
            .device
            .bind_group_layout()
            .label("Rasterizer Material Bind Group Layout")
            .uniform(0, wgpu::ShaderStages::FRAGMENT)
            .uniform(1, wgpu::ShaderStages::FRAGMENT)
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

        let swapchain_format = wgpu.target.format();

        let render_pipeline = wgpu
            .device
            .render_pipeline()
            .label("Rasterizer Render Pipeline")
            .layout(&render_pipeline_layout)
            .vertex_shader(&rasterizer_shader, "vs_main")
            .fragment_shader(&rasterizer_shader, "fs_main")
            .vertex_buffer(GpuVertex::desc())
            .vertex_buffer(InstanceTransform::desc())
            .color_target_alpha_blend(swapchain_format)
            .cull_mode(Some(wgpu::Face::Back))
            .depth_test_greater(crate::rendering::wgpu::Texture::DEPTH_FORMAT)
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
            .vertex_buffer(InstanceTransform::desc())
            .color_target_alpha_blend(swapchain_format)
            .cull_mode(None)
            .depth_test_greater(crate::rendering::wgpu::Texture::DEPTH_FORMAT)
            .build()
            .unwrap();

        let depth_texture = crate::rendering::wgpu::Texture::create_depth_texture(
            &wgpu.device,
            wgpu.target.width(),
            wgpu.target.height(),
            "rasterizer_depth_texture",
        );

        let camera_buffers = CameraBuffers::new(&wgpu.device, "Rasterizer");
        let lighting_buffers = LightingBuffers::new(&wgpu.device, "Rasterizer");
        let shadow_render_texture = ShadowRenderTexture::new(&wgpu.device);

        let other_bind_group = wgpu
            .device
            .bind_group(&other_bind_group_layout)
            .label("Rasterizer Other Bind Group")
            .buffer(0, &camera_buffers.view_projection)
            .buffer(1, &lighting_buffers.sun_direction)
            .sampler(2, shadow_render_texture.get_sampler())
            .texture(3, shadow_render_texture.get_shadow_map_array_view())
            .buffer(4, shadow_render_texture.get_packed_light_matrices_buffer())
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

        let cascade_to_debug = 0;
        let mut blit_to_screen = BlitToScreen::new(&wgpu.device, swapchain_format);
        blit_to_screen.set_texture_view(
            &wgpu.device,
            &shadow_render_texture.get_shadow_map_layer_views()[cascade_to_debug],
            true,
        );

        Self {
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
            shadow_render_texture,
            blit_to_screen,
            debug_shadow_maps: false,
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
        let (material_bind_groups, material_double_sided) = self.extract(device, world)?;
        self.material_bind_groups = material_bind_groups;
        self.material_double_sided = material_double_sided;

        Ok(())
    }

    pub fn resize(&mut self, wgpu: &WgpuResources) {
        self.depth_texture = crate::rendering::wgpu::Texture::create_depth_texture(
            &wgpu.device,
            wgpu.target.width(),
            wgpu.target.height(),
            "rasterizer_depth_texture",
        );
    }

    pub fn update_camera(&self, queue: &wgpu::Queue, world: &World, camera_entity: Entity) {
        self.camera_buffers
            .update_from_world(queue, world, camera_entity);
    }

    pub fn update_light(&mut self, queue: &wgpu::Queue, world: &World, sun_light_entity: Entity) {
        self.lighting_buffers
            .update_from_world(queue, world, sun_light_entity);

        if let Some(light) = world.get_component::<DirectionalLight>(sun_light_entity) {
            self.shadow_render_texture
                .update_light(queue, world, &light);
        }
    }

    pub fn get_depth_texture_view(&self) -> &wgpu::TextureView {
        &self.depth_texture.view
    }

    pub fn update_debug_config(
        &mut self,
        device: &wgpu::Device,
        debug_shadow_maps: bool,
        shadow_map_cascade_to_debug: usize,
    ) {
        self.blit_to_screen.set_texture_view(
            device,
            &self.shadow_render_texture.get_shadow_map_layer_views()[shadow_map_cascade_to_debug],
            true,
        );
        self.debug_shadow_maps = debug_shadow_maps;
    }

    pub fn render(
        &self,
        render_encoder: &mut wgpu::CommandEncoder,
        render_target_view: &wgpu::TextureView,
        default_material_entity: Entity,
        mesh_buffers: &MeshBuffers,
    ) {
        // TODO: Only render shadow map if the directional light has changed
        self.shadow_render_texture
            .render(render_encoder, mesh_buffers);

        if self.debug_shadow_maps {
            self.blit_to_screen
                .render(render_encoder, render_target_view);
            return;
        }

        let mut rpass = render_pass(render_encoder)
            .label("Rasterizer Render Pass")
            .color_attachment(render_target_view, Some(wgpu::Color::BLACK))
            .depth_attachment(&self.depth_texture.view, Some(0.0))
            .begin();

        rpass.set_bind_group(0, &self.other_bind_group, &[]);
        rpass.set_bind_group(1, self.probe_grid.bind_group(), &[]);

        rpass.set_vertex_buffer(0, mesh_buffers.vertex_buffer.slice(..));
        rpass.set_index_buffer(
            mesh_buffers.index_buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );

        let instance_stride = std::mem::size_of::<InstanceTransform>() as u64;
        for (i, mesh) in mesh_buffers.meshes.iter().enumerate() {
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

            let instance_offset = i as u64 * instance_stride;
            rpass.set_vertex_buffer(
                1,
                mesh_buffers
                    .instance_buffer
                    .slice(instance_offset..instance_offset + instance_stride),
            );

            rpass.draw_indexed(
                mesh.index_offset..(mesh.index_offset + mesh.index_count),
                mesh.vertex_offset as i32,
                0..1,
            );
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
        render_target_view: &wgpu::TextureView,
    ) {
        self.probe_visualization.render(
            render_encoder,
            render_target_view,
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
    type ExtractedData = (HashMap<Entity, BindGroup>, HashMap<Entity, bool>);

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
            let emissive_array = material.emissive.to_array();

            let color_buffer = device
                .buffer()
                .label("Material Color Buffer")
                .usage(wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST)
                .uniform(&color_array);
            let emissive_buffer = device
                .buffer()
                .label("Material Emissive Buffer")
                .usage(wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST)
                .uniform(&emissive_array);
            let material_bind_group = device
                .bind_group(&self.material_bgl)
                .label("Material Bind Group")
                .buffer(0, &color_buffer)
                .buffer(1, &emissive_buffer)
                .build();

            material_bind_groups
                .entry(entity)
                .or_insert(material_bind_group);
            material_double_sided
                .entry(entity)
                .or_insert(material.double_sided);
        }

        Ok((material_bind_groups, material_double_sided))
    }
}
