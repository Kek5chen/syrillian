//! Physics simulation powered by `rapier`.
//!
//! The [`PhysicsManager`] struct manages rigid bodies / joints, etc.
//! and executes physics steps each frame.

pub mod simulator;

pub use simulator::*;
