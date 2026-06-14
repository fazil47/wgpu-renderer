use std::{
    collections::{HashMap, HashSet},
    mem::size_of,
};

use crate::{
    material::MaterialIndex,
    rendering::{
        GpuVertex,
        extract::{ExtractionError, WorldExtractExt},
        wgpu::WgpuExt,
    },
};
use ecs::{Entity, World};
use maths::Mat4;

// Per-instance transform data (mat4 as four vec4 columns)
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceTransform {
    pub col0: [f32; 4],
    pub col1: [f32; 4],
    pub col2: [f32; 4],
    pub col3: [f32; 4],
}

impl InstanceTransform {
    const ATTRIBS: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![
        2 => Float32x4,
        3 => Float32x4,
        4 => Float32x4,
        5 => Float32x4
    ];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBS,
        }
    }

    pub fn from_mat4(transform: Mat4) -> Self {
        let matrix = transform.to_cols_array_2d();
        Self {
            col0: matrix[0],
            col1: matrix[1],
            col2: matrix[2],
            col3: matrix[3],
        }
    }
}

pub struct GpuMesh {
    pub vertex_offset: u32,
    pub vertex_count: u32,
    pub index_offset: u32,
    pub index_count: u32,
    pub transform: Mat4,
    pub material_entity: Option<Entity>,
}

pub struct MeshBuffers {
    pub vertices: Vec<GpuVertex>,
    pub indices: Vec<u32>,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub instance_buffer: wgpu::Buffer,
    pub meshes: Vec<GpuMesh>,
    entity_to_meshes_index: HashMap<Entity, usize>,
}

impl ecs::Resource for MeshBuffers {}

impl MeshBuffers {
    pub fn new(device: &wgpu::Device) -> Self {
        let vertices = vec![GpuVertex {
            position: [0.0, 0.0, 0.0, 1.0],
            normal: [0.0, 1.0, 0.0, 0.0],
            material_index: 0.0,
        }];
        let indices = vec![0u32];
        let instance_transforms = vec![InstanceTransform::from_mat4(Mat4::IDENTITY)];

        let vertex_buffer = device
            .buffer()
            .label("Mesh Arena Vertex Buffer")
            .usage(wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE)
            .vertex(&vertices);
        let index_buffer = device
            .buffer()
            .label("Mesh Arena Index Buffer")
            .usage(wgpu::BufferUsages::INDEX | wgpu::BufferUsages::STORAGE)
            .index(&indices);
        let instance_buffer = device
            .buffer()
            .label("Mesh Arena Instance Buffer")
            .usage(wgpu::BufferUsages::COPY_DST)
            .vertex(&instance_transforms);

        Self {
            vertices,
            indices,
            vertex_buffer,
            index_buffer,
            instance_buffer,
            meshes: Vec::new(),
            entity_to_meshes_index: HashMap::new(),
        }
    }

    pub fn update(&mut self, device: &wgpu::Device, world: &World) -> Result<(), ExtractionError> {
        let material_index = world
            .get_resource::<MaterialIndex>()
            .ok_or_else(|| ExtractionError::Misc("MaterialIndex resource not found".to_string()))?;

        let renderables = world.get_renderables();
        self.vertices.clear();
        self.indices.clear();
        self.meshes.clear();
        self.entity_to_meshes_index.clear();
        let mut instance_transforms: Vec<InstanceTransform> = Vec::new();

        for entity in renderables {
            let global_transform = world.extract_global_transform_component(entity)?;
            let mesh = world.extract_mesh_component(entity)?;
            let mat_index = mesh
                .material_entity
                .and_then(|e| material_index.get(e))
                .unwrap_or(0);

            let vertex_offset = self.vertices.len() as u32;
            let mesh_vertices: Vec<GpuVertex> = mesh
                .vertices()
                .iter()
                .map(|v| GpuVertex::from_vertex(v, mat_index, Mat4::IDENTITY))
                .collect();
            let vertex_count = mesh_vertices.len() as u32;
            self.vertices.extend_from_slice(&mesh_vertices);

            let index_offset = self.indices.len() as u32;
            let mesh_indices: Vec<u32> = match mesh.indices() {
                Some(i) => i.to_vec(),
                None => (0..vertex_count).collect(),
            };
            let index_count = mesh_indices.len() as u32;
            self.indices.extend_from_slice(&mesh_indices);

            instance_transforms.push(InstanceTransform::from_mat4(global_transform.matrix));

            self.meshes.push(GpuMesh {
                vertex_offset,
                vertex_count,
                index_offset,
                index_count,
                transform: global_transform.matrix,
                material_entity: mesh.material_entity,
            });
            self.entity_to_meshes_index
                .insert(entity, self.meshes.len() - 1);
        }

        self.vertex_buffer.destroy();
        self.vertex_buffer = device
            .buffer()
            .label("Mesh Arena Vertex Buffer")
            .usage(wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE)
            .vertex(&self.vertices);

        self.index_buffer.destroy();
        self.index_buffer = device
            .buffer()
            .label("Mesh Arena Index Buffer")
            .usage(wgpu::BufferUsages::INDEX | wgpu::BufferUsages::STORAGE)
            .index(&self.indices);

        self.instance_buffer.destroy();
        self.instance_buffer = device
            .buffer()
            .label("Mesh Arena Instance Buffer")
            .usage(wgpu::BufferUsages::COPY_DST)
            .vertex(&instance_transforms);

        Ok(())
    }

    /// Update only the instance transforms for entities whose GlobalTransform changed.
    pub fn update_transforms(
        &mut self,
        queue: &wgpu::Queue,
        world: &World,
    ) -> Result<(), ExtractionError> {
        let changed_entities: HashSet<Entity> =
            match world.read_events::<crate::core::events::GlobalTransformChanged>() {
                Some(events) => events.iter().map(|e| e.0).collect(),
                None => return Ok(()),
            };

        let instance_stride = size_of::<InstanceTransform>();

        for &entity in &changed_entities {
            if let Some(&index) = self.entity_to_meshes_index.get(&entity) {
                let global_transform = world.extract_global_transform_component(entity)?;
                self.meshes[index].transform = global_transform.matrix;

                let instance = InstanceTransform::from_mat4(global_transform.matrix);
                queue.write_buffer(
                    &self.instance_buffer,
                    (index * instance_stride) as u64,
                    bytemuck::cast_slice(&[instance]),
                );
            }
        }

        Ok(())
    }
}
