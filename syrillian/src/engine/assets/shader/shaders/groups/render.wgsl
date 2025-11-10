struct CameraData {
    position:       vec3<f32>,
    view_mat:       mat4x4<f32>,
    projection_mat: mat4x4<f32>,
    view_proj_mat:  mat4x4<f32>,
    inv_view_proj_mat: mat4x4<f32>,
}

struct SystemData {
    screen: vec2<u32>,
    time: f32,
    delta_time: f32,
}

@group(0) @binding(0) var<uniform> camera: CameraData;
@group(0) @binding(1) var<uniform> system: SystemData;
