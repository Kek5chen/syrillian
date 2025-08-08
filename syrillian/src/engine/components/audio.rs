use kira::track::{SpatialTrackBuilder, SpatialTrackHandle};
use nalgebra::{Quaternion, Vector3};
use crate::assets::{Sound, H};
use crate::components::Component;
use crate::core::GameObjectId;
use crate::World;

pub struct AudioReceiver {
    parent: GameObjectId,
}

pub struct AudioEmitter {
    parent: GameObjectId,
    handle: Option<H<Sound>>,
    spatial_track: Option<SpatialTrackHandle>,
    playing: bool,
}

impl Component for AudioEmitter {
    fn new(parent: GameObjectId) -> Self {
        Self { parent, handle: None, spatial_track: None, playing: false }
    }

    fn update(&mut self, world: &mut World) {

    }

    fn parent(&self) -> GameObjectId {
        self.parent
    }
}

impl AudioEmitter {
    pub fn init(&mut self, handle: H<Sound>, world: &mut World) {
        self.handle = Some(handle);
        let position = (Vector3::<f32>::new(0.0, 0.0, 0.0));
        let orientation = Quaternion::<f32>::identity();
        self.spatial_track = world.audio_scene.manager.add_spatial_sub_track(&world.audio_scene.listener, position, SpatialTrackBuilder::default()).ok();
    }
    pub fn play(&mut self, world: &mut World) {

        self.playing = true;
        let track = &mut self.spatial_track;
        world.audio_scene.play_sound(self.handle.unwrap(), &world.assets.clone(), self.parent.transform.position(), &mut self.spatial_track.as_mut().unwrap());
    }

    pub fn stop(&mut self) {
        self.playing = false;
    }
}

impl Component for AudioReceiver {
    fn new(parent: GameObjectId) -> Self {
        Self { parent, }
    }

    fn update(&mut self, world: &mut World) {
        let transform = &self.parent().transform;
        World::instance().audio_scene.set_receiver_position(transform.position(), *transform.rotation());
    }

    fn parent(&self) -> GameObjectId {
        self.parent
    }
}

