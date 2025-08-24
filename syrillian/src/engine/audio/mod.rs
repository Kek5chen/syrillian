use kira::listener::ListenerHandle;
use kira::track::{SpatialTrackBuilder, SpatialTrackHandle};
use kira::{AudioManager, AudioManagerSettings, DefaultBackend, Tween};
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

    /// Returns none if the spatial track limit was reached
    pub fn add_spatial_track(
        &mut self,
        initial_position: Vector3<f32>,
        track: SpatialTrackBuilder,
    ) -> Option<SpatialTrackHandle> {
        self.manager
            .add_spatial_sub_track(self.listener.id(), initial_position, track)
            .ok()
    }
}
