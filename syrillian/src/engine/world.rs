//! The [`World`] struct stores and updates all game objects. Its use is to manage any
//! "raw" data, store and provide access to the objects and behavior, with a focus on ease of use.
//!
//! It maintains the scene graph, input state and physics simulation and
//! offers utilities such as methods to create, find and remove game objects.

use crate::assets::{Material, Mesh, Shader, Store, Texture, BGL};
use crate::components::{CRef, Component};
use crate::core::component_storage::ComponentStorage;
use crate::core::{GameObject, GameObjectId, Transform};
use crate::engine::assets::AssetStore;
use crate::engine::prefabs::prefab::Prefab;
use crate::engine::rendering::Renderer;
use crate::input::InputManager;
use crate::physics::PhysicsManager;
use crate::prefabs::CameraPrefab;
use crate::rendering::lights::LightManager;
use itertools::Itertools;
use log::info;
use slotmap::{HopSlotMap, Key};
use std::collections::HashSet;
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
    /// Collection of all game objects indexed by their unique ID
    pub objects: HopSlotMap<GameObjectId, GameObject>,
    /// Collection of all components indexed by their unique ID
    pub components: ComponentStorage,
    /// Root-level game objects that have no parent
    pub children: Vec<GameObjectId>,
    /// The currently active camera used for rendering
    pub active_camera: Option<GameObjectId>,
    /// Physics simulation system
    pub physics: PhysicsManager,
    /// Input management system
    pub input: InputManager,
    /// Light management system
    pub lights: LightManager,
    /// Asset storage containing meshes, textures, materials, etc.
    pub assets: Arc<AssetStore>,

    /// Time when the world was created
    start_time: Instant,
    /// Time elapsed since the last frame
    delta_time: Duration,
    /// Time when the last frame started
    last_frame_time: Instant,
    /// Flag indicating whether a shutdown has been requested
    requested_shutdown: bool,
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
            objects: HopSlotMap::with_key(),
            components: ComponentStorage::default(),
            children: vec![],
            active_camera: None,
            physics: PhysicsManager::default(),
            input: InputManager::default(),
            lights: LightManager::default(),
            assets: AssetStore::empty(),
            start_time: Instant::now(),
            delta_time: Duration::default(),
            last_frame_time: Instant::now(),
            requested_shutdown: false,
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

    /// Retrieves a reference to a game object by its ID
    pub fn get_object(&self, obj: GameObjectId) -> Option<&GameObject> {
        self.objects.get(obj)
    }

    /// Retrieves a mutable reference to a game object by its ID
    pub fn get_object_mut(&mut self, obj: GameObjectId) -> Option<&mut GameObject> {
        self.objects.get_mut(obj)
    }

    /// Creates a new game object with the given name
    pub fn new_object<S: Into<String>>(&mut self, name: S) -> GameObjectId {
        let obj = GameObject {
            id: GameObjectId::null(),
            name: name.into(),
            children: vec![],
            parent: None,
            transform: Transform::new(GameObjectId::null()),
            drawable: None,
            components: HashSet::new(),
        };

        // TODO: Consider adding the object to the world right away
        let mut id = self.objects.insert(obj);

        id.id = id;
        id.transform.owner = id;

        id
    }

    /// Creates a new camera game object
    ///
    /// If no active camera exists yet, this camera will be set as the active camera
    /// and added as a child of the world.
    pub fn new_camera(&mut self) -> GameObjectId {
        let camera = CameraPrefab.build(self);

        if self.active_camera.is_none() {
            self.add_child(camera);
            self.active_camera = Some(camera);
        }

        camera
    }

    /// Adds a game object as a child of the world (root level)
    ///
    /// This removes any existing parent relationship the object might have.
    pub fn add_child(&mut self, mut obj: GameObjectId) {
        self.children.push(obj);
        obj.parent = None;
    }

    /// Spawns a game object from a prefab
    pub fn spawn<P: Prefab>(&mut self, prefab: &P) -> GameObjectId {
        prefab.spawn(self)
    }

    /// Executes a component function on all components of all game objects
    pub(crate) fn execute_component_func<F>(&mut self, func: F)
    where
        F: Fn(&mut dyn Component, &mut World),
    {
        let world = unsafe { &mut *(self as *mut World) };
        self.components.values_mut().for_each(|c| func(c, world))
    }

    /// Updates all game objects and their components
    ///
    /// It will tick delta time and update all components
    ///
    /// If you're using the App runtime, this will be handled for you. Only call this function
    /// if you are trying to use a detached world context.
    pub fn update(&mut self) {
        self.tick_delta_time();

        self.execute_component_func(Component::update);
        self.execute_component_func(Component::late_update);
    }

    /// Performs late update operations after the main update
    ///
    /// If you're using the App runtime, this will be handled for you. Only call this function
    /// if you are trying to use a detached world context.
    pub fn post_update(&mut self) {
        while self.physics.last_update.elapsed() > self.physics.timestep {
            self.physics.last_update += self.physics.timestep;
            self.physics.step();
        }

        self.execute_component_func(Component::post_update);
    }

    /// Prepares for the next frame by resetting the input state
    ///
    /// If you're using the App runtime, this will be handled for you. Only call this function
    /// if you are trying to use a detached world context.
    pub fn next_frame(&mut self) {
        self.input.next_frame();
    }

    /// Finds a game object by its name
    ///
    /// Note: If multiple objects have the same name, only the first one found will be returned.
    pub fn find_object_by_name(&self, name: &str) -> Option<GameObjectId> {
        self.objects
            .iter()
            .find(|(_, o)| o.name == name)
            .map(|o| o.0)
    }

    /// Gets all components of a specific type from all game objects in the world
    ///
    /// This method recursively traverses the entire scene graph to find all components
    /// of the specified type.
    pub fn get_all_components_of_type<C: Component + 'static>(&self) -> Vec<CRef<C>> {
        let mut collection = Vec::new();

        for child in &self.children {
            Self::get_components_of_children(&mut collection, *child);
        }

        collection
    }

    /// Helper method to recursively collect components of a specific type from a game object and its children
    fn get_components_of_children<C: Component + 'static>(
        collection: &mut Vec<CRef<C>>,
        obj: GameObjectId,
    ) {
        for child in &obj.children {
            Self::get_components_of_children(collection, *child);
        }

        collection.extend(obj.get_components::<C>());
    }

    /// Prints information about all game objects in the world to the log
    ///
    /// This method will print out the scene graph to the console and add some information about
    /// components and drawables attached to the objects.
    pub fn print_objects(&self) {
        info!("{} game objects in world.", self.objects.len());
        print_objects_rec(&self.children, 0)
    }

    /// Updates the delta time based on the elapsed time since the last frame
    fn tick_delta_time(&mut self) {
        self.delta_time = self.last_frame_time.elapsed();
        self.last_frame_time = Instant::now();
    }

    /// Returns the time elapsed since the last frame
    pub fn delta_time(&self) -> Duration {
        self.delta_time
    }

    /// Returns the instant in time when the world was created
    pub fn start_time(&self) -> Instant {
        self.start_time
    }

    /// Returns the total time elapsed since the world was created
    pub fn time(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Initializes runtime components of all game objects
    ///
    /// This method sets up all drawable components with the provided renderer.
    ///
    /// If you're using the App runtime, this will be handled for you. Only call this function
    /// if you are trying to use a detached world context.
    pub fn initialize_runtime(&mut self, renderer: &Renderer) {
        self.lights.init(&renderer);
        let world_ptr: *mut World = self;
        unsafe {
            for (id, obj) in &mut self.objects {
                if let Some(ref mut drawable) = obj.drawable {
                    drawable.setup(renderer, &mut *world_ptr, id)
                }
            }
        }
    }

    /// Marks a game object for deletion. This will immediately run the object internal destruction routine
    /// and also clean up any component-specific data.
    pub fn delete_object(&mut self, mut object: GameObjectId) {
        object.delete();
    }

    /// Internal method to unlink and remove a game object from the world
    ///
    /// This method will remove the object from the world's children list if it's a root-level object,
    /// unlinks it from its parent and children and then remove the object from the world's objects collection
    ///
    /// This is an internal method as it's used in form of a callback from the object,
    /// signaling that its internal destruction routine has been done, which includes its components and
    /// can now be safely unlinked.
    pub(crate) fn unlink_internal(&mut self, mut caller: GameObjectId) {
        if let Some((id, _)) = self.children.iter().find_position(|c| **c == caller) {
            self.children.remove(id);
        }

        caller.unlink();
        self.objects.remove(caller);
    }

    /// Requests a shutdown of the world
    ///
    /// The world might not shut down immediately as cleanup will be started after this.
    pub fn shutdown(&mut self) {
        self.requested_shutdown = true;
    }

    /// `true` if a shutdown has been requested, `false` otherwise
    pub fn is_shutting_down(&self) -> bool {
        self.requested_shutdown
    }
}

impl AsRef<Store<Mesh>> for World {
    fn as_ref(&self) -> &Store<Mesh> {
        &self.assets.meshes
    }
}

impl AsRef<Store<Shader>> for World {
    fn as_ref(&self) -> &Store<Shader> {
        &self.assets.shaders
    }
}

impl AsRef<Store<Texture>> for World {
    fn as_ref(&self) -> &Store<Texture> {
        &self.assets.textures
    }
}

impl AsRef<Store<Material>> for World {
    fn as_ref(&self) -> &Store<Material> {
        &self.assets.materials
    }
}

impl AsRef<Store<BGL>> for World {
    fn as_ref(&self) -> &Store<BGL> {
        &self.assets.bgls
    }
}

fn print_objects_rec(children: &Vec<GameObjectId>, i: i32) {
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
        print_objects_rec(&child.children, i + 1);
    }
}
