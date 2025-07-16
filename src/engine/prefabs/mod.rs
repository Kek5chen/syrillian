//! Prefabricated objects that can be easily spawned into a [`World`](crate::World).
//!
//! Prefabs create game objects with common configurations such as a basic
//! camera or a textured cube.

pub mod prefab;
pub mod first_person_player;
pub mod cube;
pub mod camera;

pub use prefab::Prefab;

// Premade for you :)
pub use first_person_player::FirstPersonPlayerPrefab;
pub use cube::CubePrefab;
pub use camera::CameraPrefab;