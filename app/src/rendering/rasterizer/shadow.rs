use ecs::World;

use crate::{
    lighting::DirectionalLight,
    rendering::{
        GpuVertex,
        rasterizer::{GpuMesh, InstanceTransform},
        wgpu::{QueueExt, WgpuExt, render_pass},
    },
};

pub struct ShadowRenderTexture {
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    light_buffer: wgpu::Buffer,
    light_direction_buffer: wgpu::Buffer,
}

impl ShadowRenderTexture {
    pub fn new(device: &wgpu::Device) -> Self {
        let texture = device
            .texture()
            .label("Shadow Render Texture Ping")
            .size_2d(2048, 2048)
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

        let dummy_light_data = [0.0; 16];
        let light_buffer = device
            .buffer()
            .label("Shadow Light Buffer")
            .uniform(&dummy_light_data);

        let dummy_light_direction = [0.0; 4];
        let light_direction_buffer = device
            .buffer()
            .label("Shadow Light Direction Buffer")
            .uniform(&dummy_light_direction);

        let bind_group_layout = device
            .bind_group_layout()
            .label("Shadow Render Bind Group Layout")
            .uniform(0, wgpu::ShaderStages::VERTEX)
            .uniform(1, wgpu::ShaderStages::VERTEX)
            .build();
        let bind_group = device
            .bind_group(&bind_group_layout)
            .buffer(0, &light_buffer)
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
            view: texture.create_view(&Default::default()),
            sampler,
            pipeline,
            bind_group,
            light_buffer,
            light_direction_buffer,
        }
    }

    pub fn update_light(&mut self, queue: &wgpu::Queue, world: &World, light: &DirectionalLight) {
        queue.write_buffer_data(
            &self.light_buffer,
            0,
            &light.get_light_matrix(world).to_cols_array_2d(),
        );

        let dir = light.direction.to_array();
        let dir4 = [dir[0], dir[1], dir[2], 0.0];
        queue.write_buffer_data(&self.light_direction_buffer, 0, &dir4);
    }

    pub fn render(&self, render_encoder: &mut wgpu::CommandEncoder, gpu_meshes: &Vec<GpuMesh>) {
        let mut rpass = render_pass(render_encoder)
            .label("Shadow map render pass")
            .depth_attachment(&self.view, Some(1.0))
            .begin();

        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, &self.bind_group, &[]);

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

    pub fn get_shadow_map_view(&self) -> &wgpu::TextureView {
        &self.view
    }

    pub fn get_sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    pub fn get_light_matrix_buffer(&self) -> &wgpu::Buffer {
        &self.light_buffer
    }
}
