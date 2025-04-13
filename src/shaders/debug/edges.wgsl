@vertex
fn vs_main(in: VInput) -> VOutput {
    var out: VOutput;

    let model_view_mat = camera.view_proj_mat * model.model_mat;

    out.clip_pos = model_view_mat * vec4<f32>(in.vpos, 1.0);
    out.tex_coords = vec2<f32>(in.vtex.x, 1.0 - in.vtex.y);
    out.world_pos = (model.model_mat * vec4<f32>(in.vpos, 1.0)).xyz;
    out.world_normal = normalize((model.model_mat * vec4<f32>(in.vnorm, 0.0)).xyz);
    out.world_tangent = normalize((model.model_mat * vec4<f32>(in.vtan, 0.0)).xyz);
    out.world_bitangent = normalize((model.model_mat * vec4<f32>(in.vbitan, 0.0)).xyz);

    return out;
}

@fragment
fn fs_main(in: VOutput) -> @location(0) vec4<f32> {
    var diffuse: vec4<f32>;

    // diffuse = vec4<f32>(in.vnorm, 1.0);

    diffuse = vec4<f32>(1.0, 0.0, 1.0, 1.0);

    return diffuse;
}
