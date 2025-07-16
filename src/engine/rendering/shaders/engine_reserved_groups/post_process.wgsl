struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
};

@group(0) @binding(0)
var postTexture: texture_2d<f32>;
@group(0) @binding(1)
var postSampler: sampler;
