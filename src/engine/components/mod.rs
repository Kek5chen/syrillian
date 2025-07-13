mod camera;
mod collider;
mod gravity;
mod rigid_body;
mod rotate;
mod freecam;
mod light;

pub use camera::*;
pub use collider::*;
pub use gravity::*;
pub use rigid_body::*;
pub use rotate::*;
pub use freecam::*;
pub use light::*;

use crate::core::GameObjectId;
use std::any::Any;


pub trait Component: Any {
    fn new(parent: GameObjectId) -> Self
    where
        Self: Sized;
    
    // Gets called when the game object is created directly after new
    fn init(&mut self) {}
    
    // Gets called when the component should update anything state related
    fn update(&mut self) {}
    
    // Gets called when the component should update any state that's necessary for physics
    fn late_update(&mut self) {}

    // Gets called after physics have evolved
    fn post_update(&mut self) {}

    // Gets called when the component is about to be deleted
    fn delete(&mut self) {}

    #[allow(clippy::mut_from_ref)]
    fn get_parent(&self) -> GameObjectId;
}

pub(crate) trait InternalComponentDeletion {
    fn delete_internal(&mut self);
}

impl InternalComponentDeletion for dyn Component {
    fn delete_internal(&mut self) {
        self.delete();
    }
}
