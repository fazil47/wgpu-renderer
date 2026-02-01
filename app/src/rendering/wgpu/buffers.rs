use ecs::{Entity, World};

use super::WgpuExt;
use crate::{
    camera::Camera, lighting::directional_light::CASCADED_SHADOW_NUM_CASCADES,
    rendering::wgpu::QueueExt,
};

pub struct CameraBuffers {
    pub view_projection: wgpu::Buffer,
    pub camera_to_world: wgpu::Buffer,
    pub camera_inverse_projection: wgpu::Buffer,
}

impl CameraBuffers {
    /// Create camera buffers with default identity matrices
    pub fn new(device: &wgpu::Device, label_prefix: &str) -> Self {
        let identity = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let (view_proj, camera_to_world, camera_inverse_proj) = (identity, identity, identity);

        let view_projection = device
            .buffer()
            .label(&format!("{label_prefix} View Projection Buffer"))
            .usage(wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST)
            .uniform(&[view_proj]);

        let camera_to_world = device
            .buffer()
            .label(&format!("{label_prefix} Camera to World Buffer"))
            .usage(wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST)
            .uniform(&[camera_to_world]);

        let camera_inverse_projection = device
            .buffer()
            .label(&format!("{label_prefix} Camera Inverse Projection Buffer"))
            .usage(wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST)
            .uniform(&[camera_inverse_proj]);

        Self {
            view_projection,
            camera_to_world,
            camera_inverse_projection,
        }
    }
}

/// Unified lighting buffer creation for both raytracer and rasterizer
pub struct LightingBuffers {
    /// The vec3 direction vector of the sun, and a float representing the number of CSM cascades
    pub sun_direction: wgpu::Buffer,
}

impl LightingBuffers {
    /// Create lighting buffers with default downward light direction
    pub fn new(device: &wgpu::Device, label_prefix: &str) -> Self {
        let sun_direction_data = [0.0, -1.0, 0.0, CASCADED_SHADOW_NUM_CASCADES as f32];

        let sun_direction = device
            .buffer()
            .label(&format!("{label_prefix} Sun Direction Buffer"))
            .usage(wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST)
            .uniform(&[sun_direction_data]);

        Self { sun_direction }
    }

    pub fn update_from_world(&self, queue: &wgpu::Queue, world: &World, sun_light_entity: Entity) {
        if let Some(light) =
            world.get_component::<crate::lighting::DirectionalLight>(sun_light_entity)
        {
            queue.write_buffer_data(
                &self.sun_direction,
                0,
                &light.direction.extend(CASCADED_SHADOW_NUM_CASCADES as f32),
            );
        }
    }
}

/// Update camera buffers when camera changes
impl CameraBuffers {
    pub fn update_from_world(&self, queue: &wgpu::Queue, world: &World, camera_entity: Entity) {
        if let Some(camera) = world.get_component::<Camera>(camera_entity) {
            queue.write_buffer(
                &self.view_projection,
                0,
                bytemuck::cast_slice(&[camera.view_projection().to_cols_array_2d()]),
            );
            queue.write_buffer(
                &self.camera_to_world,
                0,
                bytemuck::cast_slice(&[camera.camera_to_world().to_cols_array_2d()]),
            );
            queue.write_buffer(
                &self.camera_inverse_projection,
                0,
                bytemuck::cast_slice(&[camera.camera_inverse_projection().to_cols_array_2d()]),
            );
        }
    }
}
