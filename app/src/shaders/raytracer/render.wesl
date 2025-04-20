import super::super::util::decodeRGBA;

@group(0) @binding(0) @if(wasm) var result: texture_storage_2d<r32float, read>;
@group(0) @binding(0) @else var result: texture_storage_2d<rgba8unorm, read>;

struct VertexOutput {
    @builtin(position) position: vec4f, // Clip-space position
    @location(0) uv: vec2f,
}

var<private> full_screen_tri_positions: array<vec2f, 3> = array<vec2f, 3>(
    vec2f(-1.0, -3.0),
    vec2f(-1.0, 1.0),
    vec2f(3.0, 1.0)
);

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4f(full_screen_tri_positions[vi], 0.0, 1.0);
    out.uv = out.position.xy * 0.5 + 0.5;
    out.uv.y = 1.0 - out.uv.y;

    return out;
}

@fragment
fn fs_main(vert_output: VertexOutput) -> @location(0) vec4f {
    let coord: vec2f = floor(vert_output.uv * vec2f(textureDimensions(result).xy));

    @if(wasm)
    let result_color = decodeRGBA(textureLoad(result, vec2i(coord)).r);
    @else
    let result_color = textureLoad(result, vec2i(coord));

    return result_color;
}
