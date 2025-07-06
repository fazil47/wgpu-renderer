# Architectural Decision Log

## Decision 1: Material Flashing Fix (2025-07-06)

**Problem**: Materials were changing colors randomly due to unstable HashMap
iteration order in ECS material buffer creation.

**Solution**: Added stable sorting by entity ID:
`material_entities.sort_by_key(|entity| entity.0)`

**Impact**: Fixed visual artifacts, ensured consistent material mapping between
raytracer and rasterizer.

**Files**: `app/src/ecs/scene.rs`

## Decision 2: Legacy System Cleanup (2025-07-06)

**Problem**: Codebase had duplicate systems - legacy Scene struct alongside new
ECS, separate Camera class, etc.

**Solution**: Removed all compatibility code:

- Deleted `app/src/scene.rs` (legacy Scene struct)
- Deleted `app/src/camera.rs` (legacy Camera class)
- Deleted `app/src/lights.rs` (legacy DirectionalLight)
- Used pure ECS approach throughout

**Impact**: Reduced code duplication, simplified architecture, eliminated
maintenance burden.

## Decision 3: Module Reorganization (2025-07-06)

**Problem**: Flat module structure with unclear boundaries and poor
organization.

**Solution**: Reorganized into logical modules:

```zsh
src/
├── core/          # Application, Engine, Renderer
├── ecs/           # Entity Component System
├── rendering/     # WGPU, Rasterizer, Raytracer (was graphics/)
├── ui/            # egui integration (was top-level)
├── input/         # Camera controller (was in ecs/)
├── mesh/          # GLTF loading, static meshes
├── lighting/      # Probe lighting systems (was probe_lighting/)
├── shaders/       # WGSL shader files
├── utils/         # Utilities and helpers
└── wgpu_utils/    # WGPU abstraction layer
```

**Impact**: Better separation of concerns, clearer module boundaries, easier
navigation.

## Decision 4: Buffer Creation Consolidation (2025-07-06)

**Problem**: Duplicate buffer creation code between raytracer and rasterizer.

**Solution**: Created unified buffer utilities in
`app/src/utils/buffer_utils.rs`:

- `CameraBuffers` struct for all camera-related buffers
- `LightingBuffers` struct for directional light data
- Shared matrix calculation methods

**Impact**: Reduced code duplication by ~30%, consistent buffer management.

## Decision 5: Camera Controller Architecture (2025-07-06)

**Problem**: Accidentally deleted camera controller during cleanup, mouse input
not working.

**Solution**: Created ECS-compatible camera controller in
`app/src/input/camera_controller.rs`:

- Quaternion-based rotation mathematics
- Proper orthogonal matrix maintenance
- RMB-hold control scheme (instead of ESC toggle)
- Fixed Q/E key mapping for up/down movement

**Impact**: Restored camera functionality, improved control scheme, fixed image
stretching during rotation.

## Decision 6: Dead Code Removal (2025-07-06)

**Problem**: Unused parameters and variables causing compiler warnings.

**Solution**: Removed dead code:

- `textures_recreated` variable in rasterizer (return value not used)
- Unused texture view parameters in `ProbeUpdatePipeline::new`
- Unused `config_sun_bind_group_layout` field and creation
- Made `RaytracerBindGroupLayouts` public to fix visibility warning

**Impact**: Cleaner codebase, eliminated compiler warnings, reduced API surface.

## Decision 7: ECS Component Design (Previous session)

**Problem**: Needed flexible component system for renderer.

**Solution**: Used `Rc<RefCell<T>>` pattern for component sharing:

- Allows multiple systems to access same component
- Runtime borrow checking for safety
- Simple to implement and understand

**Impact**: Enabled flexible ECS architecture without complex lifetime
management.

## Decision 8: Material-Mesh Decoupling (Previous session)

**Problem**: Direct material-mesh coupling limited reusability.

**Solution**: Introduced `MaterialRef` component system:

- Materials stored as separate entities
- Meshes reference materials via `MaterialRef(EntityId)`
- Consistent material indexing across renderers

**Impact**: Improved flexibility, enabled material sharing, simplified buffer
creation.
