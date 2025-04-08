use std::any::Any;

#[allow(unused_imports)]
pub use camera::CameraComp;
#[allow(unused_imports)]
pub use collider::Collider3D;
#[allow(unused_imports)]
pub use gravity::GravityComp;
#[allow(unused_imports)]
pub use rigid_body::RigidBodyComponent;
#[allow(unused_imports)]
pub use rotate::RotateComponent;
#[allow(unused_imports)]
pub use freecam::FreecamController;

use crate::object::GameObjectId;

pub mod camera;
pub mod collider;
pub mod gravity;
pub mod rigid_body;
pub mod rotate;
pub mod freecam;

// TODO: resolve unsafe hell
pub trait Component: Any {
    unsafe fn new(parent: GameObjectId) -> Self
    where
        Self: Sized;
    
    // Gets called when the game object is created directly after new
    unsafe fn init(&mut self) {}
    
    // Gets called when the component should update anything state related
    unsafe fn update(&mut self) {}
    
    // Gets called when the component should update any state that's necessary for physics
    unsafe fn late_update(&mut self) {}

    // Gets called after physics have evolved
    unsafe fn post_update(&mut self) {}

    // Gets called when the component is about to be deleted
    unsafe fn delete(&mut self) {}

    #[allow(clippy::mut_from_ref)]
    unsafe fn get_parent(&self) -> GameObjectId;
}

pub(crate) trait InternalComponentDeletion {
    unsafe fn delete_internal(&mut self);
}

impl InternalComponentDeletion for dyn Component {
    unsafe fn delete_internal(&mut self) {
        unsafe {
            self.delete();
        }
    }
}
