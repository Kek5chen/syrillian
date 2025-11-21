use crate::components::{CRef, Component, NewComponent, TypedComponentId};
use crate::ensure_aligned;
use crate::world::World;
use itertools::Itertools;
use nalgebra::{Matrix4, Translation3, Vector3};
use slotmap::{Key, KeyData, new_key_type};
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};

use crate::components::InternalComponentDeletion;
use crate::core::Transform;

new_key_type! {
    /// Uniquely identifies a game object within the world.
    pub struct GameObjectId;
}

#[allow(dead_code)]
impl GameObjectId {
    const INVALID_VALUE: u64 = 0x0000_0001_ffff_ffff;
    /// Returns `true` if `self` is non-null and is contained within the [`World`] instance.
    pub fn exists(&self) -> bool {
        !self.is_null() && World::instance().objects.contains_key(*self)
    }

    /// Chaining method that applies the function `f` to `self`.
    pub fn tap<F: Fn(&mut GameObject)>(mut self, f: F) -> Self {
        if self.exists() {
            f(self.deref_mut())
        }
        self
    }

    pub(crate) fn as_ffi(&self) -> u64 {
        self.0.as_ffi()
    }

    pub(crate) fn from_ffi(id: u64) -> GameObjectId {
        GameObjectId(KeyData::from_ffi(id))
    }
}

// USING and STORING a GameObjectId is like a contract. It defines that you will recheck the
//  existence of this game object every time you re-use it. Otherwise, you will crash.
impl Deref for GameObjectId {
    type Target = GameObject;

    fn deref(&self) -> &GameObject {
        World::instance().objects.get(*self).unwrap()
    }
}

impl DerefMut for GameObjectId {
    fn deref_mut(&mut self) -> &mut Self::Target {
        World::instance().objects.get_mut(*self).unwrap()
    }
}

/// Structure representing an object tree within the world.
///
/// A game object has a unique identifier and a non-unique name.
/// It keeps track of its parent-child relationships, applied
/// transformation, and attached components. If a game object has
/// no parent it is a root-level game object within the world.
pub struct GameObject {
    /// A unique identifier for this object within the world.
    pub id: GameObjectId,
    /// The name of the object (not required to be unique).
    pub name: String,
    /// Game objects that are direct children of this object.
    pub(crate) children: Vec<GameObjectId>,
    /// Parent game object.
    /// If `None`, this object is a root-level game object.
    pub(crate) parent: Option<GameObjectId>,
    /// The world this object belongs to
    pub(crate) owning_world: *mut World,
    /// The transformation applied to the object.
    pub transform: Transform,
    /// Components attached to this object.
    pub(crate) components: HashSet<CRef<dyn Component>>,
    /// Custom Property Data (Keys & Values)
    pub(crate) custom_properties: HashMap<String, serde_json::Value>,
}

impl GameObject {
    /// Unlinks this game object from its parent or the world (root level).
    pub fn unlink(&mut self) {
        if let Some(mut parent) = self.parent.take() {
            let pos_opt = parent
                .children
                .iter()
                .find_position(|other| self.id == **other)
                .map(|(id, _)| id);
            if let Some(pos) = pos_opt {
                parent.children.remove(pos);
            }
        } else {
            let world = self.world();
            if let Some(pos) = world
                .children
                .iter()
                .find_position(|other| self.id == other.id)
            {
                world.children.remove(pos.0);
            }
        }
    }

    /// Adds another game object as a child of this one, replacing the child's previous parent relationship.
    pub fn add_child(&mut self, mut child: GameObjectId) {
        // unlink from previous parent or world
        child.unlink();

        self.children.push(child);
        child.parent = Some(self.id);
    }

    /// Adds a new [`Component`] of type `C` to this game object, initializing the component within the world,
    /// and returns the component ID.
    pub fn add_component<C>(&mut self) -> CRef<C>
    where
        C: NewComponent + 'static,
    {
        let world = self.world();
        let mut comp: C = C::new(self.id);
        comp.init(world);

        let new_comp = world.components.add(comp, self.id);
        let new_comp2 = new_comp.clone();
        self.components.insert(new_comp.into_dyn());
        new_comp2
    }

    /// Adds a new [`Component`] of type `C` to all children of this game object.
    pub fn add_child_components<C>(&mut self)
    where
        C: NewComponent + 'static,
    {
        for child in &mut self.children {
            child.add_component::<C>();
        }
    }

