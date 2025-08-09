struct VInput {
    @location(0) position:     vec3<f32>,
    @location(1) uv:           vec2<f32>,
    @location(2) normal:       vec3<f32>,
    @location(3) tangent:      vec3<f32>,
    @location(4) bone_indices: vec4<u32>,
    @location(5) bone_weights: vec4<f32>,
}

struct FInput {
    @builtin(position) position_clip:  vec4<f32>,
    @location(0) uv:                   vec2<f32>,
    @location(1) position:             vec3<f32>,
    @location(2) normal:               vec3<f32>,
    @location(3) tangent:              vec3<f32>,
    @location(4) bitangent:            vec3<f32>,
    @location(5) bone_indices:         vec4<u32>,
    @location(6) bone_weights:         vec4<f32>,
}
