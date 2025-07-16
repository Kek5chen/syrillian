@vertex
fn vs_main(in: VInput) -> VOutput {
    var out: VOutput;

    let model_view_mat = camera.view_proj_mat * model.transform;

    out.position_clip = model_view_mat * vec4<f32>(in.position, 1.0);
    out.uv = vec2<f32>(in.uv.x, 1.0 - in.uv.y);
    out.position = (model.transform * vec4<f32>(in.position, 1.0)).xyz;
    out.normal = normalize((model.transform * vec4<f32>(in.normal, 0.0)).xyz);
    out.tangent = normalize((model.transform * vec4<f32>(in.tangent, 0.0)).xyz);
    out.bitangent = normalize((model.transform * vec4<f32>(in.bitangent, 0.0)).xyz);

    return out;
}

@fragment
fn fs_main(in: VOutput) -> @location(0) vec4<f32> {
    var diffuse: vec4<f32>;

    // diffuse = vec4<f32>(in.vnorm, 1.0);

    diffuse = vec4<f32>(1.0, 0.0, 1.0, 1.0);

    return diffuse;
}
