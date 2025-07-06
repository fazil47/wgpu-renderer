use crate::rendering::wgpu_utils::WgpuExt;
use ecs::{EntityId, World};

/// Unified camera buffer creation for both raytracer and rasterizer
pub struct CameraBuffers {
    pub view_projection: wgpu::Buffer,
    pub camera_to_world: wgpu::Buffer,
    pub camera_inverse_projection: wgpu::Buffer,
}

impl CameraBuffers {
    /// Create camera buffers from ECS World
    pub fn new(
        device: &wgpu::Device,
        world: &World,
        camera_entity: EntityId,
        label_prefix: &str,
    ) -> Self {
        // Extract camera matrices from World
        let (view_proj, camera_to_world, camera_inverse_proj) = if let Some(camera_component) =
            world.get_component::<crate::rendering::Camera>(camera_entity)
        {
            let camera = camera_component.borrow();
            (
                camera.view_projection().to_cols_array_2d(),
                camera.camera_to_world().to_cols_array_2d(),
                camera.camera_inverse_projection().to_cols_array_2d(),
            )
        } else {
            // Fallback to identity matrices
            let identity = [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ];
            (identity, identity, identity)
        };

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
    pub sun_direction: wgpu::Buffer,
}

impl LightingBuffers {
    /// Create lighting buffers from ECS World
    pub fn new(
        device: &wgpu::Device,
        world: &World,
        sun_light_entity: EntityId,
        label_prefix: &str,
    ) -> Self {
        // Extract light direction from World
        let sun_direction_data = if let Some(light_component) =
            world.get_component::<crate::lighting::DirectionalLight>(sun_light_entity)
        {
            let light = light_component.borrow();
            let dir = light.direction.to_array();
            [dir[0], dir[1], dir[2], 0.0] // Convert Vec3 to Vec4 with w=0
        } else {
            [0.0, -1.0, 0.0, 0.0] // Default downward direction
        };

        let sun_direction = device
            .buffer()
            .label(&format!("{label_prefix} Sun Direction Buffer"))
            .usage(wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST)
            .uniform(&[sun_direction_data]);

        Self { sun_direction }
    }
}

/// Update camera buffers when camera changes
impl CameraBuffers {
    pub fn update_from_world(&self, queue: &wgpu::Queue, world: &World, camera_entity: EntityId) {
        if let Some(camera_component) =
            world.get_component::<crate::rendering::Camera>(camera_entity)
        {
            let camera = camera_component.borrow();
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

/// Update lighting buffers when lighting changes
impl LightingBuffers {
    pub fn update_from_world(
        &self,
        queue: &wgpu::Queue,
        world: &World,
        sun_light_entity: EntityId,
    ) {
        if let Some(light_component) =
            world.get_component::<crate::lighting::DirectionalLight>(sun_light_entity)
        {
            let light = light_component.borrow();
            let dir = light.direction.to_array();
            let direction_vec4 = [dir[0], dir[1], dir[2], 0.0]; // Convert Vec3 to Vec4
            queue.write_buffer(
                &self.sun_direction,
                0,
                bytemuck::cast_slice(&[direction_vec4]),
            );
        }
    }
}
