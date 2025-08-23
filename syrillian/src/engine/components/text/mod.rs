use crate::assets::{Font, HFont, DEFAULT_GLYPH_SIZE};
use crate::components::glyph::TextAlignment;
use crate::components::Component;
use crate::core::GameObjectId;
use crate::rendering::proxies::text_proxy::{TextDim, TextProxy, ThreeD, TwoD};
use crate::rendering::proxies::SceneProxy;
use crate::rendering::CPUDrawCtx;
use crate::World;
use delegate::delegate;
use nalgebra::{Vector2, Vector3};

pub mod glyph;
pub mod msdf_atlas;

pub type Text3D = Text<ThreeD>;
pub type Text2D = Text<TwoD>;

#[derive(Debug)]
pub struct Text<DIM: TextDim> {
    parent: GameObjectId,
    proxy: TextProxy<DIM>,
    family_name: String,
    glyph_size: i32,
    font_dirty: bool,
}

impl<DIM: TextDim> Text<DIM> {
    pub const fn text(&self) -> &TextProxy<DIM> {
        &self.proxy
    }

    pub const fn text_mut(&mut self) -> &mut TextProxy<DIM> {
        &mut self.proxy
    }

    delegate! {
        to self.proxy {
            pub fn set_text(&mut self, text: impl Into<String>);
            pub fn set_alignment(&mut self, alignment: TextAlignment);
            #[call(set_font)]
            pub fn set_font_direct(&mut self, font: HFont);
            pub const fn set_position(&mut self, x: f32, y: f32);
            pub const fn set_position_vec(&mut self, pos: Vector2<f32>);
            pub const fn set_color(&mut self, r: f32, g: f32, b: f32);
            pub const fn set_color_vec(&mut self, color: Vector3<f32>);
            pub const fn set_size(&mut self, text_size: f32);
            pub const fn set_rainbow_mode(&mut self, enable: bool);
        }
    }

    pub fn set_font(&mut self, font_family: impl Into<String>) {
        self.family_name = font_family.into();
        self.font_dirty = true;
    }
}

impl<DIM: TextDim + 'static> Component for Text<DIM> {
    fn new(parent: GameObjectId) -> Self
    where
        Self: Sized,
    {
        Self {
            parent,
            proxy: TextProxy::new("".to_string(), HFont::DEFAULT, 100.0),
            family_name: "Arial".to_string(),
            glyph_size: DEFAULT_GLYPH_SIZE,
            font_dirty: false,
        }
    }

    fn create_render_proxy(&mut self, _world: &World) -> Option<Box<dyn SceneProxy>> {
        Some(Box::new(self.proxy.clone()))
    }

    fn update_proxy(&mut self, world: &World, ctx: CPUDrawCtx) {
        if self.font_dirty {
            let font = world
                .assets
                .fonts
                .add(Font::new(self.family_name.clone(), Some(self.glyph_size)));
            self.proxy.set_font(font);
            self.font_dirty = false;
        }

        self.proxy.update_game_thread(ctx);
    }

    fn parent(&self) -> GameObjectId {
        self.parent
    }
}
