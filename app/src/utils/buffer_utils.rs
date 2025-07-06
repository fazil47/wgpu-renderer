use crate::{
    ecs::EcsScene,
    wgpu_utils::WgpuExt,
};

/// Unified camera buffer creation for both raytracer and rasterizer
pub struct CameraBuffers {
    pub view_projection: wgpu::Buffer,
    pub camera_to_world: wgpu::Buffer,
    pub camera_inverse_projection: wgpu::Buffer,
}

impl CameraBuffers {
    /// Create camera buffers from ECS scene
    pub fn new(device: &wgpu::Device, scene: &EcsScene, label_prefix: &str) -> Self {
        let (view_proj, camera_to_world, camera_inverse_proj) = if let Some(camera) = scene.get_camera_component() {
            let cam = camera.borrow();
            (
                cam.view_projection().to_cols_array_2d(),
                cam.camera_to_world().to_cols_array_2d(),
                cam.camera_inverse_projection().to_cols_array_2d(),
            )
        } else {
            // Default identity matrices
            let identity = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];
            (identity, identity, identity)
        };

        let view_projection = device
            .buffer()
            .label(&format!("{} View Projection Buffer", label_prefix))
            .uniform(&view_proj);
        
        let camera_to_world = device
            .buffer()
            .label(&format!("{} Camera to World Buffer", label_prefix))
            .uniform(&camera_to_world);
        
        let camera_inverse_projection = device
            .buffer()
            .label(&format!("{} Camera Inverse Projection Buffer", label_prefix))
            .uniform(&camera_inverse_proj);

        Self {
            view_projection,
            camera_to_world,
            camera_inverse_projection,
        }
    }
}

/// Unified lighting buffer creation
pub struct LightingBuffers {
    pub sun_direction: wgpu::Buffer,
}

impl LightingBuffers {
    /// Create lighting buffers from ECS scene
    pub fn new(device: &wgpu::Device, scene: &EcsScene, label_prefix: &str) -> Self {
        let sun_direction_data = if let Some(light) = scene.get_sun_light_component() {
            light.borrow().direction.to_array()
        } else {
            [0.0, -1.0, 0.0] // Default downward light
        };

        let sun_direction = device
            .buffer()
            .label(&format!("{} Sun Direction Buffer", label_prefix))
            .uniform(&sun_direction_data);

        Self { sun_direction }
    }
}

/// Create a uniform buffer for constant data
pub fn create_uniform_constant_buffer<T: bytemuck::Pod>(
    device: &wgpu::Device,
    label: &str,
    data: &T,
) -> wgpu::Buffer {
    device
        .buffer()
        .label(label)
        .uniform(data)
}

/// Create a storage buffer for array data
pub fn create_storage_buffer<T: bytemuck::Pod>(
    device: &wgpu::Device,
    label: &str,
    data: &[T],
) -> wgpu::Buffer {
    device
        .buffer()
        .label(label)
        .storage(data)
}

/// Create vertex buffer
pub fn create_vertex_buffer<T: bytemuck::Pod>(
    device: &wgpu::Device,
    label: &str,
    vertices: &[T],
) -> wgpu::Buffer {
    device
        .buffer()
        .label(label)
        .vertex(vertices)
}

/// Create index buffer (specifically for u32 indices)
pub fn create_index_buffer(
    device: &wgpu::Device,
    label: &str,
    indices: &[u32],
) -> wgpu::Buffer {
    device
        .buffer()
        .label(label)
        .index(indices)
}

/// Update camera buffers when camera changes
impl CameraBuffers {
    pub fn update_from_scene(&self, queue: &wgpu::Queue, scene: &EcsScene) {
        if let Some(camera) = scene.get_camera_component() {
            let cam = camera.borrow();
            queue.write_buffer(
                &self.view_projection,
                0,
                bytemuck::cast_slice(&[cam.view_projection().to_cols_array_2d()]),
            );
            queue.write_buffer(
                &self.camera_to_world,
                0,
                bytemuck::cast_slice(&[cam.camera_to_world().to_cols_array_2d()]),
            );
            queue.write_buffer(
                &self.camera_inverse_projection,
                0,
                bytemuck::cast_slice(&[cam.camera_inverse_projection().to_cols_array_2d()]),
            );
        }
    }
}

/// Update lighting buffers when lighting changes
impl LightingBuffers {
    pub fn update_from_scene(&self, queue: &wgpu::Queue, scene: &EcsScene) {
        if let Some(light) = scene.get_sun_light_component() {
            queue.write_buffer(
                &self.sun_direction,
                0,
                bytemuck::cast_slice(&[light.borrow().direction.to_array()]),
            );
        }
    }
}