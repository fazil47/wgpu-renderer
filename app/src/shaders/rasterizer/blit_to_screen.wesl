@group(0) @binding(0) @if(use_depth) var src: texture_depth_2d;
@group(0) @binding(0) @else var src: texture_2d<f32>;

const full_screen_tri_positions: array<vec2f, 3> = array<vec2f, 3>(
    vec2f(-1.0, 3.0),
    vec2f(-1.0, -1.0),
    vec2f(3.0, -1.0)
);

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    if vertex_index == 0u {
        out.position = vec4f(full_screen_tri_positions[0], 0.0, 1.0);
    } else if vertex_index == 1u {
        out.position = vec4f(full_screen_tri_positions[1], 0.0, 1.0);
    } else if vertex_index == 2u {
        out.position = vec4f(full_screen_tri_positions[2], 0.0, 1.0);
    }

    out.uv = out.position.xy * vec2f(0.5, -0.5) + vec2f(0.5, 0.5);

    return out;
}

@fragment
fn fs_main(vert_output: VertexOutput) -> @location(0) vec4f {
    let texture_size = vec2f(textureDimensions(src));
    let coords = vec2i(vert_output.uv * texture_size);
    let tex = textureLoad(src, coords, 0);

    @if(use_depth)
    return vec4f(vec3f(tex), 1.0);
    @else
    return tex;
}