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

pub mod animation;
pub mod audio;
pub mod camera;
pub mod collider;
pub mod fp_camera;
pub mod fp_movement;
pub mod freecam;
pub mod gravity;
pub mod image;
pub mod light;
pub mod mesh_renderer;
pub mod rigid_body;
pub mod rope;
pub mod rotate;
pub mod skeletal;
pub mod spring;
pub mod text;

#[cfg(debug_assertions)]
pub mod camera_debug;

pub use animation::*;
pub use camera::*;
pub use collider::*;
pub use fp_camera::*;
pub use fp_movement::*;
pub use freecam::*;
pub use gravity::*;
pub use image::*;
pub use light::*;
pub use mesh_renderer::*;
pub use rigid_body::*;
pub use rope::*;
pub use rotate::*;
pub use skeletal::*;
pub use spring::*;
pub use text::*;

#[cfg(debug_assertions)]
pub use camera_debug::*;

use crate::World;
use crate::core::GameObjectId;
use crate::rendering::CPUDrawCtx;
use crate::rendering::lights::LightProxy;
use crate::rendering::proxies::SceneProxy;
use delegate::delegate;
use slotmap::{Key, new_key_type};
use std::any::{Any, TypeId};
use std::borrow::Borrow;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::mem;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

new_key_type! { pub struct ComponentId; }

pub struct CRef<C: Component + ?Sized>(pub(crate) Option<Arc<C>>, pub(crate) TypedComponentId);

impl<C: Component + ?Sized> Clone for CRef<C> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1)
    }
}

impl<C: Component> Deref for CRef<C> {
    type Target = C;

    fn deref(&self) -> &Self::Target {
        unsafe { self.0.as_ref().unwrap_unchecked() }
    }
}

impl<C: Component> DerefMut for CRef<C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(Arc::as_ptr(self.0.as_ref().unwrap_unchecked()) as *mut _) }
        // unsafe { &mut *(&raw const *self.0.assume_init_ref() as *mut _) }
    }
}

impl<C: Component + ?Sized> Hash for CRef<C> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.typed_id().hash(state)
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

    pub fn into_dyn(self) -> CRef<dyn Component> {
        unsafe {
            CRef(
                Some(self.0.as_ref().unwrap_unchecked().clone() as Arc<dyn Component>),
                self.1,
            )
        }
    }

    /// # SAFETY
    ///
    /// This is uninitialized territory. If you use this, you'll need to make sure to
    /// overwrite it before using it. Accessing this in any way is UB.
    ///
    /// The only reason this exists is that you can save References for components which
    /// are also managed by a component so you can avoid Option. It's not recommended to
    /// use this.
    pub unsafe fn null() -> CRef<C> {
        unsafe { CRef(None, TypedComponentId::null::<C>()) }
    }
}

impl<C: Component + ?Sized> CRef<C> {
    pub fn is_a<O: Component>(&self) -> bool {
        self.1.0 == TypeId::of::<O>()
    }

    pub fn typed_id(&self) -> TypedComponentId {
        self.1
    }
}

impl CRef<dyn Component> {
    pub fn as_a<C: Component>(&self) -> Option<CRef<C>> {
        if !self.is_a::<C>() {
            return None;
        }
        let downcasted =
            Arc::downcast::<C>(unsafe { self.0.as_ref().unwrap_unchecked() }.clone()).ok()?;
        Some(CRef(Some(downcasted), self.1))
    }
}

impl Deref for CRef<dyn Component> {
    type Target = dyn Component;

    fn deref(&self) -> &Self::Target {
        unsafe { self.0.as_ref().unwrap_unchecked().as_ref() }
    }
}

impl DerefMut for CRef<dyn Component> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(&raw const **self.0.as_ref().unwrap_unchecked() as *mut _) }
    }
}

impl<C: Component + ?Sized> From<CRef<C>> for CWeak<C> {
    fn from(value: CRef<C>) -> Self {
        CWeak(value.1.1, PhantomData)
    }
}

impl<C: ?Sized + Component> PartialEq<Self> for CRef<C> {
    fn eq(&self, other: &Self) -> bool {
        self.typed_id() == other.typed_id()
    }
}

impl<C: ?Sized + Component> Eq for CRef<C> {}

impl<C: Component> Debug for CRef<C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Component").finish()
    }
}

impl<C: Component + ?Sized> Borrow<TypedComponentId> for CRef<C> {
    fn borrow(&self) -> &TypedComponentId {
        &self.1
    }
}

impl<C: Component + ?Sized> Borrow<TypedComponentId> for &CRef<C> {
    fn borrow(&self) -> &TypedComponentId {
        &self.1
    }
}

pub struct CWeak<C: Component + ?Sized>(pub(crate) ComponentId, pub(crate) PhantomData<C>);

impl<C: Component> Clone for CWeak<C> {
    fn clone(&self) -> Self {
        *self
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
        world.components.get::<C>(self.0).cloned()
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

impl TypedComponentId {
    pub fn is_a<C: Component>(&self) -> bool {
        self.0 == TypeId::of::<C>()
    }

    pub fn type_id(&self) -> TypeId {
        self.0
    }

    pub(crate) fn null<C: Component>() -> TypedComponentId {
        Self::from_typed::<C>(ComponentId::null())
    }

    pub(crate) fn from_typed<C: Component + ?Sized>(id: ComponentId) -> Self {
        TypedComponentId(TypeId::of::<C>(), id)
    }
}

/// A component attached to [`GameObject`](crate::core::GameObject).
///
/// Typical components include `Collider3D`, `MeshRenderer`, `AudioEmitter`, etc.
/// Can also be used to create custom game logic.
///
/// # Examples
///
/// ```rust
/// use nalgebra::Vector3;
/// use syrillian::World;
/// use syrillian::components::Component;
/// use syrillian::core::GameObjectId;
///
/// struct MyComponent {
///     parent: GameObjectId,
/// }
///
/// impl Component for MyComponent {
///     fn new(parent: GameObjectId) -> Self
///     where
///         Self: Sized,
///     {
///         Self { parent }
///     }
///
///     fn init(&mut self, _world: &mut World) {
///         // Sets trasnlate for parent GameObject on its init
///         self.parent.transform.translate(Vector3::new(1.0, 0.0, 0.0));
///     }
///
///     fn parent(&self) -> GameObjectId {
///         self.parent
///     }
/// }
///```
#[allow(unused)]
pub trait Component: Any + Send + Sync {
    fn new(parent: GameObjectId) -> Self
    where
        Self: Sized;

    // Gets called when the game object is created directly after new
    fn init(&mut self, world: &mut World) {}

    // Gets called when the component should update anything state-related
    fn update(&mut self, world: &mut World) {}

    // Gets called when the component should update any state that's necessary for physics
    fn late_update(&mut self, world: &mut World) {}

    // Gets called before physics are evolved
    fn pre_fixed_update(&mut self, world: &mut World) {}

    // Gets called after physics have evolved
    fn fixed_update(&mut self, world: &mut World) {}

    // Gets called after all other updates are done
    fn post_update(&mut self, world: &mut World) {}

    fn create_render_proxy(&mut self, world: &World) -> Option<Box<dyn SceneProxy>> {
        None
    }

    fn create_light_proxy(&mut self, world: &World) -> Option<Box<LightProxy>> {
        None
    }

    fn update_proxy(&mut self, world: &World, ctx: CPUDrawCtx) {}

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
