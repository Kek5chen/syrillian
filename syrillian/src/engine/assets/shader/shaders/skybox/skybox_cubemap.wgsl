#use cubemap

const MAX_DEPTH: f32 = 1.0;

struct VertexInput {
    @location(0) position: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec3<f32>,
}

// Vertex shader - transforms cube vertices to screen space
@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.tex_coords = input.position;
    let rotated = camera.view_mat * vec4<f32>(input.position, 0.0);
    output.position = camera.projection_mat * vec4<f32>(rotated.xyz, 1.0);
    output.position.z = output.position.w;
    return output;
}

@fragment  
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let coords = normalize(input.tex_coords);
    let color = textureSample(cube_texture, cube_sampler, coords).rgb;
    
    return vec4<f32>(color, 1.0);
}