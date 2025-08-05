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
    text_pos: vec2<f32>,
    color: vec3<f32>,
    text_size: f32,
}

var<push_constant> pc: PushConstants;



@vertex
fn vs_main(in: GlyphIn) -> GlyphOut {
    var out: GlyphOut;

    let text_pos = pc.text_pos;
    let glyph_offset = (in.offset * pc.text_size) / 100; // TODO: Magic number. I think this can be properly scaled
    let vpos = vec4(text_pos.xy + glyph_offset, 0.0, 1.0);

    out.position = camera.view_proj_mat * model.transform * vpos;
    out.atlas_uv = in.atlas_uv;

    return out;
}

@fragment
fn fs_main(data: GlyphOut) -> @location(0) vec4<f32> {
    let color = textureSample(t_diffuse, s_diffuse, data.atlas_uv);

    if (color.a < 0.1) {
        discard;
    }

    return vec4(pc.color, color.a);
}
