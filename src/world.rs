use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};
use itertools::Itertools;
use log::info;
use crate::asset_management::AssetManager;
use crate::components::{CameraComponent, Component};
use crate::input::input_manager::InputManager;
use crate::object::{GameObject, GameObjectId};
use crate::physics::simulator::PhysicsSimulator;
use crate::renderer::Renderer;
use crate::transform::Transform;

static mut G_WORLD: *mut World = std::ptr::null_mut();

pub struct World {
    pub objects: HashMap<GameObjectId, Box<GameObject>>,
    pub children: Vec<GameObjectId>,
    pub active_camera: Option<GameObjectId>,
    pub assets: AssetManager,
    pub physics: PhysicsSimulator,
    pub input: InputManager,
    start_time: Instant,
    delta_time: Duration,
    last_frame_time: Instant,
    requested_shutdown: bool,
    next_object_id: GameObjectId,
}

impl World {
    /// # Safety
    /// This function should not be called more than once since it registers its created world
    /// globally. Neither should the world be dropped before the App has exited and isn't used
    /// anymore.
    pub unsafe fn new() -> Box<World> {
        let mut world = Box::new(World {
            objects: HashMap::new(),
            children: vec![],
            active_camera: None,
            assets: AssetManager::default(),
            last_frame_time: Instant::now(),
            physics: PhysicsSimulator::default(),
            start_time: Instant::now(),
            delta_time: Duration::default(),
            input: InputManager::default(),
            requested_shutdown: false,
            next_object_id: GameObjectId(0),
        });

        // create a second mutable reference so G_WORLD can be used in (~un~)safe code
        unsafe {
            G_WORLD = world.as_mut();
        }

        world
    }

    // TODO: make this an option later when it's too late
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

        id
    }

    pub fn new_camera(&mut self) -> GameObjectId {
        let mut obj = self.new_object("Camera");
        obj.transform.set_compound_pos_first(true);

        obj.add_component::<CameraComponent>();

        if self.active_camera.is_none() {
            self.active_camera = Some(obj);
        }
        obj
    }

    pub fn add_child(&mut self, mut obj: GameObjectId) {
        self.children.push(obj);
        obj.parent = None;
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
                unsafe {
                    func(&mut **comp_ptr)
                }
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
            
            self.input.next_frame();
        }
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

    fn get_components_of_children<C: Component + 'static>(collection: &mut Vec<Rc<RefCell<Box<C>>>>, obj: GameObjectId) {
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
            Self::print_objects_rec(&child.children, i + 1);
        }
    }

    fn tick_delta_time(&mut self) {
        self.delta_time = self.last_frame_time.elapsed();
        self.last_frame_time = Instant::now();
    }

    pub fn get_delta_time(&self) -> Duration {
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
                    drawable.setup(
                        renderer,
                        &mut *world_ptr,
                    )
                }
            }
        }
    }

    pub fn delete_object(&mut self, mut object: GameObjectId) {
        object.delete();
    }

    pub(crate) fn unlink_internal(&mut self, mut caller: GameObjectId) {
        if let Some((id, _)) = self.children
            .iter()
            .find_position(|c| c.0 == caller.0) {
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
