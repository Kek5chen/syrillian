@vertex
fn vs_main(in: VInput) -> VOutput {
    var out: VOutput;

    let mvp_matrix = camera.view_proj_mat * model.model_mat;

    out.position = mvp_matrix * vec4<f32>(in.vpos, 1.0);
    out.local_position = in.vpos;

    return out;
}

// todo: make shadermanager be able to load vertex and fragment each and combine them in a pipeline. so i can switch 2d and 3d with the fragment shader below
@fragment
fn fs_main(in: VOutput) -> @location(0) vec4<f32> {
    let pos = in.local_position;
    var color = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    if u32(pos.x) % 2 == 0 {
        color = vec4<f32>(1.0, 0.0, 1.0, 1.0);
    }
    return color;
}