    /// Adds a new [`Component`] of type `C` to all children of this game object, and applies the provided
    /// function `f` to each newly added component.
    pub fn add_child_components_then<C>(&mut self, f: impl Fn(&mut C))
    where
        C: NewComponent + 'static,
    {
        for child in &mut self.children {
            let mut comp = child.add_component::<C>();
            f(&mut comp);
        }
    }

    /// Add a custom property to this object
    pub fn add_property(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.custom_properties.insert(key.into(), value);
    }

    /// Add a collection of custom properties to this object
    pub fn add_properties<T: IntoIterator<Item = (String, serde_json::Value)>>(
        &mut self,
        properties: T,
    ) {
        self.custom_properties.extend(properties);
    }

    /// Retrieve a custom property in this object by the given key
    pub fn property(&self, key: &str) -> Option<&serde_json::Value> {
        self.custom_properties.get(key)
    }

    /// Checks if the object has a property with the given key
    pub fn has_property(&self, key: &str) -> bool {
        self.custom_properties.contains_key(key)
    }

    /// Retrieve all custom properties of this object
    pub fn properties(&self) -> &HashMap<String, serde_json::Value> {
        &self.custom_properties
    }

    /// Remove property from this object by the given key
    pub fn remove_property(&mut self, key: &str) -> Option<serde_json::Value> {
        self.custom_properties.remove(key)
    }

    /// Remove property from this object by the given key
    pub fn clear_properties(&mut self) {
        self.custom_properties.clear();
    }

    /// Retrieves the first found [`Component`] of type `C` attached to this game object.
    pub fn get_component<C: Component + 'static>(&self) -> Option<CRef<C>> {
        self.components.iter().find_map(|c| c.as_a::<C>())
    }

    /// Returns an iterator over all [`Component`] of type `C` attached to this game object.
    pub fn get_components<C: Component + 'static>(&self) -> impl Iterator<Item = CRef<C>> {
        self.components.iter().filter_map(|c| c.clone().as_a())
    }

    /// Retrieves the first found [`Component`] of type `C` attached to a child of this game object.
    pub fn get_child_component<C>(&mut self) -> Option<CRef<C>>
    where
        C: Component + 'static,
    {
        for child in &mut self.children {
            if let Some(comp) = child.get_component::<C>() {
                return Some(comp);
            }
        }

        None
    }

    /// Removes a [`Component`] by id from this game object and the world.
    pub fn remove_component(&mut self, comp: impl Borrow<TypedComponentId>, world: &mut World) {
        let comp = *comp.borrow();
        self.components.remove(&comp);
        world.components.remove(comp);
    }

    /// Returns an immutable reference to this game object's parent ID.
    pub fn parent(&self) -> &Option<GameObjectId> {
        &self.parent
    }

    /// Returns an immutable slice of this game object's child IDs.
    pub fn children(&self) -> &[GameObjectId] {
        &self.children
    }

    /// Destroys this game object tree, cleaning up any component-specific data,
    /// then unlinks and removes the object from the world.
    pub fn delete(&mut self) {
        for mut child in self.children.iter().copied() {
            child.delete();
        }

        let world = self.world();
        for mut comp in self.components.drain() {
            comp.delete_internal(world);
            world.components.remove(&comp);
        }

        self.children.clear();

        world.unlink_internal(self.id);
    }

    pub fn world(&self) -> &'static mut World {
        unsafe { &mut *self.owning_world }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelUniform {
    pub model_mat: Matrix4<f32>,
}

ensure_aligned!(ModelUniform { model_mat }, align <= 16 * 4 => size);

impl ModelUniform {
    pub fn empty() -> Self {
        ModelUniform {
            model_mat: Matrix4::identity(),
        }
    }

    pub fn new_at(x: f32, y: f32, z: f32) -> Self {
        ModelUniform {
            model_mat: Translation3::new(x, y, z).to_homogeneous(),
        }
    }

    pub fn new_at_vec(pos: Vector3<f32>) -> Self {
        ModelUniform {
            model_mat: Translation3::from(pos).to_homogeneous(),
        }
    }

    pub fn from_matrix(translation: &Matrix4<f32>) -> Self {
        ModelUniform {
            model_mat: *translation,
        }
    }

    pub fn update(&mut self, transform: &Matrix4<f32>) {
        self.model_mat = *transform;
    }
}
