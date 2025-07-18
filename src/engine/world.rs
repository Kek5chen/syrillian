//! The [`World`] struct stores and updates all game objects. Its use is to manage any
//! "raw" data, store and provide access to the objects and behavior in an intended
//! way, with a focus on ease of use.
//!
//! It maintains the scene graph, input state and physics simulation and
//! offers utility such as methods to create, find and remove game objects.

use crate::components::Component;
use crate::core::{GameObject, GameObjectId, Transform};
use crate::engine::assets::AssetStore;
use crate::engine::prefabs::prefab::Prefab;
use crate::engine::rendering::Renderer;
use crate::input::InputManager;
use crate::physics::PhysicsSimulator;
use crate::prefabs::CameraPrefab;
use itertools::Itertools;
use log::info;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

static mut G_WORLD: *mut World = std::ptr::null_mut();

/// Central structure representing the running scene.
///
/// The world keeps track of all [`GameObject`](crate::core::GameObject)
/// instances and provides access to shared systems like physics and input.
/// Only one instance can exist at a time and is globally accessible via
/// [`World::instance`].
pub struct World {
    pub objects: HashMap<GameObjectId, Box<GameObject>>,
    pub children: Vec<GameObjectId>,
    pub active_camera: Option<GameObjectId>,
    pub physics: PhysicsSimulator,
    pub input: InputManager,
    pub assets: Arc<AssetStore>,

    start_time: Instant,
    delta_time: Duration,
    last_frame_time: Instant,
    requested_shutdown: bool,
    next_object_id: GameObjectId,
}

impl World {
    /// Create a new, empty, clean-slate world with default data.
    ///
    /// This currently isn't really useful on its own because it
    /// still depends on the initialization routine in the World::new
    /// function. In the future, a better solution has to be found.
    /// Than managing the world state globally. It's currently like this
    /// because the GameObjectId Deref makes usage very - !! very !! simple.
    /// At the cost of safety and some other things.
    fn empty() -> Box<World> {
        Box::new(World {
            objects: HashMap::new(),
            children: vec![],
            active_camera: None,
            physics: PhysicsSimulator::default(),
            input: InputManager::default(),
            assets: AssetStore::empty(),
            start_time: Instant::now(),
            delta_time: Duration::default(),
            last_frame_time: Instant::now(),
            requested_shutdown: false,
            next_object_id: GameObjectId(0),
        })
    }

    /// # Safety
    /// Creates a new world through World::empty and registers it globally.
    ///
    /// This function must only be called once during program startup since the
    /// returned world is stored in a global pointer for [`World::instance`]. The
    /// world should remain alive for the duration of the application.
    pub unsafe fn new() -> Box<World> {
        let mut world = World::empty();

        // create a second mutable reference so G_WORLD can be used in (~un~)safe code
        unsafe {
            G_WORLD = world.as_mut();
        }

        world
    }

