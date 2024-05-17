use crate::components::{CameraComp, Component, TransformComp};
use crate::object::GameObject;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::time::Instant;

pub struct World {
    pub objects: Vec<Rc<RefCell<GameObject>>>,
    pub children: Vec<Rc<RefCell<GameObject>>>,
    pub active_camera: Option<Weak<RefCell<GameObject>>>, 
    last_frame_time: Instant,
}

impl World {
    pub fn new() -> World {
        World {
            objects: vec![],
            children: vec![],
            active_camera: None,
            last_frame_time: Instant::now(),
        }
    }

    pub fn new_object(&mut self, name: &str) -> Rc<RefCell<GameObject>> {
        let mut obj = GameObject {
            name: name.to_owned(),
            children: vec![],
            transform: TransformComp::new(),
            drawable: None,
            components: vec![],
        };

        let obj_ptr: *mut GameObject = &mut obj;
        unsafe {
            obj.transform.init(&mut *obj_ptr);
        }

        self.objects.push(Rc::new(RefCell::new(obj)));
        self.objects.last().cloned().unwrap()
    }

    pub fn new_camera(&mut self) -> Rc<RefCell<GameObject>> {
        let obj = self.new_object("Camera");

        obj.borrow_mut().add_component::<CameraComp>();

        if self.active_camera.is_none() {
            self.active_camera = Some(Rc::<RefCell<GameObject>>::downgrade(&obj));
        }
        obj
    }

    pub fn add_child(&mut self, obj: Rc<RefCell<GameObject>>) {
        self.children.push(obj)
    }

    pub fn update(&mut self) {
        // i've grown wiser
        unsafe {
            for object in &self.objects {
                let object_ptr = object.as_ptr();
                (*object_ptr).transform.update(object.clone(), self.last_frame_time.elapsed().as_secs_f32());
                for comp in &(*object_ptr).components {
                    let comp_ptr = comp.as_ptr();
                    (*comp_ptr).update(object.clone(), self.last_frame_time.elapsed().as_secs_f32())
                }
            }
        }
        
        self.last_frame_time = Instant::now();
    }
}
