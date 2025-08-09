#use model
#use light

struct VSOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

var<push_constant> light_index: u32;

// Sun Light

fn calculate_sun_offset(light: Light, vid: u32, iid: u32) -> vec3<f32> {
    const ROWS: u32 = 3;
    const COLS: u32 = 3;

    let x = f32(iid % ROWS) - f32(ROWS / 2);
    let y = f32(iid / COLS) - f32(COLS / 2);

    let dir = normalize(light.direction);
    let dirT = cross(dir, vec3(0.0, 1.0, 0.0));
    let dirB = cross(dir, dirT);

    var offset = dirT * x + dirB * y;
    if vid == 0 {
        offset += dir * light.range;
    }

    return offset;
}

// Point Light

fn calculate_point_offset(light: Light, vid: u32, iid: u32) -> vec3<f32> {
    let ray_dir = calculate_point_dir(iid);
    let scaled = ray_dir * light.range / 2;

    if vid == 0 {
        return scaled;
    } else {
        return -scaled;
    }
}

fn calculate_point_dir(iid: u32) -> vec3<f32> {
    if iid > 5 {
        return vec3(0.0, 1.0, 0.0);
    }

    const DIRS = array<vec3<f32>, 6>(
        vec3(1.0, 0.0, 0.0),
        vec3(0.0, 1.0, 0.0),
        vec3(0.0, 0.0, 1.0),

        vec3(0.5, 0.5, 0.0),
        vec3(0.0, 0.5, 0.5),
        vec3(0.5, 0.0, 0.5),
    );

    return DIRS[iid];
}

@vertex
fn vs_main(@builtin(vertex_index) vid: u32, @builtin(instance_index) iid: u32) -> VSOut {
    var out: VSOut;
    let light = lights.data[light_index];

    var offset: vec3<f32>;
    var alpha: f32;

    if light.type_id == LIGHT_TYPE_SUN {
        offset = calculate_sun_offset(light, vid, iid);
        alpha = f32(vid);
    } else if light.type_id == LIGHT_TYPE_POINT {
        offset = calculate_point_offset(light, vid, iid);
        alpha = 1.0;
    }

    out.position = vec4(light.position + offset, 1.0);
    out.color = vec4(1.0, 1.0, 1.0, alpha);

    out.position = camera.view_proj_mat * out.position;

    return out;
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    return in.color;
}