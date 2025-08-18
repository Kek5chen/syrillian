struct VInput {
    @location(0) position: vec3<f32>,
    @location(1) uv:       vec2<f32>,
    @location(2) normal:   vec3<f32>,
    @location(3) tangent:  vec3<f32>,
    @location(4) bone_idx: vec4<u32>,
    @location(5) bone_w:   vec4<f32>,
}

struct FInput {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv:         vec2<f32>,
    @location(1) position:   vec3<f32>,
    @location(2) normal:     vec3<f32>,
    @location(3) tangent:    vec3<f32>,
    @location(4) bitangent:  vec3<f32>,
    @location(5) bone_idx:   vec4<u32>,
    @location(6) bone_w:     vec4<f32>,
}
