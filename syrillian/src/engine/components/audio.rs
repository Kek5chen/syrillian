use crate::World;
use crate::assets::{HSound};
use crate::components::Component;
use crate::core::GameObjectId;
use kira::Tween;
use kira::sound::static_sound::{StaticSoundHandle, StaticSoundSettings};
use kira::track::{SpatialTrackHandle};
use log::error;
use nalgebra::{Vector3};

pub struct AudioReceiver {
    parent: GameObjectId,
}

pub struct AudioEmitter {
    parent: GameObjectId,
    asset_handle: Option<HSound>,
    sound_handle: Option<StaticSoundHandle>,
    track_handle: Option<SpatialTrackHandle>,
    looping: bool,
    position: Vector3<f32>,
}

impl Component for AudioEmitter {
    fn new(parent: GameObjectId) -> Self {
        Self {
            parent,
            asset_handle: None,
            sound_handle: None,
            track_handle: None,
            looping: false,
            position: Vector3::zeros(),
        }
    }

    fn update(&mut self, world: &mut World) {
        self.position = self.parent.transform.position();
        self.track_handle
            .as_mut()
            .expect("Spatial track missing")
            .set_position(self.position, Tween::default());

        if self.looping {
            self.play(world)
        }
    }

    fn parent(&self) -> GameObjectId {
        self.parent
    }
}

impl AudioEmitter {
    pub fn init(&mut self, handle: HSound, world: &mut World) {
        self.asset_handle = Some(handle);

        self.track_handle = world.audio.add_spatial_track();
    }
    pub fn play(&mut self, world: &mut World) {
        if self.looping {
            return;
        }

        let track = match self.track_handle.as_mut() {
            Some(track) => track,
            None => {
                error!("AudioEmitter play had no track handle");
                return;
            }
        };

        let h = match self.asset_handle {
            Some(h) => h,
            None => {
                error!("AudioEmitter play had no asset handle");
                return;
            }
        };

        if let Some(sound) = world.assets.sounds.try_get(h) {
            self.sound_handle = world.audio.play_sound(sound.sound_data.clone(), track).ok();
        } else {
            return;
        }
    }

    pub fn start_looping(&mut self, world: &mut World) {
        if self.looping {
            return;
        }
        self.looping = true;

        let track = match self.track_handle.as_mut() {
            Some(track) => track,
            None => {
                error!("AudioEmitter play had no track handle");
                return;
            }
        };

        let h = match self.asset_handle {
            Some(h) => h,
            None => {
                error!("AudioEmitter play had no asset handle");
                return;
            }
        };

        if let Some(sound) = world.assets.sounds.try_get(h) {
            let settings = StaticSoundSettings::new().loop_region(0.0..);
            self.sound_handle = world
                .audio
                .play_sound(sound.sound_data.with_settings(settings), track)
                .ok();
        } else {
            return;
        }
    }

    pub fn stop_looping(&mut self) {
        self.looping = false;
        match self.sound_handle.take() {
            Some(mut handle) => handle.stop(Tween::default()),
            None => {}
        }
    }
}

impl Component for AudioReceiver {
    fn new(parent: GameObjectId) -> Self {
        Self { parent }
    }

    fn update(&mut self, world: &mut World) {
        let transform = &self.parent().transform;

        world.audio.set_receiver_position(transform.position());
        world.audio.set_receiver_orientation(*transform.rotation());
    }

    fn parent(&self) -> GameObjectId {
        self.parent
    }
}
