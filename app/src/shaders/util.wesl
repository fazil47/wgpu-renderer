const DECODE_MULTIPLIER: f32 = 1.0025; // To account for the loss of precision when encoding colors to 8 bits

fn encodeRGBA(rgba: vec4f) -> f32 {
    let r_byte = u32(round(clamp(rgba.r, 0.0, 1.0) * 255.0));
    let g_byte = u32(round(clamp(rgba.g, 0.0, 1.0) * 255.0));
    let b_byte = u32(round(clamp(rgba.b, 0.0, 1.0) * 255.0));
    let a_byte = u32(round(clamp(rgba.a, 0.0, 1.0) * 255.0));

    let packed = (r_byte << 24u) | (g_byte << 16u) | (b_byte << 8u) | a_byte;

    return bitcast<f32>(packed);
}

fn decodeRGBA(value: f32) -> vec4f {
    let packed = bitcast<u32>(value);

    let r = f32((packed >> 24u) & 0xFFu) / 255.0;
    let g = f32((packed >> 16u) & 0xFFu) / 255.0;
    let b = f32((packed >> 8u) & 0xFFu) / 255.0;
    let a = f32(packed & 0xFFu) / 255.0;

    return vec4f(r, g, b, a) * DECODE_MULTIPLIER;
}

fn inverse_mat4(m: mat4x4<f32>) -> mat4x4<f32> {
    let a00 = m[0][0]; let a01 = m[0][1]; let a02 = m[0][2]; let a03 = m[0][3];
    let a10 = m[1][0]; let a11 = m[1][1]; let a12 = m[1][2]; let a13 = m[1][3];
    let a20 = m[2][0]; let a21 = m[2][1]; let a22 = m[2][2]; let a23 = m[2][3];
    let a30 = m[3][0]; let a31 = m[3][1]; let a32 = m[3][2]; let a33 = m[3][3];

    let b00 = a00 * a11 - a01 * a10;
    let b01 = a00 * a12 - a02 * a10;
    let b02 = a00 * a13 - a03 * a10;
    let b03 = a01 * a12 - a02 * a11;
    let b04 = a01 * a13 - a03 * a11;
    let b05 = a02 * a13 - a03 * a12;
    let b06 = a20 * a31 - a21 * a30;
    let b07 = a20 * a32 - a22 * a30;
    let b08 = a20 * a33 - a23 * a30;
    let b09 = a21 * a32 - a22 * a31;
    let b10 = a21 * a33 - a23 * a31;
    let b11 = a22 * a33 - a23 * a32;

    let det = determinant(m);
    if det == 0.0 {
        return mat4x4<f32>(
            vec4f(1.0, 0.0, 0.0, 0.0),
            vec4f(0.0, 1.0, 0.0, 0.0),
            vec4f(0.0, 0.0, 1.0, 0.0),
            vec4f(0.0, 0.0, 0.0, 1.0),
        );
    }
    let inv_det = 1.0 / det;

    return mat4x4<f32>(
        vec4f(
            ( a11 * b11 - a12 * b10 + a13 * b09) * inv_det,
            (-a01 * b11 + a02 * b10 - a03 * b09) * inv_det,
            ( a31 * b05 - a32 * b04 + a33 * b03) * inv_det,
            (-a21 * b05 + a22 * b04 - a23 * b03) * inv_det
        ),
        vec4f(
            (-a10 * b11 + a12 * b08 - a13 * b07) * inv_det,
            ( a00 * b11 - a02 * b08 + a03 * b07) * inv_det,
            (-a30 * b05 + a32 * b02 - a33 * b01) * inv_det,
            ( a20 * b05 - a22 * b02 + a23 * b01) * inv_det
        ),
        vec4f(
            ( a10 * b10 - a11 * b08 + a13 * b06) * inv_det,
            (-a00 * b10 + a01 * b08 - a03 * b06) * inv_det,
            ( a30 * b04 - a31 * b02 + a33 * b00) * inv_det,
            (-a20 * b04 + a21 * b02 - a23 * b00) * inv_det
        ),
        vec4f(
            (-a10 * b09 + a11 * b07 - a12 * b06) * inv_det,
            ( a00 * b09 - a01 * b07 + a02 * b06) * inv_det,
            (-a30 * b03 + a31 * b01 - a32 * b00) * inv_det,
            ( a20 * b03 - a21 * b01 + a22 * b00) * inv_det
        )
    );
}
