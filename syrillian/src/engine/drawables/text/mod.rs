use crate::core::GameObjectId;
use crate::drawables::text::text_layouter::{TextDim, TextLayouter, ThreeD, TwoD};
use crate::drawables::Drawable;
use crate::rendering::{DrawCtx, Renderer};
use crate::World;
use delegate::delegate;
use font_kit::canvas::{Canvas, Format, RasterizationOptions};
use font_kit::font::Font;
use font_kit::hinting::HintingOptions;
use itertools::Itertools;
use log::trace;
use nalgebra::{Matrix4, Vector2, Vector3};
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::{Vector2F, Vector2I};

pub mod glyph;
pub mod text_layouter;

const FONT_ATLAS_GRID_N: u32 = 10;
const FONT_ATLAS_CHARS: [[char; FONT_ATLAS_GRID_N as usize]; FONT_ATLAS_GRID_N as usize] = [
    ['1', '2', '3', '4', '5', '6', '7', '8', '9', '0'],
    ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j'],
    ['k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't'],
    ['u', 'v', 'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D'],
    ['E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N'],
    ['O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X'],
    ['Y', 'Z', '!', '@', '#', '$', '%', '^', '&', '*'],
    ['(', ')', '-', '_', '+', '=', '[', ']', '{', '}'],
    ['|', '\\', ':', ';', '"', '\'', '<', '>', ',', '.'],
    ['/', '?', '`', '~', ' ', '\t', '\n', '\r', '\0', ' '],
];

pub type Text3D = Text<ThreeD>;
pub type Text2D = Text<TwoD>;

#[derive(Debug)]
pub struct Text<DIM: TextDim> {
    text: TextLayouter<DIM>,
}

impl<DIM: TextDim> Text<DIM> {
    pub fn new(text: String, font_family: String, text_size: f32, glyph_size: Option<i32>) -> Self {
        Self {
            text: TextLayouter::new(text, font_family, text_size, glyph_size),
        }
    }

    pub const fn text(&self) -> &TextLayouter<DIM> {
        &self.text
    }

    pub const fn text_mut(&mut self) -> &mut TextLayouter<DIM> {
        &mut self.text
    }

    delegate! {
        to self.text {
            pub fn set_text(&mut self, text: String);
            pub fn set_font(&mut self, family_name: String);
            pub fn set_atlas_glyph_size(&mut self, glyph_size: i32);
            pub const fn set_position(&mut self, x: f32, y: f32);
            pub const fn set_position_vec(&mut self, pos: Vector2<f32>);
            pub const fn set_color(&mut self, r: f32, g: f32, b: f32);
            pub const fn set_color_vec(&mut self, color: Vector3<f32>);
            pub const fn set_size(&mut self, text_size: f32);
        }
    }
}

impl<DIM: TextDim + 'static> Drawable for Text<DIM> {
    fn setup(&mut self, renderer: &Renderer, world: &mut World, _parent: GameObjectId) {
        self.text.setup(renderer, world);
    }

    fn update(&mut self, world: &mut World, parent: GameObjectId, renderer: &Renderer, outer_transform: &Matrix4<f32>) {
        self.text.update(world, parent, renderer, outer_transform)
    }

    fn draw(&self, _world: &mut World, ctx: &DrawCtx) {
        self.text.draw(&ctx.frame.cache, &ctx.pass);
    }
}

fn id_from_atlas(character: char) -> Vector2<u32> {
    for (y, row) in FONT_ATLAS_CHARS.iter().enumerate() {
        if let Some((x, _)) = row.iter().find_position(|c| **c == character) {
            return Vector2::new(x as u32, y as u32);
        }
    }
    Vector2::new(0, 0)
}

fn render_font_atlas(font: &Font, glyph_size: i32) -> Canvas {
    let point_size = glyph_size;
    let point_size_f = point_size as f32;
    let mut canvas = Canvas::new(Vector2I::splat(point_size * 10), Format::Rgba32);
    for (y, row) in FONT_ATLAS_CHARS.iter().enumerate() {
        for (x, ch) in row.iter().enumerate() {
            let x = x as f32;
            let y = y as f32 + 1.;
            let glyph_id = font.glyph_for_char(*ch).unwrap();
            let origin = font.origin(glyph_id).unwrap() * point_size_f;

            font.rasterize_glyph(
                &mut canvas,
                glyph_id,
                point_size_f,
                Transform2F::from_translation(Vector2F::new(
                    origin.x() + point_size_f * x + point_size_f / 8.,
                    origin.y() + point_size_f * y - point_size_f / 8.,
                )),
                HintingOptions::Full(point_size_f),
                RasterizationOptions::GrayscaleAa,
            )
                .unwrap();
        }
    }

    trace!(
        "Generated font atlas of size X={} Y={} (Stride {})",
        canvas.size.x(),
        canvas.size.y(),
        canvas.stride
    );

    assert_eq!(
        canvas.pixels.len(),
        (10 * point_size * 10 * point_size * 4) as usize
    );

    canvas
}
