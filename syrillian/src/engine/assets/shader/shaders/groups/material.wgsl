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
