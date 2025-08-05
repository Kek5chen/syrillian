//! Built-in components that can be attached to [`GameObject`](crate::core::GameObject).
//!
//! Components implement behavior ranging from camera control to physics. If it's dynamic,
//! it's probably a component. (Only Rendering is done in [`Drawable`](crate::drawables::Drawable)s)
//!
//! To make a component:
//! ```rust
//! use nalgebra::Vector3;
//! use syrillian::components::Component;
//! use syrillian::core::GameObjectId;
//! use syrillian::World;
//!
//! pub struct Gravity {
//!     force: f32,
//!     parent: GameObjectId, // the parent needs to be stored
//! }
//!
//! impl Component for Gravity {
//!     fn new(parent: GameObjectId) -> Self {
//!         Gravity {
//!             force: 8.91,
//!             parent,
//!         }
//!     }
//!
//!     fn update(&mut self) {
//!         let delta_time = World::instance().delta_time().as_secs_f32();
//!
//!         let movement = Vector3::new(0.0, self.force * delta_time, 0.0);
//!
//!         let transform = &mut self.parent().transform;
//!         transform.translate(movement);
//!     }
//!
//!     fn parent(&self) -> GameObjectId {
//!         self.parent
//!     }
//! }
//! ```

pub mod camera;
pub mod collider;
pub mod fp_camera;
pub mod fp_movement;
pub mod freecam;
pub mod gravity;
pub mod light;
pub mod rigid_body;
pub mod rope;
pub mod rotate;
pub mod audio;

pub use camera::*;
pub use collider::*;
pub use fp_camera::*;
pub use fp_movement::*;
pub use freecam::*;
pub use gravity::*;
pub use light::*;
pub use rigid_body::*;
pub use rope::*;
pub use rotate::*;
pub use audio::*;

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
    fn parent(&self) -> GameObjectId;
}

pub(crate) trait InternalComponentDeletion {
    fn delete_internal(&mut self);
}

impl InternalComponentDeletion for dyn Component {
    fn delete_internal(&mut self) {
        self.delete();
    }
}
