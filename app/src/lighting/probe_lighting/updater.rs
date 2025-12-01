use crate::rendering::{
    raytracer::{
        RAYTRACE_MATERIAL_STRIDE, RAYTRACE_VERTEX_MATERIAL_INDEX_OFFSET,
        RAYTRACE_VERTEX_NORMAL_OFFSET, RAYTRACE_VERTEX_STRIDE,
    },
    wgpu::WgpuExt,
};

/// Compute pipeline for updating probe coefficients
pub struct ProbeUpdatePipeline {
    compute_pipeline: wgpu::ComputePipeline,
    pub probe_bind_group_layout: wgpu::BindGroupLayout,
}

impl ProbeUpdatePipeline {
    pub fn new(
        device: &wgpu::Device,
        material_bgl: &wgpu::BindGroupLayout,
        mesh_bgl: &wgpu::BindGroupLayout,
        lights_bgl: &wgpu::BindGroupLayout,
    ) -> Self {
        let shader = device
            .shader()
            .label("Probe Update Compute Shader")
            .define_u32("MATERIAL_STRIDE", RAYTRACE_MATERIAL_STRIDE)
            .define_u32("VERTEX_STRIDE", RAYTRACE_VERTEX_STRIDE)
            .define_u32("VERTEX_NORMAL_OFFSET", RAYTRACE_VERTEX_NORMAL_OFFSET)
            .define_u32(
                "VERTEX_MATERIAL_OFFSET",
                RAYTRACE_VERTEX_MATERIAL_INDEX_OFFSET,
            )
            .wesl_runtime("package::probe_lighting::updater");

        let probe_bind_group_layout = device
            .bind_group_layout()
            .label("Probe Update Probe Bind Group Layout")
            .storage_texture_3d(
                0,
                wgpu::ShaderStages::COMPUTE,
                wgpu::StorageTextureAccess::WriteOnly,
                wgpu::TextureFormat::Rgba32Float,
            )
            .storage_texture_3d(
                1,
                wgpu::ShaderStages::COMPUTE,
                wgpu::StorageTextureAccess::WriteOnly,
                wgpu::TextureFormat::Rgba32Float,
            )
            .storage_texture_3d(
                2,
                wgpu::ShaderStages::COMPUTE,
                wgpu::StorageTextureAccess::WriteOnly,
                wgpu::TextureFormat::Rgba32Float,
            )
            .storage_texture_3d(
                3,
                wgpu::ShaderStages::COMPUTE,
                wgpu::StorageTextureAccess::WriteOnly,
                wgpu::TextureFormat::Rgba32Float,
            )
            .uniform(4, wgpu::ShaderStages::COMPUTE)
            .build();

        let pipeline_layout = device
            .pipeline_layout()
            .label("Probe Update Pipeline Layout")
            .bind_group_layouts(&[material_bgl, mesh_bgl, lights_bgl, &probe_bind_group_layout])
            .build();

        let compute_pipeline = device
            .compute_pipeline()
            .label("Probe Update Compute Pipeline")
            .layout(&pipeline_layout)
            .shader(&shader, "main")
            .build()
            .unwrap();

        Self {
            compute_pipeline,
            probe_bind_group_layout,
        }
    }

    pub fn dispatch(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        material_bind_group: &wgpu::BindGroup,
        mesh_bind_group: &wgpu::BindGroup,
        lights_bind_group: &wgpu::BindGroup,
        probe_bind_group: &wgpu::BindGroup,
        probe_count: u32,
    ) {
        log::debug!("ProbeUpdatePipeline::dispatch called with {probe_count} probes");

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Probe Update Compute Pass"),
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(&self.compute_pipeline);
        compute_pass.set_bind_group(0, material_bind_group, &[]);
        compute_pass.set_bind_group(1, mesh_bind_group, &[]);
        compute_pass.set_bind_group(2, lights_bind_group, &[]);
        compute_pass.set_bind_group(3, probe_bind_group, &[]);

        let workgroup_size = 64;
        let num_workgroups = probe_count.div_ceil(workgroup_size);
        log::debug!("Dispatching {num_workgroups} workgroups (workgroup_size={workgroup_size})");
        compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
        log::debug!("Compute workgroups dispatched successfully");
    }
}
