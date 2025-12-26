@vertex
fn vs_main_2d(in: VInput) -> FInput {
    var out: FInput;

    out.clip = model.transform * vec4<f32>(in.position, 1.0);
    out.uv = vec2<f32>(in.uv.x, 1.0 - in.uv.y);

    return out;
}

@fragment
fn fs_main_2d(in: FInput) -> @location(0) vec4<f32> {
    if mat_has_texture_diffuse(material) {
        return textureSample(t_diffuse, s_diffuse, in.uv);
    } else {
        return vec4<f32>(material.diffuse, 1.0);
    }
}
