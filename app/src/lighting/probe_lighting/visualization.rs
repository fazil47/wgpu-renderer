use crate::{
    lighting::probe_lighting::{Dimensions, ProbeGrid, ProbeLightingState},
    rendering::wgpu::{QueueExt, WgpuExt},
    rendering::{rasterizer::GpuVertex, wgpu::WgpuResources},
};
use maths::Vec3;
use wesl::include_wesl;

/// Result of probe UI interaction
#[derive(Debug, Default)]
pub struct ProbeUIResult {
    pub config_changed: bool,
    pub bake_requested: bool,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ProbeInstance {
    pub position: [f32; 3],
    pub _padding0: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
}

impl ProbeInstance {
    pub fn new(position: Vec3) -> Self {
        Self {
            position: position.to_array(),
            _padding0: 0.0,
        }
    }
}

pub struct ProbeVisualization {
    sphere_indices: Vec<u32>,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,
    instance_count: u32,
    pub show_probes: bool,
}

impl ProbeVisualization {
    pub fn new(wgpu: &WgpuResources, probe_grid: &ProbeGrid, camera_buffer: &wgpu::Buffer) -> Self {
        let (sphere_vertices, sphere_indices) = Self::generate_sphere_mesh(16, 16, 0.075);

        let vertex_buffer = wgpu
            .device
            .buffer()
            .label("Probe Sphere Vertex Buffer")
            .vertex(&sphere_vertices);
        let index_buffer = wgpu
            .device
            .buffer()
            .label("Probe Sphere Index Buffer")
            .index(&sphere_indices);

        let instance_buffer = Self::create_instance_buffer(&wgpu.device, 1000);

        let shader = wgpu
            .device
            .shader()
            .label("Probe Visualization Shader")
            .wesl(include_wesl!("probe-visualization").into());

        // Camera bind group layout (Group 0)
        let camera_bind_group_layout = wgpu
            .device
            .bind_group_layout()
            .label("Probe Visualization Camera Bind Group Layout")
            .uniform(0, wgpu::ShaderStages::VERTEX)
            .build();

        // Create camera bind group
        let camera_bind_group = wgpu
            .device
            .bind_group(&camera_bind_group_layout)
            .label("Probe Visualization Camera Bind Group")
            .buffer(0, camera_buffer)
            .build();

        let render_pipeline_layout = wgpu
            .device
            .pipeline_layout()
            .label("Probe Visualization Render Pipeline Layout")
            .bind_group_layout(&camera_bind_group_layout)
            .bind_group_layout(probe_grid.bind_group_layout())
            .build();

        let swapchain_capabilities = wgpu.surface.get_capabilities(&wgpu.adapter);
        let swapchain_format = swapchain_capabilities.formats[0];
        let render_pipeline = wgpu
            .device
            .render_pipeline()
            .label("Probe Visualization Render Pipeline")
            .layout(&render_pipeline_layout)
            .vertex_shader(&shader, "vs_main")
            .fragment_shader(&shader, "fs_main")
            .vertex_buffer(GpuVertex::desc())
            .vertex_buffer(Self::instance_desc())
            .color_target_alpha_blend(swapchain_format)
            .cull_mode(Some(wgpu::Face::Back))
            .depth_test_less(crate::rendering::wgpu::Texture::DEPTH_FORMAT)
            .build()
            .expect("Failed to create probe visualization render pipeline");

        Self {
            sphere_indices,
            vertex_buffer,
            index_buffer,
            instance_buffer,
            camera_bind_group,
            render_pipeline,
            instance_count: 0,
            show_probes: false,
        }
    }

