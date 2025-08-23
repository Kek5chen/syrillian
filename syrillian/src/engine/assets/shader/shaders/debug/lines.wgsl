struct VSIn {
    @builtin(vertex_index) index: u32,
    @location(0) start: vec3<f32>,
    @location(1) end: vec3<f32>,
    @location(2) start_color: vec4<f32>,
    @location(3) end_color: vec4<f32>,
}

struct VSOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(in: VSIn) -> VSOut {
    var out: VSOut;

    if in.index == 0 {
        out.position = vec4(in.start, 1.0);
        out.color = in.start_color;
    } else {
        out.position = vec4(in.end, 1.0);
        out.color = in.end_color;
    }

    out.position = camera.view_proj_mat * out.position;

    return out;
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    return in.color;
}