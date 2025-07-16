//! Handling of keyboard and mouse input.
//!
//! [`InputManager`] tracks key states and mouse movement and is used by
//! components and systems to react to user interaction.

pub mod input_manager;

pub use self::input_manager::*;
