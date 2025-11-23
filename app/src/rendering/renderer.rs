use crate::{
    camera::Camera,
    lighting::DirectionalLight,
    rendering::{rasterizer::Rasterizer, raytracer::Raytracer, wgpu::WgpuResources},
};
use ecs::World;

pub fn renderer_update_system(world: &mut World) {
    // Check if static data is dirty
    let is_dirty = world
        .get_resource::<crate::core::flags::DirtyFlags>()
        .map(|f| f.static_data)
        .unwrap_or(false);

    if !is_dirty {
        return;
    }

    // Find camera and sun light entities
    let camera_entity = world
        .get_entities_with::<Camera>()
        .into_iter()
        .next()
        .expect("No camera entity found");

    let sun_light_entity = world
        .get_entities_with::<DirectionalLight>()
        .into_iter()
        .next()
        .expect("No sun light entity found");

    let wgpu = world.get_resource::<WgpuResources>().unwrap();

    if let Some(mut rasterizer) = world.get_resource_mut::<Rasterizer>() {
        let _ = rasterizer.update_render_data(
            &wgpu.device,
            &wgpu.queue,
            world,
            camera_entity,
            sun_light_entity,
        );
    }

    if let Some(mut raytracer) = world.get_resource_mut::<Raytracer>() {
        let _ = raytracer.update_render_data(
            &wgpu.device,
            &wgpu.queue,
            world,
            camera_entity,
            sun_light_entity,
        );
    }
}
