@vertex
fn vs_main(in: VInput) -> VOutput {
    var out: VOutput;

    out.clip_pos = model.model_mat * vec4<f32>(in.vpos, 1.0);
    out.tex_coords = vec2<f32>(in.vtex.x, 1.0 - in.vtex.y);

    return out;
}

@fragment
fn fs_main(in: VOutput) -> @location(0) vec4<f32> {
    if material.use_diffuse_texture != 0u {
        return textureSample(t_diffuse, s_diffuse, in.tex_coords);
    } else {
        return vec4<f32>(material.diffuse, 1.0);
    }
}
