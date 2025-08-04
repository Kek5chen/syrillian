@vertex
fn vs_main(in: VInput) -> VOutput {
    var out: VOutput;

    out.position_clip = model.transform * vec4<f32>(in.position, 1.0);
    out.uv = vec2<f32>(in.uv.x, 1.0 - in.uv.y);

    return out;
}

@fragment
fn fs_main(in: VOutput) -> @location(0) vec4<f32> {
    if material.use_diffuse_texture != 0u {
        return textureSample(t_diffuse, s_diffuse, in.uv);
    } else {
        return vec4<f32>(material.diffuse, 1.0);
    }
}
