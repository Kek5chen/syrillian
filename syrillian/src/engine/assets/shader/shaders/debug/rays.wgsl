struct VSIn {
    @builtin(vertex_index) index: u32,
    @location(0) origin: vec3<f32>,
    @location(1) direction: vec3<f32>,
    @location(2) toi: f32,
}

struct VSOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(in: VSIn) -> VSOut {
    var out: VSOut;

    out.position = vec4(in.origin, 1.0);

    if in.index == 0 {
        out.color = vec4(0.9, 0.2, 0.2, 1.0);
    } else {
        out.position += vec4(normalize(in.direction) * in.toi, 0.0);
        out.color = vec4(0.4, 0.4, 0.2, 1.0);
    }

    out.position = camera.view_proj_mat * out.position;

    return out;
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    return in.color;
}