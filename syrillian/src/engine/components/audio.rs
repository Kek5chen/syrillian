use crate::World;
use crate::assets::HSound;
use crate::components::Component;
use crate::core::GameObjectId;
use kira::Tween;
use kira::sound::PlaybackState;
use kira::sound::static_sound::{StaticSoundHandle, StaticSoundSettings};
use kira::track::SpatialTrackHandle;
use log::{error, warn};
use nalgebra::Vector3;

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
    initialized: bool,
    start_offset: f64,
    playback_rate: f64,
    volume: f32,
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
            initialized: false,
            start_offset: 0.0,
            playback_rate: 1.0,
            volume: 0.0,
        }
    }

    fn update(&mut self, world: &mut World) {
        if !self.initialized {
            return;
        }

        self.position = self.parent.transform.position();

        self.track_handle
            .as_mut()
            .expect("Spatial track missing")
            .set_position(self.position, Tween::default());

        if self.looping {
            self.play(world);
        }
    }

    fn parent(&self) -> GameObjectId {
        self.parent
    }
}

impl AudioEmitter {
    pub fn init(&mut self, handle: HSound, world: &mut World) {
        self.set_sound(handle);
        self.track_handle = world.audio.add_spatial_track();

        self.initialized = true;
    }

    pub fn play(&mut self, world: &mut World) {
        if self.is_playing() {
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
            let mut settings = StaticSoundSettings::new()
                .start_position(self.start_offset)
                .volume(self.volume)
                .playback_rate(self.playback_rate);

            if self.looping {
                settings = StaticSoundSettings::new()
                    .loop_region(self.start_offset..)
                    .volume(self.volume)
                    .playback_rate(self.playback_rate);
            }

            self.sound_handle = world
                .audio
                .play_sound(sound.sound_data.with_settings(settings), track)
                .ok();
        } else {
            warn!("AudioEmitter play had no sound handle");
        }
        return;
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume;
    }

    pub fn set_playback_rate(&mut self, playback_rate: f64) {
        self.playback_rate = playback_rate;
    }

    pub fn set_start_offset(&mut self, start_offset: f64) {
        self.start_offset = start_offset;
    }

    pub fn start_looping(&mut self) {
        self.looping = true;
    }

    pub fn stop_looping(&mut self) {
        self.looping = false;
        match self.sound_handle.take() {
            Some(mut handle) => handle.stop(Tween::default()),
            None => {}
        }
    }

    pub fn is_playing(&self) -> bool {
        let sound = match self.sound_handle.as_ref() {
            Some(track) => track,
            None => {
                return false;
            }
        };
        sound.state() == PlaybackState::Playing
    }

    pub fn set_sound(&mut self, sound: HSound) {
        self.asset_handle = Some(sound.clone());
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
