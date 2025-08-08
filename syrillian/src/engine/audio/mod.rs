use std::collections::HashMap;
use kira::{AudioManager, AudioManagerSettings, DefaultBackend, sound::static_sound::StaticSoundData, Tween, Value};
use std::path::Path;
use std::sync::Arc;
use kira::listener::ListenerHandle;
use kira::track::{SpatialTrackBuilder, SpatialTrackHandle};
use nalgebra::{Quaternion, Vector3};
use crate::assets::{AssetStore, Sound, H};
use crate::World;

pub struct AudioScene {
    receiver_position: Vector3<f32>,
    receiver_orientation: Quaternion<f32>,
    pub manager: AudioManager<DefaultBackend>,
    pub listener: ListenerHandle,
    // spatial_track: SpatialTrackHandle,
}

impl AudioScene {
    pub fn new() -> Self {
        let mut manager = AudioManager::new(AudioManagerSettings::default()).expect("Failed to initialize audio manager");


        let position = (Vector3::<f32>::new(0.0, 0.0, 0.0));
        let orientation = (Quaternion::<f32>::identity());

        // have to use mint feature from nalgebra for conversions
        let listener = manager.add_listener(position, orientation).expect("Failed to add audio listener");
        //let spatial_track = manager.add_spatial_sub_track(&listener, position, SpatialTrackBuilder::default()).expect("Failed to add audio spatial track");
        Self {
            receiver_position: position,
            receiver_orientation: orientation,
            manager,
            listener,
            //spatial_track
        }
    }

    pub fn set_receiver_position(&mut self, receiver_position: Vector3<f32>, receiver_orientation: Quaternion<f32>) {
        self.receiver_position = receiver_position;
        self.receiver_orientation = receiver_orientation;
    }

    pub fn play_sound(&mut self, handle: H<Sound>, assets: &Arc<AssetStore>, position: Vector3<f32>, track: &mut SpatialTrackHandle) {
        let sound = assets.sounds.try_get(handle).unwrap();

        self.listener.set_position(self.receiver_position, Tween::default());
        
        track.play(sound.clone().sound_data);
        //self.spatial_track.set_position(position, Tween::default());

        // self.spatial_track.play(sound.clone().sound_data);

    }

    pub fn add_track(&mut self, track: SpatialTrackBuilder) {}
}

// /// Stores a [StaticSoundData]
// pub struct AudioAsset {
//     sound_data: StaticSoundData,
// }
//
// /// Stores an [AudioManager] and a map of id's to audio assets
// pub struct AudioSystem {
//     manager: AudioManager<DefaultBackend>,
//     assets: HashMap<String, AudioAsset>,
// }
//
// /// Stores an [AudioSystem], the position of the receiver (for now there is only one per scene),
// /// and the max distance that audio can be heard from the receiver
// pub struct AudioScene {
//     audio_system: AudioSystem,
//     receiver_position: Vector3<f32>,
//     max_distance: f32,
// }
//
// impl Default for AudioScene {
//     fn default() -> Self {
//         Self {
//             audio_system: AudioSystem::default(),
//             receiver_position: Vector3::new(0.0, 0.0, 0.0),
//             max_distance: 50.0,
//         }
//     }
// }
//
// impl AudioScene {
//     /// Loads a sound file from a path and saves it under id
//     pub fn load_sound(&mut self, id: &str, path: &str) {
//         self.audio_system.load_sound(id, path);
//     }
//
//     /// Sets the receiver position for the scene for spatial audio
//     pub fn set_receiver_position(&mut self, receiver_position: Vector3<f32>) {
//         self.receiver_position = receiver_position;
//     }
//
//     // Calculate linear volume scalar (0-1) based on distance between source and receiver and the max distance
//     pub fn calculate_volume_linear(distance: f32, max_distance: f32) -> f32 {
//         if distance >= max_distance {
//             0.0
//         } else {
//             (1.0 - (distance / max_distance)).clamp(0.0, 1.0)
//         }
//     }
//
//     // Converts normalized volume scalar to decibels
//     pub fn volume_to_db(volume: f32) -> f32 {
//         -60.0 + (0.0 - (-60.0)) * volume.clamp(0.0, 1.0)
//     }
//
//
//     /// Plays a sound from an id based off the position in the scene
//     pub fn play_sound(&mut self, handle: H<Sound>, position: Vector3<f32>) {
//         // This is such a hack. Need to find a spatial audio lib
//         /*
//         let distance = (position - self.receiver_position).magnitude();
//
//         let volume: f32 = Self::calculate_volume_linear(distance, self.max_distance);
//
//         let volume_db = Self::volume_to_db(volume);
//
//         // still need to implement panning
//         let pan = 0.0;
//         */
//         // self.audio_system.play_sound(sound_id, volume_db.min(0.0), pan);
//     }
// }
//
// impl Default for AudioSystem {
//     fn default() -> Self {
//         Self {
//             assets: HashMap::new(),
//             manager: AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).expect("Failed to create AudioManager")
//         }
//     }
// }
//
// impl AudioSystem {
//     pub fn has_asset(&self, id: &str) -> bool {
//         return self.assets.contains_key(id);
//     }
//
//     /// Loads a sound file from a path and saves it under id
//     pub fn load_sound(&mut self, id: &str, path: &str) {
//         match StaticSoundData::from_file(Path::new(path)) {
//             Ok(sound_data) => {
//                 self.assets.insert(id.to_string(), AudioAsset { sound_data });
//                 println!("Succesfully loaded sound from '{}'", path);
//             }
//             Err(e) => {
//                 println!("Failed to load sound from '{}': {:?}", path, e);
//             }
//         }
//     }
//
//     /// Play a sound from an id at a given volume and pan
//     pub fn play_sound(&mut self, id: &str, volume: f32, pan: f32) {
//         match self.assets.get(id) {
//             Some(asset) => {
//                 match self.manager.play(asset.sound_data.clone()) {
//                     Ok(mut handle) => {
//                         handle.set_volume(volume, Tween::default());
//                         handle.set_panning(pan, Tween::default());
//                     },
//                     Err(e) => {
//                         eprintln!("Error playing sound '{}': {:?}", id, e);
//                     }
//                 }
//             }
//             None => {
//                 println!("Failed to load sound {}", id);
//             }
//         }
//     }
// }
