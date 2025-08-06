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
//!     fn update(&mut self, world: &mut World) {
//!         let delta_time = world.delta_time().as_secs_f32();
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
pub mod spring;
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
pub use spring::*;
pub use audio::*;

use crate::core::GameObjectId;
use crate::World;
use delegate::delegate;
use slotmap::{new_key_type, Key};
use std::any::{Any, TypeId};
use std::fmt::{Debug, Formatter};
use std::hash::Hash;
use std::marker::PhantomData;
use std::mem;
use std::ops::{Deref, DerefMut};

new_key_type! { pub struct ComponentId; }

#[macro_export]
macro_rules! c {
    ($id:expr) => {
        $crate::World::instance().components.get($id)
    };
}

#[macro_export]
macro_rules! c_mut {
    ($id:expr) => {
        $crate::World::instance().components.get_mut($id)
    };
}

#[macro_export]
macro_rules! c_any {
    ($id:expr) => {
        $crate::World::instance().components.get_dyn($id)
    };
}

#[macro_export]
macro_rules! c_any_mut {
    ($id:expr) => {
        $crate::World::instance().components.get_dyn_mut($id)
    };
}

pub struct CRef<C: Component>(pub(crate) ComponentId, pub(crate) PhantomData<C>);

impl<C: Component> Clone for CRef<C> {
    fn clone(&self) -> Self {
        CRef(self.0, self.1)
    }
}

impl<C: Component> Copy for CRef<C> {}

impl<C: Component> Deref for CRef<C> {
    type Target = C;

    fn deref(&self) -> &Self::Target {
        World::instance()
            .components
            .get(CRef(self.0, PhantomData))
            .unwrap()
    }
}

impl<C: Component> DerefMut for CRef<C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        World::instance()
            .components
            .get_mut(CRef(self.0, PhantomData))
            .unwrap()
    }
}

impl<C: Component> From<CRef<C>> for CWeak<C> {
    fn from(value: CRef<C>) -> Self {
        CWeak(value.0, value.1)
    }
}

#[allow(unused)]
impl<C: Component> CRef<C> {
    pub(crate) fn forget_lifetime(mut self) -> &'static mut C {
        unsafe { mem::transmute(self.deref_mut()) }
    }

    pub fn downgrade(self) -> CWeak<C> {
        self.into()
    }

    pub fn null() -> CRef<C> {
        CRef(ComponentId::null(), PhantomData)
    }

    delegate! {
        to self.0 {
            fn is_null(&self) -> bool;
        }
    }
}

impl<C: Component> Default for CRef<C> {
    fn default() -> Self {
        CRef(ComponentId::default(), PhantomData)
    }
}

impl<C: Component> PartialEq<Self> for CRef<C> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<C: Component> Eq for CRef<C> {}

impl<C: Component> Debug for CRef<C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Component").finish()
    }
}

pub struct CWeak<C: Component>(pub(crate) ComponentId, pub(crate) PhantomData<C>);

impl<C: Component> Clone for CWeak<C> {
    fn clone(&self) -> Self {
        CWeak(self.0, self.1)
    }
}

impl<C: Component> Copy for CWeak<C> {}

impl<C: Component> PartialEq<Self> for CWeak<C> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<C: Component> Eq for CWeak<C> {}

impl<C: Component> Debug for CWeak<C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Weak Component").finish()
    }
}

#[allow(unused)]
impl<C: Component> CWeak<C> {
    pub fn exists(&self, world: &World) -> bool {
        world
            .components
            ._get::<C>()
            .map(|c| c.contains_key(self.0))
            .unwrap_or(false)
    }
    pub fn upgrade(&self, world: &World) -> Option<CRef<C>> {
        self.exists(world).then(|| CRef(self.0, self.1))
    }

    pub fn null() -> CWeak<C> {
        CWeak(ComponentId::null(), PhantomData)
    }

    delegate! {
        to self.0 {
            fn is_null(&self) -> bool;
        }
    }
}

impl<C: Component> Default for CWeak<C> {
    fn default() -> Self {
        CWeak(ComponentId::default(), PhantomData)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct TypedComponentId(pub(crate) TypeId, pub(crate) ComponentId);

impl From<TypedComponentId> for ComponentId {
    fn from(value: TypedComponentId) -> Self {
        value.1
    }
}

impl<C: Component> From<CRef<C>> for TypedComponentId {
    fn from(value: CRef<C>) -> Self {
        TypedComponentId(TypeId::of::<C>(), value.0)
    }
}

impl TypedComponentId {
    pub fn is_a<C: Component>(&self) -> bool {
        self.0 == TypeId::of::<C>()
    }

    pub fn as_a<C: Component>(&self) -> Option<CRef<C>> {
        self.is_a::<C>().then(|| CRef(self.1, PhantomData))
    }

    pub fn type_id(&self) -> TypeId {
        self.0
    }
}

#[allow(unused)]
pub trait Component: Any {
    fn new(parent: GameObjectId) -> Self
    where
        Self: Sized;

    // Gets called when the game object is created directly after new
    fn init(&mut self, world: &mut World) {}

    // Gets called when the component should update anything state related
    fn update(&mut self, world: &mut World) {}

    // Gets called when the component should update any state that's necessary for physics
    fn late_update(&mut self, world: &mut World) {}

    // Gets called after physics have evolved
    fn post_update(&mut self, world: &mut World) {}

    // Gets called when the component is about to be deleted
    fn delete(&mut self, world: &mut World) {}

    #[allow(clippy::mut_from_ref)]
    fn parent(&self) -> GameObjectId;
}

pub(crate) trait InternalComponentDeletion {
    fn delete_internal(&mut self, world: &mut World);
}

impl InternalComponentDeletion for dyn Component {
    fn delete_internal(&mut self, world: &mut World) {
        self.delete(world);
    }
}
