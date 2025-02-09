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

@vertex
fn vs_main(in: VInput) -> VOutput {
    var out: VOutput;

    let model_view_mat = camera.view_proj_mat * model.model_mat;

    out.position = model_view_mat * vec4<f32>(in.vpos, 1.0);
    out.tex_coords = vec2<f32>(in.vtex.x, 1.0 - in.vtex.y);
    out.frag_pos = (model.model_mat * vec4<f32>(in.vpos, 1.0)).xyz;
    out.vnorm = normalize((model.model_mat * vec4<f32>(in.vnorm, 0.0)).xyz);
    out.tangent = normalize((model.model_mat * vec4<f32>(in.vtan, 0.0)).xyz);
    out.bitangent = normalize((model.model_mat * vec4<f32>(in.vbitan, 0.0)).xyz);

    return out;
}

@fragment
fn fs_main(in: VOutput) -> @location(0) vec4<f32> {
    var diffuse: vec4<f32>;

    // diffuse = vec4<f32>(in.vnorm, 1.0);

    diffuse = vec4<f32>(1.0, 0.0, 1.0, 1.0);

    return diffuse;
}
