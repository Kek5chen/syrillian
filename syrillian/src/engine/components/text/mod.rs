use crate::World;
use crate::assets::{DEFAULT_ATLAS_SIZE, Font, HFont};
use crate::components::Component;
use crate::core::GameObjectId;
use crate::rendering::CPUDrawCtx;
use crate::rendering::glyph::TextAlignment;
use crate::rendering::proxies::SceneProxy;
use crate::rendering::proxies::text_proxy::{TextDim, TextProxy, ThreeD, TwoD};
use delegate::delegate;
use nalgebra::{Vector2, Vector3};

pub type Text3D = Text<3, ThreeD>;
pub type Text2D = Text<2, TwoD>;

#[derive(Debug)]
pub struct Text<const D: u8, DIM: TextDim<D>> {
    parent: GameObjectId,
    proxy: TextProxy<D, DIM>,
    family_name: String,
    glyph_size: u32,
    font_dirty: bool,
}

impl<const D: u8, DIM: TextDim<D>> Text<D, DIM> {
    pub const fn text(&self) -> &TextProxy<D, DIM> {
        &self.proxy
    }

    pub const fn text_mut(&mut self) -> &mut TextProxy<D, DIM> {
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

impl<const D: u8, DIM: TextDim<D> + 'static> Component for Text<D, DIM> {
    fn new(parent: GameObjectId) -> Self
    where
        Self: Sized,
    {
        Self {
            parent,
            proxy: TextProxy::new("".to_string(), HFont::DEFAULT, 100.0),
            family_name: "Arial".to_string(),
            glyph_size: DEFAULT_ATLAS_SIZE,
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
