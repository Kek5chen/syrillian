const MAX_BONES : u32 = 256u;

struct ModelData {
    transform: mat4x4<f32>,
    // For correct normal transformation with non-uniform scaling,
    // add the inverse transpose of the upper 3x3 model matrix:
    // normal_mat: mat3x3<f32>,
}
@group(1) @binding(0) var<uniform> model: ModelData;

struct BoneData {
    mats : array<mat4x4<f32>, MAX_BONES>,
}
@group(1) @binding(1) var<uniform> bones: BoneData;



struct Material {
    diffuse: vec3<f32>,
    use_diffuse_texture: u32,
    use_normal_texture: u32,
    shininess: f32,
    opacity: f32,
}
@group(2) @binding(0) var<uniform> material: Material;

@group(2) @binding(1) var t_diffuse: texture_2d<f32>;
@group(2) @binding(2) var s_diffuse: sampler;
@group(2) @binding(3) var t_normal: texture_2d<f32>;
@group(2) @binding(4) var s_normal: sampler;
