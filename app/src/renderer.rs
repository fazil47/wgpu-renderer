use std::sync::Arc;

use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::{
    camera::Camera,
    egui::initialize_egui,
    rasterizer::{self, initialize_rasterizer},
    raytracer::{self, create_raytracer_result_texture, initialize_raytracer},
    scene::Scene,
};

pub struct Renderer {
    pub rasterizer: rasterizer::Rasterizer,
    pub raytracer: raytracer::Raytracer,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
    pub egui: crate::egui::RendererEguiResources,
    pub wgpu: crate::wgpu::RendererWgpuResources,
}

impl Renderer {
    pub async fn new(
        window: Arc<Window>,
        window_size: &winit::dpi::PhysicalSize<u32>,
        camera: &Camera,
        scene: &Scene,
    ) -> Self {
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                // Request an adapter which can render to our surface
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find an appropriate adapter");

        // Create the logical device and command queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Device"),
                    required_features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES, // This can be removed when wgpu is upgraded to the next version.
                    // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
                    required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .expect("Failed to create device");

        let surface_config = surface
            .get_default_config(&adapter, window_size.width, window_size.height)
            .expect("Failed to get default surface configuration");
        surface.configure(&device, &surface_config);

        let (egui_renderer, egui_state) = initialize_egui(
            &window,
            &device,
            &surface_config,
            window.scale_factor() as f32,
        );

        // Initialize vertex and index buffers
        let mesh = crate::mesh::CornellBox::new();
        #[allow(unused_mut, unused_assignments)]
        let mut vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertices Buffer"),
            contents: bytemuck::cast_slice(&mesh.vertices),
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST,
        });
        #[allow(unused_mut, unused_assignments)]
        let mut index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Indices Buffer"),
            contents: bytemuck::cast_slice(&mesh.indices),
            usage: wgpu::BufferUsages::INDEX
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST,
        });
        #[allow(unused_mut, unused_assignments)]
        let mut num_indices = mesh.indices.len() as u32;

        #[cfg(not(target_arch = "wasm32"))]
        {
            let mesh = crate::mesh::PlyMesh::new("assets/cornell-box.ply");
            vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertices Buffer"),
                contents: bytemuck::cast_slice(&mesh.vertices),
                usage: wgpu::BufferUsages::VERTEX
                    | wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST,
            });
            index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Indices Buffer"),
                contents: bytemuck::cast_slice(&mesh.indices),
                usage: wgpu::BufferUsages::INDEX
                    | wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST,
            });
            num_indices = mesh.indices.len() as u32;
        }

        let (
            rasterizer_camera_view_proj_uniform,
            rasterizer_sun_direction_uniform_buffer,
            rasterizer_bind_group,
            rasterizer_render_pipeline,
        ) = initialize_rasterizer(
            &camera,
            &scene.sun_light.direction,
            &device,
            &surface,
            &adapter,
        );

        let rasterizer_depth_texture = crate::wgpu::Texture::create_depth_texture(
            &device,
            &surface_config,
            "rasterizer_depth_texture",
        );

        let (raytracer_result_texture, raytracer_result_texture_view) =
            create_raytracer_result_texture(&device, window_size.width, window_size.height);

        let (
            raytracer_render_bind_group_layout,
            raytracer_render_bind_group,
            raytracer_render_pipeline,
            raytracer_frame_count_uniform_buffer,
            raytracer_vertex_stride_uniform_buffer,
            raytracer_vertex_color_offset_uniform_buffer,
            raytracer_vertex_normal_offset_uniform_buffer,
            raytracer_camera_to_world_uniform_buffer,
            raytracer_camera_inverse_projection_uniform_buffer,
            raytracer_sun_direction_uniform_buffer,
            raytracer_compute_bind_group_layout,
            raytracer_compute_bind_group,
            raytracer_compute_pipeline,
        ) = initialize_raytracer(
            0,
            &vertex_buffer,
            &index_buffer,
            &camera,
            &scene.sun_light.direction,
            &raytracer_result_texture_view,
            &device,
            &surface,
            &adapter,
        );

        Self {
            wgpu: crate::wgpu::RendererWgpuResources {
                instance,
                surface,
                adapter,
                device,
                queue,
                surface_config,
            },
            egui: crate::egui::RendererEguiResources {
                renderer: egui_renderer,
                state: egui_state,
            },
            vertex_buffer,
            index_buffer,
            num_indices,
            rasterizer: rasterizer::Rasterizer {
                depth_texture: rasterizer_depth_texture,
                camera_view_proj_uniform: rasterizer_camera_view_proj_uniform,
                sun_direction_uniform_buffer: rasterizer_sun_direction_uniform_buffer,
                bind_group: rasterizer_bind_group,
                render_pipeline: rasterizer_render_pipeline,
            },
            raytracer: raytracer::Raytracer {
                result_texture: raytracer_result_texture,
                result_texture_view: raytracer_result_texture_view,
                render_bind_group_layout: raytracer_render_bind_group_layout,
                render_bind_group: raytracer_render_bind_group,
                render_pipeline: raytracer_render_pipeline,
                frame_count_uniform_buffer: raytracer_frame_count_uniform_buffer,
                vertex_stride_uniform_buffer: raytracer_vertex_stride_uniform_buffer,
                vertex_color_offset_uniform_buffer: raytracer_vertex_color_offset_uniform_buffer,
                vertex_normal_offset_uniform_buffer: raytracer_vertex_normal_offset_uniform_buffer,
                camera_to_world_uniform_buffer: raytracer_camera_to_world_uniform_buffer,
                camera_inverse_projection_uniform_buffer:
                    raytracer_camera_inverse_projection_uniform_buffer,
                sun_direction_uniform_buffer: raytracer_sun_direction_uniform_buffer,
                compute_bind_group_layout: raytracer_compute_bind_group_layout,
                compute_bind_group: raytracer_compute_bind_group,
                compute_pipeline: raytracer_compute_pipeline,
            },
        }
    }
}