    fn generate_sphere_mesh(
        lat_segments: u32,
        lon_segments: u32,
        radius: f32,
    ) -> (Vec<GpuVertex>, Vec<u32>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        // Generate vertices
        for lat in 0..=lat_segments {
            let theta = lat as f32 * std::f32::consts::PI / lat_segments as f32;
            let sin_theta = theta.sin();
            let cos_theta = theta.cos();

            for lon in 0..=lon_segments {
                let phi = lon as f32 * 2.0 * std::f32::consts::PI / lon_segments as f32;
                let sin_phi = phi.sin();
                let cos_phi = phi.cos();

                let x = cos_phi * sin_theta;
                let y = cos_theta;
                let z = sin_phi * sin_theta;

                let position = [x * radius, y * radius, z * radius, 1.0];
                let normal = [x, y, z, 0.0];

                vertices.push(GpuVertex { position, normal });
            }
        }

        // Generate indices
        for lat in 0..lat_segments {
            for lon in 0..lon_segments {
                let first = lat * (lon_segments + 1) + lon;
                let second = first + lon_segments + 1;

                // First triangle
                indices.push(first);
                indices.push(first + 1);
                indices.push(second);

                // Second triangle
                indices.push(second);
                indices.push(first + 1);
                indices.push(second + 1);
            }
        }

        (vertices, indices)
    }

