use crate::assets::{Font, HFont, DEFAULT_GLYPH_SIZE};
use crate::core::GameObjectId;
use crate::drawables::text::glyph::TextAlignment;
use crate::drawables::text::text_layouter::{TextDim, TextLayouter, ThreeD, TwoD};
use crate::drawables::Drawable;
use crate::rendering::{DrawCtx, Renderer};
use crate::World;
use delegate::delegate;
use nalgebra::{Matrix4, Vector2, Vector3};

pub mod glyph;
pub mod text_layouter;

pub type Text3D = Text<ThreeD>;
pub type Text2D = Text<TwoD>;

#[derive(Debug)]
pub struct Text<DIM: TextDim> {
    text: TextLayouter<DIM>,
    family_name: String,
    glyph_size: i32,
    font_dirty: bool,
}

impl<DIM: TextDim> Text<DIM> {
    pub fn new(text: String, family_name: String, text_size: f32, glyph_size: Option<i32>) -> Self {
        let world = World::instance();
        let glyph_size = glyph_size.unwrap_or(DEFAULT_GLYPH_SIZE);
        let font = world.assets.fonts.load_sized(&family_name, glyph_size);

        Self {
            text: TextLayouter::new(text, font, text_size),
            family_name,
            glyph_size,
            font_dirty: false,
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
            pub fn set_alignment(&mut self, alignment: TextAlignment);
            #[call(set_font)]
            pub fn set_font_direct(&mut self, font: HFont);
            pub const fn set_position(&mut self, x: f32, y: f32);
            pub const fn set_position_vec(&mut self, pos: Vector2<f32>);
            pub const fn set_color(&mut self, r: f32, g: f32, b: f32);
            pub const fn set_color_vec(&mut self, color: Vector3<f32>);
            pub const fn set_size(&mut self, text_size: f32);
        }
    }

    pub fn set_font(&mut self, font_family: String) {
        self.family_name = font_family;
        self.font_dirty = true;
    }
}

impl<DIM: TextDim + 'static> Drawable for Text<DIM> {
    fn setup(&mut self, renderer: &Renderer, world: &mut World, _parent: GameObjectId) {
        self.text.setup(renderer, world);
    }

    fn update(
        &mut self,
        world: &mut World,
        parent: GameObjectId,
        renderer: &Renderer,
        outer_transform: &Matrix4<f32>,
    ) {
        if self.font_dirty {
            let font = world
                .assets
                .fonts
                .add(Font::new(self.family_name.clone(), Some(self.glyph_size)));
            self.text.set_font(font);
            self.font_dirty = false;
        }

        self.text.update(world, parent, renderer, outer_transform)
    }

    fn draw(&self, _world: &mut World, ctx: &DrawCtx) {
        self.text.draw(&ctx);
    }
}
