use std::any::Any;

use nalgebra::Matrix4;
use wgpu::RenderPass;

use crate::asset_management::ShaderId;
use crate::core::GameObjectId;
use crate::rendering::Renderer;
use crate::world::World;

#[allow(unused_variables)]
pub trait Drawable: Any {
    fn setup(
        &mut self,
        renderer: &Renderer,
        world: &mut World,
    ) {}
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
        rpass: &mut RenderPass,
        renderer: &Renderer,
        shader_override: Option<ShaderId>,
    );
}
