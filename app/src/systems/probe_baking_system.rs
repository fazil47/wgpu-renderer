use crate::rendering::rasterizer::Rasterizer;
use crate::rendering::wgpu::WgpuResources;
use crate::ui::UiState;
use ecs::World;

pub fn probe_baking_system(world: &mut World) {
    let bake_requested = world
        .get_resource::<UiState>()
        .map(|s| s.bake_requested)
        .unwrap_or(false);

    if !bake_requested {
        return;
    }

    let wgpu = world.get_resource::<WgpuResources>().unwrap();
    // We need mutable access to rasterizer to bake probes
    if let Some(rasterizer) = world.get_resource::<Rasterizer>() {
        // We need material and mesh bind groups from Raytracer
        let (material_bind_group, mesh_bind_group) = {
            let raytracer = world
                .get_resource::<crate::rendering::raytracer::Raytracer>()
                .unwrap();
            (
                raytracer.get_material_bind_group().clone(),
                raytracer.get_mesh_bind_group().clone(),
            )
        };

        rasterizer.bake_probes(
            &wgpu.device,
            &wgpu.queue,
            &material_bind_group,
            &mesh_bind_group,
        );
    }
}
