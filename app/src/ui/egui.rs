use std::{cell::RefCell, rc::Rc};

use ecs::{Entity, World};
use transform_gizmo_egui::{Gizmo, GizmoConfig, GizmoExt, GizmoMode, GizmoOrientation};
use winit::window::Window;

use crate::{camera::Camera, transform::Transform};

pub struct RendererEgui {
    pub renderer: egui_wgpu::Renderer,
    pub state: egui_winit::State,
    pub gizmo: Rc<RefCell<Gizmo>>,
}

impl RendererEgui {
    pub fn new(
        window: &Window,
        device: &wgpu::Device,
        surface_config: &wgpu::SurfaceConfiguration,
        pixels_per_point: f32,
    ) -> Self {
        // TODO: Try with dithering enabled
        let egui_renderer = egui_wgpu::Renderer::new(device, surface_config.format, None, 1, false);
        let egui_ctx = egui::Context::default();

        let egui_viewport_id = egui_ctx.viewport_id();
        let egui_state = egui_winit::State::new(
            egui_ctx,
            egui_viewport_id,
            window,
            Some(pixels_per_point),
            None,
            None,
        );

        Self {
            renderer: egui_renderer,
            state: egui_state,
            gizmo: Rc::new(RefCell::new(Gizmo::default())),
        }
    }

    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        render_encoder: &mut wgpu::CommandEncoder,
        surface_texture_view: &wgpu::TextureView,
        egui_primitives: &[egui::ClippedPrimitive],
        egui_screen_descriptor: &egui_wgpu::ScreenDescriptor,
    ) {
        self.renderer.update_buffers(
            device,
            queue,
            render_encoder,
            egui_primitives,
            egui_screen_descriptor,
        );

        let egui_rpass = render_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Rasterizer Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: surface_texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        self.renderer.render(
            &mut egui_rpass.forget_lifetime(),
            egui_primitives,
            egui_screen_descriptor,
        );
    }

    pub fn update_camera(&self, world: &World, camera_entity: Entity) {
        if let Some(camera_component) = world.get_component::<Camera>(camera_entity) {
            let camera = camera_component.borrow();
            self.gizmo.borrow_mut().update_config(GizmoConfig {
                view_matrix: camera.view_matrix().into(),
                projection_matrix: camera.projection_matrix().into(),
                modes: GizmoMode::all(),
                orientation: GizmoOrientation::Local,
                ..Default::default()
            });
        }
    }

    pub fn select_entity(&self, world: &World, ui: &egui::Ui, entity: Entity) -> bool {
        let mut has_changed = false;
        let transform = world.get_component::<Transform>(entity);

        if transform.is_none() {
            return has_changed;
        }

        let transform = transform.unwrap();
        let transform_read = transform.clone();
        let transform_read = *transform_read.borrow();
        if let Some((_, new_transforms)) = self
            .gizmo
            .borrow_mut()
            .interact(ui, &[transform_read.into()])
        {
            let transform = transform.clone();
            *transform.borrow_mut() = new_transforms[0].into();
            has_changed = true;
        }

        has_changed
    }
}
