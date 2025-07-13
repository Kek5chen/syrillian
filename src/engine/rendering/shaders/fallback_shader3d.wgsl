@vertex
fn vs_main(in: VInput) -> VOutput {
    var out: VOutput;

    let mvp_matrix = camera.view_proj_mat * model.model_mat;

    out.clip_pos = mvp_matrix * vec4<f32>(in.vpos, 1.0);
    out.tex_coords = in.vtex;

    return out;
}

// todo: make shadermanager be able to load vertex and fragment each and combine them in a pipeline. so i can switch 2d and 3d with the fragment shader below
@fragment
fn fs_main(in: VOutput) -> @location(0) vec4<f32> {
    let tex = in.tex_coords;
    var color = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    if u32(tex.x * 10.0) % 2 == 0 && u32(tex.y * 10.0) % 2 != 0 {
        color = vec4<f32>(1.0, 0.0, 1.0, 1.0);
    } else if u32(tex.x * 10.0) % 2 != 0 && u32(tex.y * 10.0) % 2 == 0 {
        color = vec4<f32>(1.0, 0.0, 1.0, 1.0);
    }
    return color;
}
