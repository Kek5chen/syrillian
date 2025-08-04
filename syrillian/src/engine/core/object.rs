use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::mem;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

use itertools::Itertools;
use nalgebra::Matrix4;

use crate::components::Component;
use crate::drawables::drawable::Drawable;
use crate::world::World;
use crate::{ensure_aligned, utils};

use crate::components::InternalComponentDeletion;
use crate::core::Transform;

#[derive(Copy, Clone, Eq, Ord, PartialOrd, PartialEq, Hash)]
#[repr(transparent)]
pub struct GameObjectId(pub(crate) usize);

impl Debug for GameObjectId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.exists() {
            write!(f, "GameObject: {}", self.name)
        } else {
            f.write_str("Invalid GameObjectId")
        }
    }
}

#[allow(dead_code)]
impl GameObjectId {
    pub fn exists(&self) -> bool {
        !self.is_invalid() && World::instance().objects.contains_key(self)
    }

    pub fn invalid() -> GameObjectId {
        GameObjectId(usize::MAX)
    }

    pub fn is_invalid(&self) -> bool {
        self.0 == usize::MAX
    }

    pub fn tap<F: Fn(&mut GameObject)>(mut self, f: F) -> Self {
        if self.exists() {
            f(self.deref_mut())
        }
        self
    }
}

// USING and STORING a GameObjectId is like a contract. It defines that you will recheck the
// existance of this game object every time you re-use it. Otherwise you will crash.
impl Deref for GameObjectId {
    type Target = GameObject;

    fn deref(&self) -> &GameObject {
        World::instance().get_object(self).unwrap()
    }
}

impl DerefMut for GameObjectId {
    fn deref_mut(&mut self) -> &mut Self::Target {
        World::instance().get_object_mut(self).unwrap()
    }
}

pub struct GameObject {
    pub id: GameObjectId,
    pub name: String,
    pub(crate) children: Vec<GameObjectId>,
    pub(crate) parent: Option<GameObjectId>,
    pub transform: Transform,
    pub(crate) drawable: Option<Box<dyn Drawable>>,
    pub(crate) components: Vec<Rc<RefCell<Box<dyn Component>>>>,
}

impl GameObject {
    pub fn unlink(&mut self) {
        if let Some(mut parent) = self.parent.take() {
            let pos_opt = parent
                .children
                .iter()
                .find_position(|other| self.id.0 == other.0)
                .map(|(id, _)| id);
            if let Some(pos) = pos_opt {
                parent.children.remove(pos);
            }
        }
    }

    pub fn add_child(&mut self, mut child: GameObjectId) {
        // if child had a parent, remove it from there
        child.unlink();

        self.children.push(child);
        child.parent = Some(self.id);
    }

    pub fn set_drawable(&mut self, drawable: Box<dyn Drawable>) {
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

    pub fn add_component<'b, C>(&mut self) -> &'b mut C
    where
        C: Component + 'static,
    {
        unsafe {
            let mut comp: Box<dyn Component> = Box::new(C::new(self.id));
            let comp_inner_ptr: utils::FatPtr<C> =
                mem::transmute(comp.as_mut() as *mut dyn Component);
            let comp_inner_ref: &mut C = &mut *comp_inner_ptr.data;

            comp.init();

            let comp: Rc<RefCell<Box<dyn Component>>> = Rc::new(RefCell::new(comp));
            let comp_dyn: Rc<RefCell<Box<dyn Component>>> = comp;

            self.components.push(comp_dyn);

            comp_inner_ref
        }
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
            let comp = child.add_component::<C>();
            f(comp);
        }
    }

    // FIXME: this works for now but is stupidly fucked up.
    //   only change this if entity ids are used for Components in the future :>>
    pub fn get_component<C: Component + 'static>(&self) -> Option<Rc<RefCell<Box<C>>>> {
        for component in &self.components {
            let raw_ptr: *const Box<dyn Component> = component.as_ptr();
            let type_id = unsafe { (**raw_ptr).type_id() };

            if type_id == TypeId::of::<C>() {
                return Some(unsafe {
                    let rc_clone = Rc::clone(component);
                    mem::transmute::<Rc<RefCell<Box<dyn Component>>>, Rc<RefCell<Box<C>>>>(rc_clone)
                });
            }
        }
        None
    }

    // FIXME: The thing above also counts here
    pub fn get_components<C: Component + 'static>(&self) -> Vec<Rc<RefCell<Box<C>>>> {
        let mut components = Vec::new();
        for component in &self.components {
            let raw_ptr: *const Box<dyn Component> = component.as_ptr();
            let type_id = unsafe { (**raw_ptr).type_id() };

            if type_id == TypeId::of::<C>() {
                unsafe {
                    let rc_clone = Rc::clone(component);
                    let component = mem::transmute::<
                        Rc<RefCell<Box<dyn Component>>>,
                        Rc<RefCell<Box<C>>>,
                    >(rc_clone);
                    components.push(component);
                }
            }
        }
        components
    }

    pub fn get_child_component<C>(&mut self) -> Option<Rc<RefCell<Box<C>>>>
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

    pub fn parent(&self) -> &Option<GameObjectId> {
        &self.parent
    }

    pub fn children(&self) -> &[GameObjectId] {
        &self.children
    }

    pub fn delete(&mut self) {
        for child in &mut self.children {
            child.delete();
        }

        for comp in self.components.drain(..) {
            let mut comp = comp.borrow_mut();
            comp.delete_internal();
        }

        self.children.clear();

        World::instance().unlink_internal(self.id);
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

    pub fn update(&mut self, object: GameObjectId, outer_transform: &Matrix4<f32>) {
        self.model_mat = outer_transform * object.transform.full_matrix().to_homogeneous();
    }
}
