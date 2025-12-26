const MAT_PARAM_DIFFUSE_TEXTURE: u32 = 1;
const MAT_PARAM_NORMAL_TEXTURE: u32 = 1 << 1;
const MAT_PARAM_ROUGHNESS_TEXTURE: u32 = 1 << 2;
const MAT_PARAM_LIT: u32 = 1 << 3;
const MAT_PARAM_CAST_SHADOWS: u32 = 1 << 4;
const MAT_PARAM_GRAYSCALE_DIFFUSE: u32 = 1 << 5;

struct Material {
    diffuse: vec3<f32>,
    roughness: f32,
    metallic: f32,
    alpha: f32,
    params: u32,
}
@group(2) @binding(0) var<uniform> material: Material;

@group(2) @binding(1) var t_diffuse: texture_2d<f32>;
@group(2) @binding(2) var s_diffuse: sampler;
@group(2) @binding(3) var t_normal: texture_2d<f32>;
@group(2) @binding(4) var s_normal: sampler;
@group(2) @binding(5) var t_roughness: texture_2d<f32>;
@group(2) @binding(6) var s_roughness: sampler;

fn mat_has_texture_diffuse(material: Material) -> bool {
    return (material.params & MAT_PARAM_DIFFUSE_TEXTURE) != 0u;
}

fn mat_has_texture_normal(material: Material) -> bool {
    return (material.params & MAT_PARAM_NORMAL_TEXTURE) != 0u;
}

fn mat_has_texture_roughness(material: Material) -> bool {
    return (material.params & MAT_PARAM_ROUGHNESS_TEXTURE) != 0u;
}

fn mat_is_lit(material: Material) -> bool {
    return (material.params & MAT_PARAM_LIT) != 0u;
}

fn mat_has_cast_shadows(material: Material) -> bool {
    return (material.params & MAT_PARAM_CAST_SHADOWS) != 0u;
}

fn mat_is_grayscale_diffuse(material: Material) -> bool {
    return (material.params & MAT_PARAM_GRAYSCALE_DIFFUSE) != 0u;
}
