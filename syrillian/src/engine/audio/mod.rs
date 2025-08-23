use kira::listener::ListenerHandle;
use kira::sound::static_sound::StaticSoundHandle;
use kira::track::{SpatialTrackBuilder, SpatialTrackHandle};
use kira::{
    AudioManager, AudioManagerSettings, DefaultBackend, PlaySoundError, Tween,
    sound::static_sound::StaticSoundData,
};
use nalgebra::{Quaternion, Vector3};

pub struct AudioScene {
    manager: AudioManager<DefaultBackend>,
    listener: ListenerHandle,
}

impl AudioScene {
    pub fn new() -> Self {
        let mut manager = AudioManager::new(AudioManagerSettings::default())
            .expect("Failed to initialize audio manager");

        let position = Vector3::zeros();
        let orientation = Quaternion::identity();

        let listener = manager
            .add_listener(position, orientation)
            .expect("Failed to add audio listener");

        Self { manager, listener }
    }

    pub fn set_receiver_position(&mut self, receiver_position: Vector3<f32>) {
        self.listener
            .set_position(receiver_position, Tween::default());
    }

    pub fn set_receiver_orientation(&mut self, receiver_orientation: Quaternion<f32>) {
        self.listener
            .set_orientation(receiver_orientation, Tween::default());
    }

    pub fn play_sound(
        &mut self,
        sound: StaticSoundData,
        track: &mut SpatialTrackHandle,
    ) -> Result<StaticSoundHandle, PlaySoundError<()>> {
        track.play(sound)
    }

    pub fn add_spatial_track(&mut self) -> Option<SpatialTrackHandle> {
        let position = Vector3::zeros();
        self.manager
            .add_spatial_sub_track(self.listener.id(), position, SpatialTrackBuilder::default())
            .ok()
    }
}
