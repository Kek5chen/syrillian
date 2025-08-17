use std::any::Any;
use std::collections::HashSet;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};

use crate::components::{CRef, Component, TypedComponentId};
use crate::drawables::drawable::Drawable;
use crate::world::World;
use crate::{c_any_mut, ensure_aligned};
use itertools::Itertools;
use nalgebra::{Matrix4, Translation3, Vector3};
use slotmap::{new_key_type, Key, KeyData};

use crate::components::InternalComponentDeletion;
use crate::core::Transform;

new_key_type! { pub struct GameObjectId; }

#[allow(dead_code)]
impl GameObjectId {
    const INVALID_VALUE: u64 = 0x0000_0001_ffff_ffff;
    pub fn exists(&self) -> bool {
        !self.is_null() && World::instance().objects.contains_key(*self)
    }

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

pub struct GameObject {
    pub id: GameObjectId,
    pub name: String,
    pub(crate) children: Vec<GameObjectId>,
    pub(crate) parent: Option<GameObjectId>,
    pub transform: Transform,
    pub(crate) drawable: Option<Box<dyn Drawable>>,
    pub(crate) components: HashSet<TypedComponentId>,
}

impl GameObject {
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
            let world = World::instance();
            if let Some(pos) = world.children.iter().find_position(|other| self.id == **other) {
                world.children.remove(pos.0);
            }
        }
    }

    pub fn add_child(&mut self, mut child: GameObjectId) {
        // unlink from previous parent or world
        child.unlink();

        self.children.push(child);
        child.parent = Some(self.id);
    }

    pub fn set_drawable(&mut self, drawable: impl Drawable) {
        self.set_drawable_box(Box::new(drawable));
    }

    #[inline]
    pub fn set_drawable_box(&mut self, drawable: Box<dyn Drawable>) {
        self.drawable = Some(drawable);
    }

    pub fn remove_drawable(&mut self) {
        self.drawable = None;
    }

    pub fn drawable<D: Drawable>(&self) -> Option<&D> {
        let drawable = self.drawable.as_ref()?.as_ref();
        Some((drawable as &dyn Any).downcast_ref::<D>()?)
    }

    pub fn drawable_mut<D: Drawable>(&mut self) -> Option<&mut D> {
        let drawable = self.drawable.as_mut()?.as_mut();
        Some((drawable as &mut dyn Any).downcast_mut::<D>()?)
    }

    pub fn add_component<'b, C>(&mut self) -> CRef<C>
    where
        C: Component + 'static,
    {
        let world = World::instance();
        let mut comp: C = C::new(self.id);
        comp.init(world);

        let id = world.components.add(comp);
        self.components.insert(id.into());
        id
    }

    pub fn add_child_components<C>(&mut self)
    where
        C: Component + 'static,
    {
        for child in &mut self.children {
            child.add_component::<C>();
        }
    }

    pub fn add_child_components_then<C>(&mut self, f: impl Fn(&mut C))
    where
        C: Component + 'static,
    {
        for child in &mut self.children {
            let mut comp = child.add_component::<C>();
            f(&mut comp);
        }
    }

    pub fn get_component<C: Component + 'static>(&self) -> Option<CRef<C>> {
        self.components
            .iter()
            .find_map(|c| c.as_a::<C>())
    }

    pub fn get_components<C: Component + 'static>(&self) -> impl Iterator<Item=CRef<C>> {
        self.components
            .iter()
            .filter_map(|c| c.as_a())
    }

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

    pub fn remove_component(&mut self, comp: impl Into<TypedComponentId>, world: &mut World) {
        let comp = comp.into();
        self.components.remove(&comp);
        world.components.remove(comp);
    }

    pub fn parent(&self) -> &Option<GameObjectId> {
        &self.parent
    }

    pub fn children(&self) -> &[GameObjectId] {
        &self.children
    }

    pub fn delete(&mut self) {
        for mut child in self.children.iter().copied() {
            child.delete();
        }

        let world = World::instance();
        for typed in self.components.drain() {
            if let Some(comp) = c_any_mut!(typed) {
                comp.delete_internal(world);
                world.components.remove(typed);
            }
        }

        self.children.clear();

        world.unlink_internal(self.id);
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

    pub fn from_translation(translation: Translation3<f32>) -> Self {
        ModelUniform {
            model_mat: translation.to_homogeneous(),
        }
    }

    pub fn update(&mut self, transform: &Matrix4<f32>) {
        self.model_mat = *transform;
    }
}
