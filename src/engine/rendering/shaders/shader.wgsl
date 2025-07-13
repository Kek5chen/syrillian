struct VOutput {
    @location(0) v_color: vec4<f32>,
    @builtin(position) v_pos: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> transform: vec3<f32>;

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32, @location(0) in_vpos: vec2<f32>) -> VOutput {
    var colors = array<vec4<f32>, 3>(
        vec4<f32>(1, 0, 0, 1),
        vec4<f32>(0, 1, 0, 1),
        vec4<f32>(0, 0, 1, 1),
    );

    var out: VOutput;
    out.v_color = colors[in_vertex_index];
    out.v_pos = vec4<f32>(in_vpos + transform.xy, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VOutput) -> @location(0) vec4<f32> {
    return in.v_color;
}
