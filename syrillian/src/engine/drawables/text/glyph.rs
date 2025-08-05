use crate::assets::id_from_atlas;
use font_kit::font::Font;
use nalgebra::Vector2;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GlyphVertex {
    pos: Vector2<f32>,
    atlas_uv: Vector2<f32>,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GlyphRenderData {
    triangles: [[GlyphVertex; 3]; 2],
}

#[derive(Debug, Copy, Clone)]
pub enum TextAlignment {
    Left,
    Right,
    Center,
}

impl GlyphVertex {
    pub const fn new(pos: Vector2<f32>, atlas_uv: Vector2<f32>) -> Self {
        Self { pos, atlas_uv }
    }
}

impl GlyphRenderData {
    pub fn new(offset: &Vector2<f32>, atlas_len: u32, glyph: char) -> Self {
        let atlas_id = id_from_atlas(glyph);
        let atlas_len = atlas_len as f32;
        let atlas_base = Vector2::new(atlas_id.x as f32, atlas_id.y as f32 + 1.);

        let atlas_top_left = (atlas_base + Vector2::new(0.0, -1.0)) / atlas_len;
        let atlas_top_right = (atlas_base + Vector2::new(1.0, -1.0)) / atlas_len;
        let atlas_bottom_right = (atlas_base + Vector2::new(1.0, 0.0)) / atlas_len;
        let atlas_bottom_left = atlas_base / atlas_len;

        let pos_top_left = Vector2::new(offset.x, offset.y + 1.);
        let pos_top_right = Vector2::new(offset.x + 1., offset.y + 1.);
        let pos_bottom_left = Vector2::new(offset.x, offset.y);
        let pos_bottom_right = Vector2::new(offset.x + 1., offset.y);

        let top_left = GlyphVertex::new(pos_top_left, atlas_top_left);
        let top_right = GlyphVertex::new(pos_top_right, atlas_top_right);
        let bottom_left = GlyphVertex::new(pos_bottom_left, atlas_bottom_left);
        let bottom_right = GlyphVertex::new(pos_bottom_right, atlas_bottom_right);

        Self {
            triangles: [
                [top_left, bottom_left, top_right],
                [top_right, bottom_left, bottom_right],
            ],
        }
    }
}

fn align_glyph_geometry(
    glyph_bounds: &mut [GlyphRenderData],
    alignment: TextAlignment,
    row_widths: &[(usize, f32)],
) {
    let offset = match alignment {
        TextAlignment::Left => return,
        TextAlignment::Right => -1.,
        TextAlignment::Center => -0.5,
    };

    let mut glyphs = glyph_bounds.iter_mut();
    for (items, width) in row_widths {
        for _ in 0..*items {
            let Some(glyph) = glyphs.next() else {
                if cfg!(debug_assertions) {
                    panic!("Glyphs ran out before row members did");
                }
                return;
            };

            for triangle in glyph.triangles.iter_mut().flatten() {
                triangle.pos.x += offset * width;
            }
        }
    }
}

pub fn generate_glyph_geometry_stream(
    text: &str,
    font: &Font,
    alignment: TextAlignment,
    atlas_len: u32,
) -> Vec<GlyphRenderData> {
    if text.is_empty() {
        return vec![];
    }

    let mut glyph_bounds: Vec<GlyphRenderData> = Vec::new();
    let mut offset = Vector2::zeros();

    let mut row_widths: Vec<(usize, f32)> = Vec::new();
    let mut width: f32 = 0.0;
    let mut row_characters: usize = 0;
    for character in text.chars() {
        if character == '\n' {
            offset = Vector2::new(0.0, offset.y - 1.0);
            row_widths.push((row_characters, width));
            row_characters = 0;
            width = 0.0;
            continue;
        }

        let glyph_id = font.glyph_for_char(character).unwrap();
        let glyph_size = font.advance(glyph_id).unwrap() / 2048.;
        glyph_bounds.push(GlyphRenderData::new(&offset, atlas_len, character));

        offset.x += glyph_size.x();
        width = width.max(offset.x);
        row_characters += 1;
    }
    row_widths.push((row_characters, width));

    align_glyph_geometry(&mut glyph_bounds, alignment, &row_widths);

    glyph_bounds
}
