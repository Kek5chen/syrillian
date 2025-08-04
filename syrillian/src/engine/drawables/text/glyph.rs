use crate::drawables::text::{id_from_atlas, FONT_ATLAS_GRID_N};
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

impl GlyphVertex {
    pub const fn new(pos: Vector2<f32>, atlas_uv: Vector2<f32>) -> Self {
        Self { pos, atlas_uv }
    }
}

impl GlyphRenderData {
    pub fn new(offset: &Vector2<f32>, glyph: char) -> Self {
        let atlas_id = id_from_atlas(glyph);
        let atlas_len = FONT_ATLAS_GRID_N as f32;
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
                // [top_left, top_right.clone(), bottom_left.clone()],
                // [top_right, bottom_right, bottom_left.clone()],
                [top_left, bottom_left.clone(), top_right.clone()],
                [top_right, bottom_left.clone(), bottom_right],
            ],
        }
    }
}

pub fn generate_glyph_geometry_stream(text: &str, font: &Font) -> Vec<GlyphRenderData> {
    let mut glyph_bounds: Vec<GlyphRenderData> = Vec::new();
    let mut offset = Vector2::zeros();

    for character in text.chars() {
        if character == '\n' {
            offset = Vector2::new(0., offset.y - 1.);
            continue;
        }

        let glyph_id = font.glyph_for_char(character).unwrap();
        let glyph_size = font.advance(glyph_id).unwrap() / 2048.;
        glyph_bounds.push(GlyphRenderData::new(&offset, character));
        offset.x += glyph_size.x();
    }

    glyph_bounds
}
