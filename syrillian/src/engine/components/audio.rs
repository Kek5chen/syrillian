use crate::components::Component;
use crate::core::GameObjectId;
use crate::World;

pub struct AudioReceiver {
    parent: GameObjectId,
}

pub struct AudioEmitter {
    parent: GameObjectId,
    sound_id: String,
    playing: bool,
}

impl Component for AudioEmitter {
    fn new(parent: GameObjectId) -> Self {
        Self { parent, sound_id: String::new(), playing: false }
    }

    fn update(&mut self, world: &mut World) {
        if !self.playing {
            return;
        }
        let pos = self.parent.transform.position();
        World::instance().audio_scene.play_sound(self.sound_id.as_str(), pos);
    }

    fn parent(&self) -> GameObjectId {
        self.parent
    }
}

impl AudioEmitter {
    pub fn set_sound(&mut self, sound: String) {
        self.sound_id = sound;
    }
    pub fn play(&mut self) {
        self.playing = true;
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
        World::instance().audio_scene.set_receiver_position(transform.position());
    }

    fn parent(&self) -> GameObjectId {
        self.parent
    }
}

