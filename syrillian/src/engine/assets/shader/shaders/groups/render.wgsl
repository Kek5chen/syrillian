struct CameraData {
    position:       vec3<f32>,
    view_mat:       mat4x4<f32>,
    projection_mat: mat4x4<f32>,
    view_proj_mat:  mat4x4<f32>,
}

struct SystemData {
    screen: vec2<u32>,
    time: f32,
    delta_time: f32,
}

@group(0) @binding(0) var<uniform> camera: CameraData;
@group(0) @binding(1) var<uniform> system: SystemData;
//@group(0) @binding(2) var g_normals: texture_storage_2d<rgba8unorm, write>;
//@group(0) @binding(3) var g_position: texture_storage_2d<rgba8unorm, write>;