// Cubemap Skybox Shader for Syrillian Engine
// Renders cubemap-based skyboxes with proper infinite distance rendering

const MAX_DEPTH: f32 = 1.0;

// Cubemap texture and sampler bindings
@group(0) @binding(0) var cube_texture: texture_cube<f32>;
@group(0) @binding(1) var cube_sampler: sampler;

// View-projection matrices uniform buffer
@group(1) @binding(0) var<uniform> camera_uniform: CameraUniform;

struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
}

// Vertex input structure
struct VertexInput {
    @location(0) position: vec3<f32>,
}

// Vertex output / Fragment input structure  
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec3<f32>,
}

// Vertex shader - transforms cube vertices to screen space
@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    
    // Use the vertex position directly as texture coordinates for cubemap sampling
    output.tex_coords = input.position;
    
    // Extract rotation-only view matrix for infinite distance rendering
    var rotation_only_view = extract_rotation_matrix(camera_uniform.view);
    
    // Transform using projection and rotation-only view matrices
    output.position = camera_uniform.view_proj * rotation_only_view * vec4<f32>(input.position, 1.0);
    
    // Force skybox to render at maximum depth
    output.position.z = output.position.w;
    
    return output;
}

// Helper function to extract rotation-only matrix from view matrix
fn extract_rotation_matrix(view: mat4x4<f32>) -> mat4x4<f32> {
    return mat4x4<f32>(
        vec4<f32>(view[0].xyz, 0.0),
        vec4<f32>(view[1].xyz, 0.0),
        vec4<f32>(view[2].xyz, 0.0),
        vec4<f32>(0.0, 0.0, 0.0, 1.0)
    );
}

// Fragment shader - samples from cubemap texture
@fragment  
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Sample cubemap using normalized direction vector
    let coords = normalize(input.tex_coords);
    let color = textureSample(cube_texture, cube_sampler, coords).rgb;
    
    return vec4<f32>(color, 1.0);
}