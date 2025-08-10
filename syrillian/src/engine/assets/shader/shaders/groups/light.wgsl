const LIGHT_TYPE_POINT: u32 = 0;
const LIGHT_TYPE_SUN: u32 = 1;
const LIGHT_TYPE_SPOT: u32 = 2;

struct Light {
    position: vec3<f32>,
    direction: vec3<f32>,
    range: f32,
    color: vec3<f32>,
    intensity: f32,
    inner_angle: f32,
    outer_angle: f32,
    type_id: u32,
}

@group(3) @binding(0)
var<uniform> light_count: u32;

struct Lights { data: array<Light>, }
@group(3) @binding(1)
var<storage, read> lights: Lights;

