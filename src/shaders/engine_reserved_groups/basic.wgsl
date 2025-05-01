const MAX_BONES = 4;

struct VInput {
    @location(0) vpos:         vec3<f32>,
    @location(1) vtex:         vec2<f32>,
    @location(2) vnorm:        vec3<f32>,
    @location(3) vtan:         vec3<f32>,
    @location(4) vbitan:       vec3<f32>,
    @location(5) vboneidx:     vec4<u32>,
    @location(6) vboneweights: vec4<f32>,
}

struct VOutput {
    @builtin(position) clip_pos:  vec4<f32>,
    @location(0) tex_coords:      vec2<f32>,
    @location(1) world_pos:       vec3<f32>,
    @location(2) world_normal:    vec3<f32>,
    @location(3) world_tangent:   vec3<f32>,
    @location(4) world_bitangent: vec3<f32>,
    @location(5) bone_indices:    vec4<u32>,
    @location(6) bone_weights:    vec4<f32>,
}

struct CameraData {
    pos:            vec3<f32>,
    rot:            vec3<f32>, // redundant?
    scale:          vec3<f32>, // redundant?
    view_mat:       mat4x4<f32>,
    projection_mat: mat4x4<f32>,
    view_proj_mat:  mat4x4<f32>,
}

struct BoneData {
    offset_matrix: mat4x4<f32>,
}

struct ModelData {
    model_mat: mat4x4<f32>,
    // For correct normal transformation with non-uniform scaling,
    // add the inverse transpose of the upper 3x3 model matrix:
    // normal_mat: mat3x3<f32>,
}

struct Material {
    diffuse: vec3<f32>,
    use_diffuse_texture: u32,
    use_normal_texture: u32,
    shininess: f32,
    opacity: f32,
}

struct PointLight {
    pos: vec3<f32>,
    color: vec3<f32>,
    radius: f32,
    intensity: f32,
}

struct SystemData {
    screen: vec2<u32>,
    time: f32,
    delta_time: f32,
}

@group(0) @binding(0)
var<uniform> camera: CameraData;

@group(0) @binding(1)
var<uniform> system: SystemData;

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


@group(3) @binding(0)
var<uniform> point_light_count: u32;

@group(3) @binding(1)
var<storage, read> point_lights: array<PointLight>;


@group(4) @binding(0)
var<uniform> bones: BoneData;

