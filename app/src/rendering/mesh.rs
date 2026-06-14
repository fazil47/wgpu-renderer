use std::{
    collections::{HashMap, HashSet},
    mem::size_of,
};

use crate::{
    material::{DEFAULT_MATERIAL_INDEX, MaterialIndex},
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
    // Cursors track the next free position in each arena.
    // They only advance on add; removal leaves holes (no compaction).
    vertex_cursor: usize,
    index_cursor: usize,
    mesh_cursor: usize,
}

impl ecs::Resource for MeshBuffers {}

/// Initial size (in bytes) for each GPU buffer (vertex, index, instance).
const INITIAL_BUFFER_SIZE: usize = 4 * 1024; // 4 KB

impl MeshBuffers {
    pub fn new(device: &wgpu::Device) -> Self {
        let zeros = vec![0u8; INITIAL_BUFFER_SIZE];
        let index_zeros = vec![0u32; INITIAL_BUFFER_SIZE / size_of::<u32>()];

        let vertex_buffer = device
            .buffer()
            .label("Mesh Arena Vertex Buffer")
            .usage(wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST)
            .vertex(&zeros);
        let index_buffer = device
            .buffer()
            .label("Mesh Arena Index Buffer")
            .usage(wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST)
            .index(&index_zeros);
        let instance_buffer = device
            .buffer()
            .label("Mesh Arena Instance Buffer")
            .usage(wgpu::BufferUsages::COPY_DST)
            .vertex(&zeros);

        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            vertex_buffer,
            index_buffer,
            instance_buffer,
            meshes: Vec::new(),
            entity_to_meshes_index: HashMap::new(),
            vertex_cursor: 0,
            index_cursor: 0,
            mesh_cursor: 0,
        }
    }

    /// Add a single renderable entity to the mesh arena.
    /// Appends its vertex/index/instance data at the current cursor positions
    /// and advances the cursors. Grows GPU buffers if they run out of space.
    pub fn add_mesh(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        entity: Entity,
        world: &World,
    ) -> Result<(), ExtractionError> {
        let material_index = world
            .get_resource::<MaterialIndex>()
            .ok_or_else(|| ExtractionError::Misc("MaterialIndex resource not found".to_string()))?;

        let global_transform = world.extract_global_transform_component(entity)?;
        let mesh = world.extract_mesh_component(entity)?;
        let mat_index = mesh
            .material_entity
            .and_then(|e| material_index.get(e))
            .unwrap_or(DEFAULT_MATERIAL_INDEX);

        // Build CPU-side vertex data
        let mesh_vertices: Vec<GpuVertex> = mesh
            .vertices()
            .iter()
            .map(|v| GpuVertex::from_vertex(v, mat_index, Mat4::IDENTITY))
            .collect();
        let vertex_count = mesh_vertices.len();

        // Build CPU-side index data
        let mesh_indices: Vec<u32> = match mesh.indices() {
            Some(i) => i.to_vec(),
            None => (0..vertex_count as u32).collect(),
        };
        let index_count = mesh_indices.len();

        // Ensure GPU buffers have enough capacity
        self.ensure_vertex_capacity(device, queue, self.vertex_cursor + vertex_count);
        self.ensure_index_capacity(device, queue, self.index_cursor + index_count);
        self.ensure_instance_capacity(device, queue, self.mesh_cursor + 1);

        // Record offsets before advancing cursors
        let vertex_offset = self.vertex_cursor as u32;
        let index_offset = self.index_cursor as u32;

        // Append to CPU-side vecs
        self.vertices.extend_from_slice(&mesh_vertices);
        self.indices.extend_from_slice(&mesh_indices);

        // Write to GPU buffers at cursor positions
        queue.write_buffer(
            &self.vertex_buffer,
            (self.vertex_cursor * size_of::<GpuVertex>()) as u64,
            bytemuck::cast_slice(&mesh_vertices),
        );
        queue.write_buffer(
            &self.index_buffer,
            (self.index_cursor * size_of::<u32>()) as u64,
            bytemuck::cast_slice(&mesh_indices),
        );

        let instance = InstanceTransform::from_mat4(global_transform.matrix);
        queue.write_buffer(
            &self.instance_buffer,
            (self.mesh_cursor * size_of::<InstanceTransform>()) as u64,
            bytemuck::cast_slice(&[instance]),
        );

        // Advance cursors
        self.vertex_cursor += vertex_count;
        self.index_cursor += index_count;

        self.meshes.push(GpuMesh {
            vertex_offset,
            vertex_count: vertex_count as u32,
            index_offset,
            index_count: index_count as u32,
            transform: global_transform.matrix,
            material_entity: mesh.material_entity,
        });
        self.entity_to_meshes_index.insert(entity, self.mesh_cursor);
        self.mesh_cursor += 1;

        Ok(())
    }

    /// Mark a mesh entity as removed.
    /// Zeros vertex/index counts so the rasterizer won't draw it and the
    /// BVH builder will skip it. The GPU buffer data is left as a hole —
    /// no compaction.
    pub fn remove_mesh(&mut self, entity: Entity) {
        if let Some(&index) = self.entity_to_meshes_index.get(&entity) {
            self.meshes[index].vertex_count = 0;
            self.meshes[index].index_count = 0;
            self.entity_to_meshes_index.remove(&entity);
        }
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

    /// Ensure the vertex buffer can hold at least `required_elements` vertices.
    /// If the current buffer is too small, allocates a new one (at least 2x the
    /// old size) and re-uploads existing data.
    fn ensure_vertex_capacity(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        required_elements: usize,
    ) {
        let required_bytes = required_elements * size_of::<GpuVertex>();
        if required_bytes as u64 <= self.vertex_buffer.size() {
            return;
        }
        let new_size = (self.vertex_buffer.size() as usize * 2).max(required_bytes);
        let new_buffer = device
            .buffer()
            .label("Mesh Arena Vertex Buffer")
            .usage(
                wgpu::BufferUsages::VERTEX
                    | wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST,
            )
            .vertex(&vec![0u8; new_size]);
        if !self.vertices.is_empty() {
            queue.write_buffer(&new_buffer, 0, bytemuck::cast_slice(&self.vertices));
        }
        self.vertex_buffer.destroy();
        self.vertex_buffer = new_buffer;
    }

    /// Ensure the index buffer can hold at least `required_elements` indices.
    fn ensure_index_capacity(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        required_elements: usize,
    ) {
        let required_bytes = required_elements * size_of::<u32>();
        if required_bytes as u64 <= self.index_buffer.size() {
            return;
        }
        let new_size = (self.index_buffer.size() as usize * 2).max(required_bytes);
        let new_buffer = device
            .buffer()
            .label("Mesh Arena Index Buffer")
            .usage(
                wgpu::BufferUsages::INDEX
                    | wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST,
            )
            .index(&vec![0u32; new_size / size_of::<u32>()]);
        if !self.indices.is_empty() {
            queue.write_buffer(&new_buffer, 0, bytemuck::cast_slice(&self.indices));
        }
        self.index_buffer.destroy();
        self.index_buffer = new_buffer;
    }

    /// Ensure the instance buffer can hold at least `required_elements` instances.
    fn ensure_instance_capacity(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        required_elements: usize,
    ) {
        let required_bytes = required_elements * size_of::<InstanceTransform>();
        if required_bytes as u64 <= self.instance_buffer.size() {
            return;
        }
        let new_size = (self.instance_buffer.size() as usize * 2).max(required_bytes);
        let new_buffer = device
            .buffer()
            .label("Mesh Arena Instance Buffer")
            .usage(wgpu::BufferUsages::COPY_DST)
            .vertex(&vec![0u8; new_size]);
        let existing: Vec<InstanceTransform> = self
            .meshes
            .iter()
            .map(|m| InstanceTransform::from_mat4(m.transform))
            .collect();
        if !existing.is_empty() {
            queue.write_buffer(&new_buffer, 0, bytemuck::cast_slice(&existing));
        }
        self.instance_buffer.destroy();
        self.instance_buffer = new_buffer;
    }
}
