//! The [`World`] struct stores and updates all game objects. Its use is to manage any
//! "raw" data, store and provide access to the objects and behavior, with a focus on ease of use.
//!
//! It maintains the scene graph, input state and physics simulation and
//! offers utilities such as methods to create, find and remove game objects.

use crate::assets::{BGL, Material, Mesh, Shader, Sound, Store, Texture};
use crate::audio::AudioScene;
use crate::components::{CRef, CWeak, CameraComponent, Component};
use crate::core::component_storage::ComponentStorage;
use crate::core::{GameObject, GameObjectId, Transform};
use crate::engine::assets::AssetStore;
use crate::engine::prefabs::prefab::Prefab;
use crate::game_thread::GameAppEvent;
use crate::input::InputManager;
use crate::physics::PhysicsManager;
use crate::prefabs::CameraPrefab;
use crate::rendering::CPUDrawCtx;
use crate::rendering::message::RenderMsg;
use itertools::Itertools;
use log::info;
use slotmap::{HopSlotMap, Key};
use std::collections::HashSet;
use std::mem::swap;
use std::sync::{Arc, mpsc};
use web_time::{Duration, Instant};

static mut G_WORLD: *mut World = std::ptr::null_mut();

/// Central structure representing the running scene.
///
/// The world keeps track of all [`GameObject`](GameObject)
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
    active_camera: CWeak<CameraComponent>,
    /// Physics simulation system
    pub physics: PhysicsManager,
    /// Input management system
    pub input: InputManager,
    /// Asset storage containing meshes, textures, materials, etc.
    pub assets: Arc<AssetStore>,
    /// Spatial audio
    pub audio: AudioScene,

    /// Time when the world was created
    start_time: Instant,
    /// Time elapsed since the last frame
    delta_time: Duration,
    /// Time when the last frame started
    last_frame_time: Instant,

    /// Flag indicating whether a shutdown has been requested
    requested_shutdown: bool,
    render_tx: mpsc::Sender<RenderMsg>,
    game_event_tx: mpsc::Sender<GameAppEvent>,
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
    fn empty(
        render_tx: mpsc::Sender<RenderMsg>,
        game_event_tx: mpsc::Sender<GameAppEvent>,
        assets: Arc<AssetStore>,
    ) -> Box<World> {
        Box::new(World {
            objects: HopSlotMap::with_key(),
            components: ComponentStorage::default(),
            children: vec![],
            active_camera: CWeak::null(),
            physics: PhysicsManager::default(),
            input: InputManager::new(game_event_tx.clone()),
            assets,
            audio: AudioScene::default(),

            start_time: Instant::now(),
            delta_time: Duration::default(),
            last_frame_time: Instant::now(),

            requested_shutdown: false,
            render_tx,
            game_event_tx,
        })
    }

    /// # Safety
    ///
    /// Creates a new world through World::empty and registers it globally.
    ///
    /// This function must only be called once during program startup since the
    /// returned world is stored in a global pointer for [`World::instance`]. The
    /// world should remain alive for the duration of the application.
    pub unsafe fn new(
        assets: Arc<AssetStore>,
        render_tx: mpsc::Sender<RenderMsg>,
        game_event_tx: mpsc::Sender<GameAppEvent>,
    ) -> Box<World> {
        let mut world = World::empty(render_tx, game_event_tx, assets);

        // create a second mutable reference so G_WORLD can be used in (~un~)safe code
        unsafe {
            G_WORLD = world.as_mut();
        }

        world
    }

    /// # Safety
    ///
    /// View [`World::new`]. This function will just set up data structures around the world
    /// needed for initialization. Mostly useful for tests.
    pub unsafe fn fresh() -> (
        Box<World>,
        mpsc::Receiver<RenderMsg>,
        mpsc::Receiver<GameAppEvent>,
    ) {
        let (tx1, rx1) = mpsc::channel();
        let (tx2, rx2) = mpsc::channel();
        let store = AssetStore::new();
        let world = unsafe { World::new(store, tx1, tx2) };
        (world, rx1, rx2)
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
    pub fn new_camera(&mut self) -> CRef<CameraComponent> {
        let obj = CameraPrefab.build(self);
        let camera = obj
            .get_component::<CameraComponent>()
            .expect("CameraPrefab should always attach a camera to itself");

        if !self.active_camera.exists(self) {
            self.add_child(obj);
            self.set_active_camera(camera.clone());
        }

        camera
    }

    pub fn set_active_camera(&mut self, camera: CRef<CameraComponent>) {
        self.active_camera = camera.downgrade();
    }

    pub fn active_camera(&self) -> CWeak<CameraComponent> {
        self.active_camera
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

    /// Runs possible physics update if the timestep time has elapsed yet
    pub fn fixed_update(&mut self) {
        while self.physics.last_update.elapsed() >= self.physics.timestep {
            self.execute_component_func(Component::pre_fixed_update);

            self.physics.last_update += self.physics.timestep;
            self.physics.step();

            self.execute_component_func(Component::fixed_update);
        }

        let rem = self.physics.last_update.elapsed();
        self.physics.alpha =
            (rem.as_secs_f32() / self.physics.timestep.as_secs_f32()).clamp(0.0, 1.0);
    }

    /// Updates all game objects and their components
    ///
    /// It will tick delta time and update all components
    ///
    /// If you're using the App runtime, this will be handled for you. Only call this function
    /// if you are trying to use a detached world context.
    pub fn update(&mut self) {
        self.execute_component_func(Component::update);
        self.execute_component_func(Component::late_update);
    }

    /// Performs late update operations after the main update
    ///
    /// If you're using the App runtime, this will be handled for you. Only call this function
    /// if you are trying to use a detached world context.
    pub fn post_update(&mut self) {
        let mut frame_proxy_batch = Vec::with_capacity(self.components.len());
        self.execute_component_func(Component::post_update);
        self.sync_fresh_components();
        self.sync_removed_components();

        for (_, obj) in self.objects.iter() {
            if !obj.transform.is_dirty() {
                continue;
            }
            for comp in obj.components.iter() {
                frame_proxy_batch.push(RenderMsg::UpdateTransform(
                    comp.typed_id(),
                    obj.transform.get_global_transform_matrix(),
                ));
            }
        }
        let world = self as *mut World;
        for (ctid, comp) in self.components.iter_mut() {
            let ctx = CPUDrawCtx::new(ctid, &mut frame_proxy_batch);
            unsafe {
                comp.update_proxy(&*world, ctx);
            }
        }

        if let Some(mut active_camera) = self.active_camera.upgrade(self) {
            let obj = active_camera.parent();
            if obj.transform.is_dirty() {
                let pos = obj.transform.position();
                let view_mat = obj.transform.view_matrix_rigid().to_matrix();
                let view_proj_mat = active_camera.projection.as_matrix() * view_mat;
                frame_proxy_batch.push(RenderMsg::UpdateActiveCamera(Box::new(move |cam| {
                    cam.view_mat = view_mat;
                    cam.proj_view_mat = view_proj_mat;
                    cam.pos = pos;
                })));
            }

            if active_camera.is_projection_dirty() {
                let proj_mat = active_camera.projection;
                frame_proxy_batch.push(RenderMsg::UpdateActiveCamera(Box::new(move |cam| {
                    cam.proj_view_mat = proj_mat.as_matrix() * cam.view_mat;
                    cam.projection_mat = proj_mat;
                })));
                active_camera.clear_projection_dirty();
            }
        }

        self.render_tx
            .send(RenderMsg::CommandBatch(frame_proxy_batch))
            .unwrap();
    }

    /// Internally sync removed components to the Render Thread for proxy deletion
    fn sync_removed_components(&mut self) {
        if self.components.removed.is_empty() {
            return;
        }

        let mut removed = Vec::new();
        swap(&mut removed, &mut self.components.removed);

        for ctid in removed {
            self.render_tx.send(RenderMsg::RemoveProxy(ctid)).unwrap();
        }
    }

    /// Internally sync new components to the Render Thread for proxy creation
    fn sync_fresh_components(&mut self) {
        if self.components.fresh.is_empty() {
            return;
        }

        let mut fresh = Vec::new();
        swap(&mut fresh, &mut self.components.fresh);
        for cid in fresh {
            let Some(mut comp) = self.components.get_dyn(cid) else {
                continue;
            };

            let local_to_world = comp.parent().transform.get_global_transform_matrix();
            if let Some(proxy) = comp.create_render_proxy(World::instance()) {
                self.render_tx
                    .send(RenderMsg::RegisterProxy(cid, proxy, local_to_world))
                    .unwrap();
            }
            if let Some(proxy) = comp.create_light_proxy(World::instance()) {
                self.render_tx
                    .send(RenderMsg::RegisterLightProxy(cid, proxy))
                    .unwrap();
            }
        }
    }

    /// Prepares for the next frame by resetting the input state
    ///
    /// If you're using the App runtime, this will be handled for you. Only call this function
    /// if you are trying to use a detached world context.
    pub fn next_frame(&mut self) {
        for child in self.objects.values_mut() {
            child.transform.clear_dirty();
        }
        self.input.next_frame();
        self.tick_delta_time();
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

    pub fn set_window_title(&mut self, title: String) {
        self.game_event_tx
            .send(GameAppEvent::UpdateWindowTitle(title))
            .unwrap();
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

impl AsRef<Store<Sound>> for World {
    fn as_ref(&self) -> &Store<Sound> {
        &self.assets.sounds
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
        print_objects_rec(&child.children, i + 1);
    }
}
