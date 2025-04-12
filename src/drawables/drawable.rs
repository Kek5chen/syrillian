use std::any::Any;

use nalgebra::Matrix4;
use wgpu::{Device, Queue, RenderPass};

use crate::object::GameObjectId;
use crate::world::World;

pub trait Drawable: Any {
    fn setup(
        &mut self,
        device: &Device,
        queue: &Queue,
        world: &mut World,
    );
    fn update(
        &mut self,
        world: &mut World,
        parent: GameObjectId,
        queue: &Queue,
        outer_transform: &Matrix4<f32>,
    );
    fn draw(&self, world: &mut World, rpass: &mut RenderPass);
}
