@vertex
fn vs_main(in: VInput) -> VOutput {
    var out: VOutput;

    let world_pos_4 = model.transform * vec4<f32>(in.position, 1.0);
    out.position = world_pos_4.xyz;
    out.position_clip = camera.view_proj_mat * world_pos_4;

    out.uv = vec2<f32>(in.uv.x, 1.0 - in.uv.y);

    // FIXME: This is only correct for uniform scaling + rotation.
    // For non-uniform scaling, transform using the inverse transpose of the model matrix (normal_mat).
    // normal_mat needs to be passed into ModelData.
    out.normal = normalize((model.transform * vec4<f32>(in.normal, 0.0)).xyz);
    out.tangent = normalize((model.transform * vec4<f32>(in.tangent, 0.0)).xyz);
    out.bitangent = normalize((model.transform * vec4<f32>(in.bitangent, 0.0)).xyz);

    out.bone_indices = in.bone_indices;
    out.bone_weights = in.bone_weights;

    return out;
}
