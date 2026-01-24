use ecs::World;
use maths::{Mat4, Vec3};

use crate::{
    lighting::{DirectionalLight, directional_light::CASCADED_SHADOW_NUM_CASCADES},
    rendering::{
        GpuVertex,
        rasterizer::{GpuMesh, InstanceTransform},
        wgpu::{QueueExt, WgpuExt, render_pass},
    },
};

pub const SHADOW_MAP_SIZE: u32 = 2048;

pub struct ShadowRenderTexture {
    layers_views: Vec<wgpu::TextureView>, // Render to each layer separately
    array_view: wgpu::TextureView,        // For sampling
    sampler: wgpu::Sampler,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    light_matrices_buffer: wgpu::Buffer,
    light_direction_buffer: wgpu::Buffer,
    matrices_uniform_padded_size: usize,
}

impl ShadowRenderTexture {
    pub fn new(device: &wgpu::Device) -> Self {
        let texture = device
            .texture()
            .label(&format!("Shadow Render Texture Ping"))
            .size_2d_array(
                SHADOW_MAP_SIZE,
                SHADOW_MAP_SIZE,
                CASCADED_SHADOW_NUM_CASCADES as u32,
            )
            .format(wgpu::TextureFormat::Depth32Float)
            .usage(wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING)
            .build();

        let shader = device
            .shader()
            .wesl_buildtime(wesl::include_wesl!("shadow-mapping").into());

        let sampler = device
            .sampler()
            .label("Shadow Map Sampler")
            .shadow()
            .clamp()
            .build();

        let uniform_alignment = device.limits().min_uniform_buffer_offset_alignment as usize;
        let matrices_size = std::mem::size_of::<Mat4>();
        let matrices_uniform_padded_size = uniform_alignment.max(matrices_size);

        let dummy_light_matrices =
            vec![0u8; matrices_uniform_padded_size * CASCADED_SHADOW_NUM_CASCADES];
        let light_matrices_buffer = device
            .buffer()
            .label("Shadow Light Matrices Buffer")
            .uniform_bytes(&dummy_light_matrices);

        // The fourth component contains the number of cascades
        let dummy_light_direction = [0.0, 0.0, 0.0, 0.0];
        let light_direction_buffer = device
            .buffer()
            .label("Shadow Light Direction Buffer")
            .uniform(&dummy_light_direction);

        let bind_group_layout = device
            .bind_group_layout()
            .label("Shadow Render Bind Group Layout")
            .uniform_dynamic(0, wgpu::ShaderStages::VERTEX)
            .uniform(1, wgpu::ShaderStages::VERTEX)
            .build();
        let bind_group = device
            .bind_group(&bind_group_layout)
            .buffer_range(
                0,
                &light_matrices_buffer,
                0,
                Some(matrices_uniform_padded_size as u64),
            )
            .buffer(1, &light_direction_buffer)
            .build();

        let pipeline_layout = device
            .pipeline_layout()
            .label("Shadow Render Pipeline Layout")
            .bind_group_layouts(&[&bind_group_layout])
            .build();
        let pipeline = device
            .render_pipeline()
            .label("Shadow Render Pipeline")
            .layout(&pipeline_layout)
            .vertex_shader(&shader, "vs_main")
            .vertex_buffer(GpuVertex::desc())
            .vertex_buffer(InstanceTransform::desc())
            .cull_mode(Some(wgpu::Face::Back))
            .depth_test_with_bias(
                wgpu::TextureFormat::Depth32Float,
                wgpu::CompareFunction::Less,
                wgpu::DepthBiasState {
                    constant: 2,
                    slope_scale: 3.0,
                    clamp: 0.0,
                },
            )
            .build()
            .unwrap();

        Self {
            layers_views: (0..CASCADED_SHADOW_NUM_CASCADES)
                .map(|i| {
                    texture.create_view(&wgpu::TextureViewDescriptor {
                        base_array_layer: i as u32,
                        array_layer_count: Some(1),
                        dimension: Some(wgpu::TextureViewDimension::D2),
                        ..Default::default()
                    })
                })
                .collect(),
            array_view: texture.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                ..Default::default()
            }),
            sampler,
            pipeline,
            bind_group,
            light_matrices_buffer,
            light_direction_buffer,
            matrices_uniform_padded_size,
        }
    }

    pub fn update_light(&mut self, queue: &wgpu::Queue, world: &World, light: &DirectionalLight) {
        self.write_to_light_buffers(
            queue,
            &light.get_cascaded_light_matrices(world),
            light.direction,
        );
    }

    pub fn render(&self, render_encoder: &mut wgpu::CommandEncoder, gpu_meshes: &Vec<GpuMesh>) {
        for cascade_index in 0..CASCADED_SHADOW_NUM_CASCADES {
            let mut rpass = render_pass(render_encoder)
                .label("Shadow map render pass")
                .depth_attachment(&self.layers_views[cascade_index], Some(1.0))
                .begin();

            rpass.set_pipeline(&self.pipeline);
            let bg_offset = (self.matrices_uniform_padded_size * cascade_index) as u32;
            rpass.set_bind_group(0, &self.bind_group, &[bg_offset]);

            for gpu_mesh in gpu_meshes {
                rpass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
                rpass.set_vertex_buffer(1, gpu_mesh.instance_buffer.slice(..));

                if let Some((buffer, count)) = gpu_mesh.index_buffer.as_ref() {
                    rpass.set_index_buffer(buffer.slice(..), wgpu::IndexFormat::Uint32);
                    rpass.draw_indexed(0..*count, 0, 0..1);
                } else {
                    rpass.draw(0..gpu_mesh.vertex_count, 0..1);
                }
            }
        }
    }

    pub fn get_shadow_map_array_view(&self) -> &wgpu::TextureView {
        &self.array_view
    }

    pub fn get_shadow_map_layer_views(&self) -> &Vec<wgpu::TextureView> {
        &self.layers_views
    }

    pub fn get_sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    pub fn get_light_matrices_buffer(&self) -> &wgpu::Buffer {
        &self.light_matrices_buffer
    }

    fn write_to_light_buffers(
        &self,
        queue: &wgpu::Queue,
        light_matrices: &[Mat4; CASCADED_SHADOW_NUM_CASCADES],
        direction: Vec3,
    ) {
        let mat4_size = std::mem::size_of::<Mat4>();
        let padded_size = self.matrices_uniform_padded_size;
        let mut padded_data = vec![0u8; padded_size * CASCADED_SHADOW_NUM_CASCADES];
        for (i, mat) in light_matrices.iter().enumerate() {
            let offset = i * padded_size;
            let bytes = bytemuck::bytes_of(mat);
            padded_data[offset..(offset + mat4_size)].copy_from_slice(bytes);
        }

        queue.write_buffer_bytes(&self.light_matrices_buffer, 0, &padded_data);

        let dir = direction.to_array();
        let dir4 = [dir[0], dir[1], dir[2], 0.0];
        queue.write_buffer_data(&self.light_direction_buffer, 0, &dir4);
    }
}
