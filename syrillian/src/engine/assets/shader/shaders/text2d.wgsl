#use model
#use camera

struct GlyphIn {
    @location(0) offset: vec2<f32>,
    @location(1) atlas_uv: vec2<f32>
}

struct GlyphOut {
    @builtin(position) position: vec4<f32>,
    @location(0) atlas_uv: vec2<f32>,
}

struct PushConstants {
    glyph_size: u32,
    pos: vec2<f32>,
}

var<push_constant> pc: PushConstants;



@vertex
fn vs_main(in: GlyphIn) -> GlyphOut {
    var out: GlyphOut;
    let screen_size = vec2<f32>(system.screen);

    var base_pos = (pc.pos / screen_size - 0.5) * 2;
    base_pos.y = -base_pos.y;
    let sized_offset = (in.offset * f32(pc.glyph_size)) / screen_size;

    out.position = vec4(base_pos + sized_offset, 0.0, 1.0);
    out.atlas_uv = in.atlas_uv;

    return out;
}

@fragment
fn fs_main(data: GlyphOut) -> @location(0) vec4<f32> {
    let color = textureSample(t_diffuse, s_diffuse, data.atlas_uv);

    if (color.a < 0.5) {
        discard;
    }

    return color;
}