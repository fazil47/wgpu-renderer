pub mod updater;
pub mod visualization;

use crate::wgpu_utils::{QueueExt, WgpuExt};
use maths::Vec3;

#[derive(Clone, Debug, PartialEq)]
pub enum ProbeLightingState {
    Disabled = 0,
    Enabled = 1,
    Only = 2,
}

#[derive(Clone, Debug)]
pub struct ProbeGridConfig {
    pub dimensions: Dimensions,
    pub world_min: Vec3,
    pub world_max: Vec3,
    pub probe_lighting_mix: f32,
    pub probe_lighting_state: ProbeLightingState,
}

impl Default for ProbeGridConfig {
    fn default() -> Self {
        Self {
            dimensions: Dimensions { x: 2, y: 2, z: 2 },
            world_min: Vec3::new(-0.75, -0.75, -0.75),
            world_max: Vec3::new(0.75, 0.75, 0.75),
            probe_lighting_mix: 0.2,
            probe_lighting_state: ProbeLightingState::Enabled,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ProbeGridConfigUniform {
    pub dimensions: Dimensions,
    pub _padding0: u32,
    pub world_min: Vec3,
    pub probe_lighting_mix: f32,
    pub world_max: Vec3,
    pub probe_lighting_state: u32,
}

impl From<&ProbeGridConfig> for ProbeGridConfigUniform {
    fn from(config: &ProbeGridConfig) -> Self {
        Self {
            dimensions: config.dimensions,
            world_min: config.world_min,
            world_max: config.world_max,
            probe_lighting_mix: config.probe_lighting_mix,
            probe_lighting_state: config.probe_lighting_state.clone() as u32,
            _padding0: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Dimensions {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

/// Fixed-size 3D grid of light probes
pub struct ProbeGrid {
    pub config: ProbeGridConfig,
    config_buffer: wgpu::Buffer,
    brick_atlas_sampler: wgpu::Sampler,
    l0_brick_atlas: wgpu::Texture,
    l0_brick_atlas_view: wgpu::TextureView,
    l1x_brick_atlas: wgpu::Texture,
    l1x_brick_atlas_view: wgpu::TextureView,
    l1y_brick_atlas: wgpu::Texture,
    l1y_brick_atlas_view: wgpu::TextureView,
    l1z_brick_atlas: wgpu::Texture,
    l1z_brick_atlas_view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    bind_group_layout: wgpu::BindGroupLayout,
    is_dirty: bool,
}

impl ProbeGrid {
    pub fn new(device: &wgpu::Device, config: ProbeGridConfig) -> Self {
        let Dimensions {
            x: width,
            y: height,
            z: depth,
        } = config.dimensions;

        let bind_group_layout = device
            .bind_group_layout()
            .label("Probe Grid Bind Group Layout")
            .uniform(0, wgpu::ShaderStages::FRAGMENT)
            .sampler(1, wgpu::ShaderStages::FRAGMENT)
            .texture_3d(2, wgpu::ShaderStages::FRAGMENT)
            .texture_3d(3, wgpu::ShaderStages::FRAGMENT)
            .texture_3d(4, wgpu::ShaderStages::FRAGMENT)
            .texture_3d(5, wgpu::ShaderStages::FRAGMENT)
            .build();

        let brick_atlas_sampler = device
            .sampler()
            .label("Probe Grid Sampler")
            .clamp()
            .linear()
            .build();

        let atlas_size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: depth,
        };

        let create_atlas = |label: &str| {
            let tex = device
                .texture()
                .label(label)
                .size_3d(atlas_size)
                .format(wgpu::TextureFormat::Rgba32Float)
                .usage(
                    wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::COPY_DST
                        | wgpu::TextureUsages::STORAGE_BINDING,
                )
                .build();
            let view = tex.create_view(&Default::default());
            (tex, view)
        };

        let (l0_brick_atlas, l0_brick_atlas_view) = create_atlas("Probe SH L0 Atlas");
        let (l1x_brick_atlas, l1x_brick_atlas_view) = create_atlas("Probe SH L1X Atlas");
        let (l1y_brick_atlas, l1y_brick_atlas_view) = create_atlas("Probe SH L1Y Atlas");
        let (l1z_brick_atlas, l1z_brick_atlas_view) = create_atlas("Probe SH L1Z Atlas");

        let config_data = ProbeGridConfigUniform::from(&config);
        let config_buffer = device
            .buffer()
            .label("Probe Grid Uniforms Buffer")
            .uniform(&config_data);

        let bind_group = device
            .bind_group(&bind_group_layout)
            .label("Probe Grid Bind Group")
            .buffer(0, &config_buffer)
            .sampler(1, &brick_atlas_sampler)
            .texture(2, &l0_brick_atlas_view)
            .texture(3, &l1x_brick_atlas_view)
            .texture(4, &l1y_brick_atlas_view)
            .texture(5, &l1z_brick_atlas_view)
            .build();

        Self {
            config,
            config_buffer,
            brick_atlas_sampler,
            l0_brick_atlas,
            l0_brick_atlas_view,
            l1x_brick_atlas,
            l1x_brick_atlas_view,
            l1y_brick_atlas,
            l1y_brick_atlas_view,
            l1z_brick_atlas,
            l1z_brick_atlas_view,
            bind_group_layout,
            bind_group,
            is_dirty: true,
        }
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    pub fn get_l0_brick_atlas_view(&self) -> &wgpu::TextureView {
        &self.l0_brick_atlas_view
    }

    pub fn get_l1x_brick_atlas_view(&self) -> &wgpu::TextureView {
        &self.l1x_brick_atlas_view
    }

    pub fn get_l1y_brick_atlas_view(&self) -> &wgpu::TextureView {
        &self.l1y_brick_atlas_view
    }

    pub fn get_l1z_brick_atlas_view(&self) -> &wgpu::TextureView {
        &self.l1z_brick_atlas_view
    }

    pub fn get_brick_atlas_sampler(&self) -> &wgpu::Sampler {
        &self.brick_atlas_sampler
    }

    pub fn get_config_buffer(&self) -> &wgpu::Buffer {
        &self.config_buffer
    }

    pub fn world_bounds(&self) -> (Vec3, Vec3) {
        (self.config.world_min, self.config.world_max)
    }

    pub fn mark_dirty(&mut self) {
        self.is_dirty = true;
    }

    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    pub fn dimensions(&self) -> Dimensions {
        self.config.dimensions
    }

    pub fn clear_dirty(&mut self) {
        self.is_dirty = false;
    }

    pub fn update_config_buffer(&self, queue: &wgpu::Queue) {
        let config_data = ProbeGridConfigUniform::from(&self.config);
        queue.write_buffer_data(&self.config_buffer, 0, &config_data);
    }

    pub fn recreate_textures_if_needed(&mut self, device: &wgpu::Device) -> bool {
        let Dimensions {
            x: width,
            y: height,
            z: depth,
        } = self.config.dimensions;

        // Check if current texture dimensions match config
        let current_size = self.l0_brick_atlas.size();
        if current_size.width == width
            && current_size.height == height
            && current_size.depth_or_array_layers == depth
        {
            return false; // No change needed
        }

        log::info!(
            "Recreating probe atlas textures with dimensions {width}x{height}x{depth}"
        );

        let atlas_size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: depth,
        };

        let create_atlas = |label: &str| {
            let tex = device
                .texture()
                .label(label)
                .size_3d(atlas_size)
                .format(wgpu::TextureFormat::Rgba32Float)
                .usage(
                    wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::COPY_DST
                        | wgpu::TextureUsages::STORAGE_BINDING,
                )
                .build();
            let view = tex.create_view(&Default::default());
            (tex, view)
        };

        let (l0_brick_atlas, l0_brick_atlas_view) = create_atlas("Probe SH L0 Atlas");
        let (l1x_brick_atlas, l1x_brick_atlas_view) = create_atlas("Probe SH L1X Atlas");
        let (l1y_brick_atlas, l1y_brick_atlas_view) = create_atlas("Probe SH L1Y Atlas");
        let (l1z_brick_atlas, l1z_brick_atlas_view) = create_atlas("Probe SH L1Z Atlas");

        // Update the textures
        self.l0_brick_atlas = l0_brick_atlas;
        self.l0_brick_atlas_view = l0_brick_atlas_view;
        self.l1x_brick_atlas = l1x_brick_atlas;
        self.l1x_brick_atlas_view = l1x_brick_atlas_view;
        self.l1y_brick_atlas = l1y_brick_atlas;
        self.l1y_brick_atlas_view = l1y_brick_atlas_view;
        self.l1z_brick_atlas = l1z_brick_atlas;
        self.l1z_brick_atlas_view = l1z_brick_atlas_view;

        // Recreate bind group with new texture views
        self.bind_group = device
            .bind_group(&self.bind_group_layout)
            .label("Probe Grid Bind Group")
            .buffer(0, &self.config_buffer)
            .sampler(1, &self.brick_atlas_sampler)
            .texture(2, &self.l0_brick_atlas_view)
            .texture(3, &self.l1x_brick_atlas_view)
            .texture(4, &self.l1y_brick_atlas_view)
            .texture(5, &self.l1z_brick_atlas_view)
            .build();

        true // Textures were recreated
    }
}
