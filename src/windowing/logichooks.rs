use std::error::Error;
use winit::window::Window;

use crate::world::World;

pub type HookFunc = fn(world: &mut World, window: &Window) -> Result<(), Box<dyn Error>>;

pub struct LogicHooks {
    pub init: Option<HookFunc>,
    pub update: Option<HookFunc>,
    pub deinit: Option<HookFunc>,
}
