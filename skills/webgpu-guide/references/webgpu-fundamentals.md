# WebGPU Fundamentals

Based on webgpufundamentals.org.

## Key Concepts

### Basics
*   **Fundamentals**: The core of WebGPU API.
*   **Inter-stage Variables**: Passing data between shader stages.
*   **Uniforms**: Global read-only variables.
*   **Storage Buffers**: Writable buffers for large data.
*   **Vertex Buffers**: Per-vertex data.
*   **Textures**: Images and data grids.

### Textures
*   **Loading Images**: creating textures from images.
*   **Using Video**: creating textures from video.
*   **Cube Maps**: 6-sided textures.
*   **Storage Textures**: Random access writeable textures.
*   **Multisampling / MSAA**: Anti-aliasing.

### Advanced Topics
*   **Constants**: Pipeline-overridable constants.
*   **Data Memory Layout**: std140 vs std430.
*   **Transparency and Blending**: Alpha blending modes.
*   **Bind Group Layouts**: Describing resource bindings.
*   **Copying Data**: buffer-to-buffer, texture-to-buffer, etc.
*   **Optional Features and Limits**: Capabilities beyond the baseline.
*   **Timing Performance**: Timestamp queries.
*   **Compatibility Mode**: Running on older hardware.

### 3D Math
*   **Matrices**: Translation, Rotation, Scale, Projections.
*   **Cameras**: View matrices, LookAt.
*   **Scene Graphs**: Hierarchical transforms.

### Compute Shaders
*   **Basics**: General purpose GPU computing.
*   **Image Histogram**: Example compute usage.