    // TODO: make this an option later when it's too late
    /// Returns a mutable reference to the global [`World`] instance.
    ///
    /// # Panics
    /// Panics if [`World::new`] has not been called beforehand.
    pub fn instance() -> &'static mut World {
        unsafe {
            if G_WORLD.is_null() {
                panic!("G_WORLD has not been initialized");
            }
            &mut *G_WORLD
        }
    }

    pub fn get_object(&self, obj: &GameObjectId) -> Option<&GameObject> {
        self.objects.get(obj).map(|o| o.as_ref())
    }

    pub fn get_object_mut(&mut self, obj: &GameObjectId) -> Option<&mut Box<GameObject>> {
        self.objects.get_mut(obj)
    }

    pub fn new_object<S: Into<String>>(&mut self, name: S) -> GameObjectId {
        let id = self.next_object_id;
        self.next_object_id.0 += 1;

        let obj = Box::new(GameObject {
            id,
            name: name.into(),
            children: vec![],
            parent: None,
            transform: Transform::new(id),
            drawable: None,
            components: vec![],
        });

        self.objects.insert(id, obj);
        // TODO: Consider adding the object to the world right away

        id
    }

    pub fn new_camera(&mut self) -> GameObjectId {
        let camera = CameraPrefab.build(self);

        if self.active_camera.is_none() {
            self.add_child(camera);
            self.active_camera = Some(camera);
        }

        camera
    }

    pub fn add_child(&mut self, mut obj: GameObjectId) {
        self.children.push(obj);
        obj.parent = None;
    }

    pub fn spawn<P: Prefab>(&mut self, prefab: &P) -> GameObjectId {
        prefab.spawn(self)
    }

    unsafe fn execute_component_func(&mut self, func: unsafe fn(&mut dyn Component)) {
        for (id, object) in &self.objects {
            // just a big hack
            // TODO!!!!: FIND OUT WHY IDS GO CRAZY
            if id >= &self.next_object_id {
                return;
            }
            let object_ptr = object;
            for comp in &object_ptr.components {
                let comp_ptr = comp.as_ptr();
                unsafe { func(&mut **comp_ptr) }
            }
        }
    }

    pub fn update(&mut self) {
        self.tick_delta_time();

        unsafe {
            self.execute_component_func(Component::update);
            self.execute_component_func(Component::late_update);

            while self.physics.last_update.elapsed() > self.physics.timestep {
                self.physics.last_update += self.physics.timestep;
                self.physics.step();
                self.execute_component_func(Component::post_update);
            }
        }

        self.input.next_frame();
    }

    pub fn find_object_by_name(&self, name: &str) -> Option<GameObjectId> {
        self.objects
            .iter()
            .find(|(_, o)| o.name == name)
            .map(|o| o.0)
            .cloned()
    }

    pub fn get_all_components_of_type<C: Component + 'static>(&self) -> Vec<Rc<RefCell<Box<C>>>> {
        let mut collection = Vec::new();

        for child in &self.children {
            Self::get_components_of_children(&mut collection, *child);
        }

        collection
    }

    fn get_components_of_children<C: Component + 'static>(
        collection: &mut Vec<Rc<RefCell<Box<C>>>>,
        obj: GameObjectId,
    ) {
        for child in &obj.children {
            Self::get_components_of_children(collection, *child);
        }

        collection.extend(obj.get_components::<C>());
    }

    pub fn print_objects(&self) {
        info!("{} game objects in world.", self.objects.len());
        Self::print_objects_rec(&self.children, 0)
    }

    pub fn print_objects_rec(children: &Vec<GameObjectId>, i: i32) {
        for child in children {
            info!("{}- {}", "  ".repeat(i as usize), &child.name);
            info!(
                "{}-> Components: {}",
                "  ".repeat(i as usize + 1),
                child.components.len()
            );
            info!(
                "{}-> Has Drawable: {}",
                "  ".repeat(i as usize + 1),
                child.drawable.is_some()
            );
            Self::print_objects_rec(&child.children, i + 1);
        }
    }

    fn tick_delta_time(&mut self) {
        self.delta_time = self.last_frame_time.elapsed();
        self.last_frame_time = Instant::now();
    }

    pub fn delta_time(&self) -> Duration {
        self.delta_time
    }

    pub fn start_time(&self) -> &Instant {
        &self.start_time
    }

    pub fn time(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn initialize_runtime(&mut self, renderer: &Renderer) {
        let world_ptr: *mut World = self;
        unsafe {
            for obj in self.objects.values_mut() {
                if let Some(ref mut drawable) = obj.drawable {
                    drawable.setup(renderer, &mut *world_ptr)
                }
            }
        }
    }

    pub fn delete_object(&mut self, mut object: GameObjectId) {
        object.delete();
    }

    pub(crate) fn unlink_internal(&mut self, mut caller: GameObjectId) {
        if let Some((id, _)) = self.children.iter().find_position(|c| c.0 == caller.0) {
            self.children.remove(id);
        }

        caller.unlink();
        self.objects.remove(&caller);
    }

    pub fn shutdown(&mut self) {
        self.requested_shutdown = true;
    }

    pub fn is_shutting_down(&self) -> bool {
        self.requested_shutdown
    }
}
