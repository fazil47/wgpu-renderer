fn encodeRGBA(rgba: vec4f) -> f32 {
    let r_byte = u32(round(clamp(rgba.r, 0.0, 1.0) * 255.0));
    let g_byte = u32(round(clamp(rgba.g, 0.0, 1.0) * 255.0));
    let b_byte = u32(round(clamp(rgba.b, 0.0, 1.0) * 255.0));
    let a_byte = u32(round(clamp(rgba.a, 0.0, 1.0) * 255.0));
    
    let packed = (r_byte << 24) | (g_byte << 16) | (b_byte << 8) | a_byte;
    
    return bitcast<f32>(packed);
}

fn decodeRGBA(value: f32) -> vec4f {
    let packed = bitcast<u32>(value);
    
    let r = f32((packed >> 24) & 0xFF) / 255.0;
    let g = f32((packed >> 16) & 0xFF) / 255.0;
    let b = f32((packed >> 8) & 0xFF) / 255.0;
    let a = f32(packed & 0xFF) / 255.0;
    
    return vec4f(r, g, b, a);
}