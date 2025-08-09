#use model

struct GlyphIn {
    @location(0) offset: vec2<f32>,
    @location(1) atlas_uv: vec2<f32>
}

struct GlyphOut {
    @builtin(position) position: vec4<f32>,
    @location(0) atlas_uv: vec2<f32>,
}

struct PushConstants {
    text_pos: vec2<f32>,
    color: vec3<f32>,
    text_size: f32,
}

var<push_constant> pc: PushConstants;



@vertex
fn vs_main(in: GlyphIn) -> GlyphOut {
    var out: GlyphOut;
    let screen_size = vec2<f32>(system.screen);

    let vertex_offset = in.offset * f32(pc.text_size); // dumb world-space glyph offset
    let text_offset = vec2(vertex_offset.x, -vertex_offset.y) + pc.text_pos;
    let vpos = model.transform * vec4(text_offset, 0.0, 1.0); // vertex pos in world space
    let screen_pos = vec4((vpos.xy / screen_size - vec2(0.5, 0.5)) * vec2(2, -2), 0.0, 1.0);

    out.position = screen_pos;
    out.atlas_uv = in.atlas_uv;

    return out;
}

@fragment
fn fs_main(data: GlyphOut) -> @location(0) vec4<f32> {
    let color = textureSample(t_diffuse, s_diffuse, data.atlas_uv);

    if (color.a < 0.5) {
        discard;
    }

    return vec4(pc.color, color.a);
}