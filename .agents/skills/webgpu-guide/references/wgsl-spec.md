# WGSL Specification Summary

Based on W3C WGSL Recommendation.

## Structure of a WGSL Program

A WGSL module contains:
*   **Directives**: Control module behavior (e.g., `enable f16;`).
*   **Functions**: Executable code (e.g., `@vertex fn vs_main() ...`).
*   **Statements**: Variable declarations, control flow.
*   **Literals**: `1.0`, `1u`, `true`.
*   **Constants**: Named values computed at compile/pipeline creation time.
*   **Variables**: Named memory locations (`var<uniform> ...`).
*   **Attributes**: Metadata like `@location(0)`, `@builtin(position)`.

## Built-in Types

### Scalar
*   `bool`: true/false
*   `i32`: 32-bit signed integer
*   `u32`: 32-bit unsigned integer
*   `f32`: 32-bit floating point
*   `f16`: 16-bit floating point (requires extension)

### Vector
*   `vec2<T>`, `vec3<T>`, `vec4<T>`
*   Aliases: `vec2f` (vec2<f32>), `vec3i` (vec3<i32>), `vec4u` (vec4<u32>), `vec2h` (vec2<f16>)

### Matrix
*   `matCxR<T>`: C columns, R rows.
*   Examples: `mat4x4f`, `mat3x3f`.

### Other
*   `array<T, N>`: Fixed size array.
*   `array<T>`: Runtime sized array (last field of storage struct).
*   `struct`: User defined.
*   `atomic<T>`: For atomic operations.

### Texture & Sampler Types
*   `texture_2d<f32>`
*   `texture_depth_2d`
*   `texture_storage_2d<format, access>`
*   `sampler`
*   `sampler_comparison`

## Texture Functions

*   `textureDimensions(t)`: Get size.
*   `textureLoad(t, coords, level)`: Read single texel (no filtering).
*   `textureSample(t, s, coords)`: Sample with filtering.
*   `textureSampleCompare(t, s_cmp, coords, depth_ref)`: Shadow map comparison.
*   `textureStore(t, coords, value)`: Write to storage texture.
