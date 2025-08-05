#use model
#use camera

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
    var text_pos = pc.text_pos;
    let glyph_offset = (in.offset * pc.text_size) / 100; // TODO: Magic number. I think this can be properly scaled
    let vpos = vec4(text_pos.xy + glyph_offset, 0.0, 1.0);

    let position = camera.view_proj_mat * model.transform * vpos;

    return position;
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4(pc.color, 1.0);
}
