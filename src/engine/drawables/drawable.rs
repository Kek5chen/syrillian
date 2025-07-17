use std::any::Any;

use crate::core::GameObjectId;
use crate::engine::rendering::DrawCtx;
use crate::rendering::Renderer;
use crate::world::World;
use nalgebra::Matrix4;

#[allow(unused_variables)]
#[rustfmt::skip]
pub trait Drawable: Any {
    fn setup(&mut self, renderer: &Renderer, world: &mut World) {}
    fn update(
        &mut self,
        world: &mut World,
        parent: GameObjectId,
        renderer: &Renderer,
        outer_transform: &Matrix4<f32>,
    ) {}
    fn draw(
        &self,
        world: &mut World,
        ctx: &DrawCtx,
    );
}