    fn instance_desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ProbeInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 2,
                format: wgpu::VertexFormat::Float32x3, // position
            }],
        }
    }

    pub fn update_probes(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        probe_grid: &ProbeGrid,
    ) {
        let mut instances = Vec::new();
        let Dimensions { x, y, z } = probe_grid.config.dimensions;
        let world_max = probe_grid.config.world_max;
        let world_min = probe_grid.config.world_min;
        let world_size = world_max - world_min;

        for i in 0..x {
            for j in 0..y {
                for k in 0..z {
                    // Use the same normalization logic as the shader
                    let normalized_pos = Vec3::new(
                        i as f32 / (x - 1).max(1) as f32,
                        j as f32 / (y - 1).max(1) as f32,
                        k as f32 / (z - 1).max(1) as f32,
                    );

                    let position = world_min + normalized_pos * world_size;
                    instances.push(ProbeInstance::new(position));
                }
            }
        }

        self.instance_count = instances.len() as u32;
        self.resize_instance_buffer_if_needed(device, instances.len());
        queue.write_buffer_slice(&self.instance_buffer, 0, &instances);
    }

    pub fn render(
        &self,
        render_encoder: &mut wgpu::CommandEncoder,
        surface_texture_view: &wgpu::TextureView,
        depth_texture_view: &wgpu::TextureView,
        probe_bind_group: &wgpu::BindGroup,
    ) {
        if self.instance_count == 0 || !self.show_probes {
            return;
        }

        // Create render pass using helper function
        let mut render_pass =
            Self::create_render_pass(render_encoder, surface_texture_view, depth_texture_view);

        // Set up rendering state and draw instanced spheres
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        render_pass.set_bind_group(1, probe_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        render_pass.draw_indexed(
            0..self.sphere_indices.len() as u32,
            0,
            0..self.instance_count,
        );
    }

    /// Create instance buffer with initial capacity
    fn create_instance_buffer(device: &wgpu::Device, initial_capacity: usize) -> wgpu::Buffer {
        device
            .buffer()
            .label("Probe Instance Buffer")
            .usage(wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST)
            .empty((std::mem::size_of::<ProbeInstance>() * initial_capacity) as u64)
    }

    /// Resize instance buffer if needed with growth strategy
    fn resize_instance_buffer_if_needed(
        &mut self,
        device: &wgpu::Device,
        required_instances: usize,
    ) {
        let required_size = required_instances * std::mem::size_of::<ProbeInstance>();
        if self.instance_buffer.size() < required_size as u64 {
            // Double the size for future growth to avoid frequent reallocations
            let new_capacity = (required_instances * 2).max(1000);
            self.instance_buffer = Self::create_instance_buffer(device, new_capacity);
        }
    }

    /// Create render pass with standard settings for probe visualization
    fn create_render_pass<'a>(
        render_encoder: &'a mut wgpu::CommandEncoder,
        surface_texture_view: &'a wgpu::TextureView,
        depth_texture_view: &'a wgpu::TextureView,
    ) -> wgpu::RenderPass<'a> {
        render_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Probe Visualization Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: surface_texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load, // Don't clear, render on top
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_texture_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load, // Don't clear depth
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        })
    }

    /// Run UI controls for probe visualization and grid configuration
    pub fn run_ui(&mut self, ui: &mut egui::Ui, probe_grid: &mut ProbeGrid) -> ProbeUIResult {
        let mut result = ProbeUIResult::default();

        ui.collapsing("Probe Grid", |ui| {
            let dims = &mut probe_grid.config.dimensions;
            let mut are_probes_dirty = false;
            are_probes_dirty |= ui
                .add(egui::Slider::new(&mut dims.x, 2..=32).text("Grid X"))
                .changed();
            are_probes_dirty |= ui
                .add(egui::Slider::new(&mut dims.y, 2..=32).text("Grid Y"))
                .changed();
            are_probes_dirty |= ui
                .add(egui::Slider::new(&mut dims.z, 2..=32).text("Grid Z"))
                .changed();

            ui.add_space(10.0);

            let probe_config = &mut probe_grid.config;
            are_probes_dirty |= ui
                .add(
                    egui::Slider::new(&mut probe_config.world_min.x, -20.0..=0.0)
                        .text("World Min X"),
                )
                .changed();
            are_probes_dirty |= ui
                .add(
                    egui::Slider::new(&mut probe_config.world_min.y, -20.0..=0.0)
                        .text("World Min Y"),
                )
                .changed();
            are_probes_dirty |= ui
                .add(
                    egui::Slider::new(&mut probe_config.world_min.z, -20.0..=0.0)
                        .text("World Min Z"),
                )
                .changed();
            are_probes_dirty |= ui
                .add(
                    egui::Slider::new(&mut probe_config.world_max.x, 0.0..=20.0)
                        .text("World Max X"),
                )
                .changed();
            are_probes_dirty |= ui
                .add(
                    egui::Slider::new(&mut probe_config.world_max.y, 0.0..=20.0)
                        .text("World Max Y"),
                )
                .changed();
            are_probes_dirty |= ui
                .add(
                    egui::Slider::new(&mut probe_config.world_max.z, 0.0..=20.0)
                        .text("World Max Z"),
                )
                .changed();

            ui.add_space(10.0);

            are_probes_dirty |= ui
                .add(
                    egui::Slider::new(&mut probe_config.probe_lighting_mix, 0.0..=1.0)
                        .text("Probe Lighting Mix"),
                )
                .changed();

            ui.add_space(10.0);

            let probe_config_lighting_state = &mut probe_config.probe_lighting_state;
            egui::ComboBox::from_id_salt("Probe Lighting State")
                .selected_text(format!("Probe Lighting {probe_config_lighting_state:?}"))
                .show_ui(ui, |ui| {
                    are_probes_dirty |= ui
                        .selectable_value(
                            probe_config_lighting_state,
                            ProbeLightingState::Disabled,
                            "Probe Lighting Disabled",
                        )
                        .changed();

                    are_probes_dirty |= ui
                        .selectable_value(
                            probe_config_lighting_state,
                            ProbeLightingState::Enabled,
                            "Probe Lighting Enabled",
                        )
                        .changed();

                    are_probes_dirty |= ui
                        .selectable_value(
                            probe_config_lighting_state,
                            ProbeLightingState::Only,
                            "Probe Lighting Only",
                        )
                        .changed();
                });

            ui.add_space(10.0);

            if ui.checkbox(&mut self.show_probes, "Show Probes").changed() {
                result.config_changed = true;
            }

            ui.add_space(10.0);

            if ui.button("Bake Probes").clicked() {
                result.bake_requested = true;
                result.config_changed = true;
            }

            if are_probes_dirty {
                result.config_changed = true;
                probe_grid.mark_dirty();
            }
        });

        result
    }
}
