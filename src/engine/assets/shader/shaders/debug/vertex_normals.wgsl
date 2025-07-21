#use model

struct VSIn {
    // 0 = base, 1 = tip
    @location(0) index: u32,
    @location(1) position: vec3<f32>,
    @location(2) normal: vec3<f32>,
}

struct VSOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(in: VSIn) -> VSOut {
    var out: VSOut;

    var world_pos = vec4(in.position, 1.0);

    if in.index == 0 {
        out.color = vec4(0.5, 0.0, 1.0, 1.0);
    } else {
        world_pos += vec4(normalize(in.normal) / 2, 0.0);
        out.color = vec4(0.0, 0.5, 1.0, 0.0);
    }

    out.position = camera.view_proj_mat * model.transform * world_pos;

    return out;
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    return in.color;
}