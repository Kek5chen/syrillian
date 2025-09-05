use kira::listener::ListenerHandle;
use kira::track::{SpatialTrackBuilder, SpatialTrackHandle};
use kira::{AudioManager, AudioManagerSettings, DefaultBackend, Tween};
use nalgebra::{Quaternion, Vector3};

pub struct AudioScene {
    manager: AudioManager<DefaultBackend>,
    listener: ListenerHandle,
}

impl Default for AudioScene {
    fn default() -> Self {
        let mut manager = match AudioManager::new(AudioManagerSettings::default()) {
            Ok(manager) => manager,
            Err(e) => {
                log::error!("Failed to initialize audio manager : {}", e);
                std::process::exit(1);
            }
        };

        let position = Vector3::zeros();
        let orientation = Quaternion::identity();

        let listener = match manager.add_listener(position, orientation) {
            Ok(listener) => listener,
            Err(e) => {
                log::error!("Failed to add audio listener : {}", e);
                std::process::exit(1);
            }
        };

        Self { manager, listener }
    }
}

impl AudioScene {
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
