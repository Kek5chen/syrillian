//! Prefabricated objects that can be easily spawned into a [`World`](crate::World).
//!
//! Prefabs create game objects with common configurations such as a basic
//! camera or a textured cube.

pub mod camera;
pub mod cube;
pub mod first_person_player;
pub mod prefab;
pub mod sphere;

pub use prefab::Prefab;

// Premade for you :)
pub use camera::CameraPrefab;
pub use cube::CubePrefab;
pub use first_person_player::FirstPersonPlayerPrefab;
