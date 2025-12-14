use crate::rendering::wgpu::{WgpuExt, render_pass};

pub struct BlitToScreen {
    depth_bgl: wgpu::BindGroupLayout,
    color_bgl: wgpu::BindGroupLayout,
    depth_pipeline: wgpu::RenderPipeline,
    color_pipeline: wgpu::RenderPipeline,
    bind_group: Option<wgpu::BindGroup>,
    use_depth: bool,
}

impl BlitToScreen {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let depth_bgl = device
            .bind_group_layout()
            .label("Blit to screen depth bind group layout")
            .texture_depth(0, wgpu::ShaderStages::FRAGMENT)
            .build();
        let color_bgl = device
            .bind_group_layout()
            .label("Blit to screen color bind group layout")
            .texture_2d(0, wgpu::ShaderStages::FRAGMENT)
            .build();

        let depth_pipeline_layout = device
            .pipeline_layout()
            .label("Blit to screen depth pipeline layout")
            .bind_group_layout(&depth_bgl)
            .build();
        let color_pipeline_layout = device
            .pipeline_layout()
            .label("Blit to screen color pipeline layout")
            .bind_group_layout(&color_bgl)
            .build();

        let depth_shader = device
            .shader()
            .label("Blit to screen depth fragment shader")
            .set_feature("use_depth")
            .wesl_runtime("package::rasterizer::blit_to_screen");
        let depth_pipeline = device
            .render_pipeline()
            .label("Blit to screen depth pipeline")
            .layout(&depth_pipeline_layout)
            .vertex_shader(&depth_shader, "vs_main")
            .fragment_shader(&depth_shader, "fs_main")
            .color_target_replace(surface_format)
            .build()
            .unwrap();

        let color_shader = device
            .shader()
            .label("Blit to screen color fragment shader")
            .wesl_runtime("package::rasterizer::blit_to_screen");
        let color_pipeline = device
            .render_pipeline()
            .label("Blit to screen color pipeline")
            .layout(&color_pipeline_layout)
            .vertex_shader(&color_shader, "vs_main")
            .fragment_shader(&color_shader, "fs_main")
            .color_target_replace(surface_format)
            .build()
            .unwrap();

        Self {
            depth_bgl,
            color_bgl,
            depth_pipeline,
            color_pipeline,
            bind_group: None,
            use_depth: false,
        }
    }

    pub fn set_texture_view(
        &mut self,
        device: &wgpu::Device,
        view: &wgpu::TextureView,
        use_depth: bool,
    ) {
        let bind_group = if use_depth {
            device.bind_group(&self.depth_bgl).texture(0, view).build()
        } else {
            device.bind_group(&self.color_bgl).texture(0, view).build()
        };

        self.bind_group = Some(bind_group);
        self.use_depth = use_depth;
    }

    pub fn render(
        &self,
        render_encoder: &mut wgpu::CommandEncoder,
        surface_texture_view: &wgpu::TextureView,
    ) {
        if self.bind_group.is_none() {
            panic!("Blit to screen bind group not set");
        }
        let bind_group = self.bind_group.as_ref().unwrap();

        let mut rpass = render_pass(render_encoder)
            .label("Blit to screen render pass")
            .color_attachment(&surface_texture_view, Some(wgpu::Color::BLACK))
            .begin();

        let pipeline = if self.use_depth {
            &self.depth_pipeline
        } else {
            &self.color_pipeline
        };

        rpass.set_pipeline(pipeline);

        rpass.set_bind_group(0, bind_group, &[]);

        rpass.draw(0..3, 0..1);
    }
}

impl ecs::Resource for BlitToScreen {}
