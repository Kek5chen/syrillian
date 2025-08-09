#use model

struct GlyphIn {
    @location(0) offset: vec2<f32>,
}

struct PushConstants {
    text_pos: vec2<f32>,
    color: vec3<f32>,
    text_size: f32,
}

var<push_constant> pc: PushConstants;



@vertex
fn vs_main(in: GlyphIn) -> @builtin(position) vec4<f32> {
    let screen_size = vec2<f32>(system.screen);

    let vertex_offset = in.offset * f32(pc.text_size); // dumb world-space glyph offset
    let text_offset = vec2(vertex_offset.x, -vertex_offset.y) + pc.text_pos;
    let vpos = model.transform * vec4(text_offset, 0.0, 1.0); // vertex pos in world space
    let screen_pos = vec4((vpos.xy / screen_size - vec2(0.5, 0.5)) * vec2(2, -2), 0.0, 1.0);

    return screen_pos;
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4(pc.color, 1.0);
}