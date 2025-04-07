struct VInput {
    @location(0) vpos: vec3<f32>,
    @location(1) vtex: vec2<f32>,
    @location(2) vnorm: vec3<f32>,
    @location(3) vtan: vec3<f32>,
    @location(4) vbitan: vec3<f32>,
}

struct VOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) frag_pos: vec3<f32>,
    @location(2) vnorm: vec3<f32>,
    @location(3) tangent: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
}

struct CameraData {
    pos: vec3<f32>,
    rot: vec3<f32>,
    scale: vec3<f32>,
    view_mat: mat4x4<f32>,
    projection_mat: mat4x4<f32>,
    view_proj_mat: mat4x4<f32>,
}

struct ModelData {
    model_mat: mat4x4<f32>,
}

struct Material {
    diffuse: vec3<f32>,
    _padding1: u32,
    use_diffuse_texture: u32,
    use_normal_texture: u32,
    shininess: f32,
    opacity: f32,
}

@group(0) @binding(0)
var<uniform> camera: CameraData;

@group(1) @binding(0)
var<uniform> model: ModelData;

@group(2) @binding(0)
var<uniform> material: Material;

@group(2) @binding(1)
var t_diffuse: texture_2d<f32>;

@group(2) @binding(2)
var s_diffuse: sampler;

@group(2) @binding(3)
var t_normal: texture_2d<f32>;

@group(2) @binding(4)
var s_normal: sampler;
