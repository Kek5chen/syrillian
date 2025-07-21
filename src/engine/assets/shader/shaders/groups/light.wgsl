struct PointLight {
    position: vec3<f32>,
    radius: f32,
    color: vec3<f32>,
    intensity: f32,
    specular_color: vec3<f32>,
    specular_intensity: f32,
}

@group(3) @binding(0)
var<uniform> point_light_count: u32;

@group(3) @binding(1)
var<storage, read> point_lights: array<PointLight>;

