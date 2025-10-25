---
name: rust-refactor
description: Review Rust code for type safety, validate GPU memory layouts (bytemuck, repr(C), alignment), suggest idiomatic patterns, catch GPU-specific safety issues, and help with refactoring during buffer consolidation
---

# Rust Refactoring & GPU Type Safety

## Project Context

You're refactoring wgpu-renderer architecture to consolidate GPU buffers and improve safety. This involves careful Rust/GPU memory interaction where small mistakes cause silent runtime failures (GPU accepts wrong data, renders incorrectly without errors).

**Current State**:
- Custom ECS system with component storage (Arc<RefCell<T>> pattern)
- GPU buffer management with bytemuck for memory safety
- Rasterizer + Raytracer with different buffer layouts
- Buffer consolidation planned (mesh buffers, uniforms, etc.)

**Goal**: Refactor safely with type guarantees and idiomatic Rust patterns.

## What This Skill Knows

### Your Codebase Patterns

**Memory Patterns You Use**:
- `Arc<RefCell<T>>` for shared mutable component storage
- `bytemuck::Pod` and `bytemuck::Zeroable` for GPU data
- `#[repr(C)]` for matching GPU struct layouts
- Manual offset calculations for packed data
- Zero-copy GPU data transfer

**GPU Safety Concerns**:
- Memory alignment (16-byte alignment for uniforms, struct-dependent for storage)
- Data layout matching between Rust and WGSL
- Type safety across Rust/GPU boundary
- Buffer binding safety (can't bind wrong-sized data)
- Lifetime management for shared GPU resources

**Your Buffer Structures** (from .scratchpad):
- CameraBuffers (view_projection, camera_to_world, camera_inverse_projection)
- LightingBuffers (sun_direction)
- RaytracerBuffers (materials, vertices, indices, strides)
- ConsolidatedMeshBuffers (vertex + index consolidation planned)
- SceneData (consolidation target)

### GPU Memory Safety Requirements

**What Must Be True**:
1. **Struct alignment matches GPU backend** - Different APIs align differently
2. **Field order matches shaders** - Rust and WGSL struct definitions must be identical
3. **Padding is explicit** - Don't rely on implicit padding, declare it
4. **bytemuck requirements met** - Must derive Pod and Zeroable correctly
5. **Buffer sizes correct** - Writing wrong amount of data causes GPU memory corruption
6. **Lifetimes sound** - Borrowed buffers must not outlive owner

### Rust Idioms for This Codebase

**Good patterns**:
- Type-level guarantees (generic markers for different buffer purposes)
- Builder pattern for complex initialization
- Trait-based abstraction (Extract trait for ECS data)
- Zero-cost abstractions (no runtime overhead)

**Anti-patterns to avoid**:
- Overuse of Arc<RefCell<>> (causes runtime panics if already borrowed)
- Manual offset tracking (error-prone, doesn't scale)
- Implicit padding (hard to maintain, breaks on struct changes)
- unsafe code without safety comments
- Mixing mutable borrows in hot paths

### Your Constraints

- Self-contained (no heavy dependencies)
- Learning-focused (prefer clarity, add optimization later)
- Cross-platform (code must work on Vulkan/Metal/DX12/WebGPU)
- Minimal external crates (prefer Rust standard library + essentials)

## When to Activate This Skill

Use this skill when:
- **Refactoring code**: "Is this pattern idiomatic?"
- **Type safety concerns**: "Will this compile correctly?"
- **Memory layout issues**: "Does this alignment match GPU?"
- **Lifetime problems**: "Why can't I move this?"
- **Struct changes**: "What padding do I need here?"
- **Buffer consolidation**: "Is this offset calculation correct?"
- **Error handling**: "How should I handle this Result?"
- **Testing refactoring**: "Did I break anything?"

## How This Skill Helps

### 1. **Validate Type Safety**
You ask: "Is this Arc<RefCell<T>> necessary here?"
I analyze:
- Whether shared mutable access is needed
- If you could use Rc<RefCell<T>> instead (thread-local)
- Whether a simple owned value would work
- Runtime panic risks (if borrowed while already borrowed)

### 2. **Check Memory Layouts**
You ask: "Is this struct layout correct for GPU?"
I verify:
- Correct #[repr(C)] usage
- Field alignment (16-byte for uniforms)
- Padding declarations (explicit is better)
- Matches WGSL struct layout exactly
- Within size limits (64 KiB for uniforms)

### 3. **Review bytemuck Derives**
You ask: "Can this derive Pod and Zeroable?"
I check:
- All fields are Pod (no pointers, etc.)
- No generic lifetimes or type parameters
- No padding_is_valid issues (rare, but important)
- Alignment requirements met
- Safe to transmute from/to bytes

### 4. **Suggest Idiomatic Patterns**
You ask: "How should I handle this error?"
I suggest:
- Using Result<T, E> for fallible operations
- Error types and context (anyhow, thiserror)
- Error propagation patterns (? operator)
- Appropriate panic vs. recovery

### 5. **Refactoring Safety**
You ask: "Is it safe to move this field?"
I check:
- Whether it affects Drop order
- Whether Arc/Rc cloning still works
- Whether borrowing patterns change
- Whether GPU resource lifetimes are still valid

## Key Topics I Cover

### Type System
- Generic types and where clauses
- Trait bounds for GPU data (Pod, Zeroable)
- Associated types
- Type aliases for clarity
- Phantom types for compile-time safety

### Memory Management
- Ownership and borrowing rules
- Reference counting (Arc, Rc)
- Interior mutability (RefCell, Mutex)
- Lifetime elision
- Drop order and cleanup

### GPU-Specific Rust
- bytemuck for GPU data
- Memory alignment requirements
- Zero-copy data transfer
- Type safety across boundaries
- Buffer resource management

### Patterns
- Builder pattern (complex initialization)
- RAII (Resource Acquisition Is Initialization)
- Trait-based abstraction
- Marker types
- Error handling patterns

### Refactoring Techniques
- Extracting functions
- Moving fields between structs
- Changing ownership models
- Adding/removing lifetimes
- Testing after changes

## Example Queries This Skill Answers

1. "Is this Arc<RefCell<T>> pattern correct?"
2. "Do my struct alignment rules match GPU requirements?"
3. "Can this field derive bytemuck::Pod?"
4. "Why does this borrow checker error happen?"
5. "Should I use a generic type here?"
6. "How do I handle this Result properly?"
7. "Is it safe to move this field to a new struct?"
8. "What's the idiomatic way to handle this error?"
9. "Does my padding match GPU struct alignment?"
10. "Can I simplify this type definition?"

## GPU Memory Alignment Quick Reference

**Uniform Buffers (std140 layout)**:
- Scalars: 4 bytes
- Vectors (vec2, vec3, vec4): 8 or 16 bytes
- Matrices: Columns are aligned to vec4 (16 bytes)
- Structs: Aligned to largest member, padded to multiple

**Storage Buffers (std430 layout)**:
- More compact than std140
- Scalars: 4 bytes
- Vectors: Same as std140
- Structs: Aligned to largest member

**WGSL Alignment Rules**:
- `mat4x4<f32>`: 64 bytes (4 vec4s)
- `vec3<f32>`: 16 bytes (padded, not 12)
- `array<T, N>`: Each element aligned to 16 bytes minimum

## Safety Checklists

**Before writing to GPU buffer**:
- [ ] Struct size matches buffer size
- [ ] Field order matches shader struct
- [ ] Alignment matches GPU requirements
- [ ] bytemuck derive is correct
- [ ] No uninitialized memory

**Before refactoring structs**:
- [ ] Drop order is still correct
- [ ] Arc/Rc cloning still works
- [ ] GPU buffer layouts unchanged (or updated everywhere)
- [ ] Lifetimes are still valid
- [ ] Tests still pass

**Before consolidating buffers**:
- [ ] Offset calculations are explicit and correct
- [ ] Type safety is maintained (BufferSlice<T>)
- [ ] Alignment matches GPU backend
- [ ] Tests validate offsets match shader

## How I Think About Your Project

Rust's type system is your **safety net** for GPU programming. I help you:
- Leverage Rust's guarantees (compile-time checks instead of runtime debugging)
- Avoid common pitfalls (alignment, lifetime, borrow checker)
- Write idiomatic code (patterns that are clear and maintainable)
- Catch bugs before they silently corrupt GPU memory

Safety first, optimization second.
