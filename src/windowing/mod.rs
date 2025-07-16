//! Platform windowing and event loop utilities.
//!
//! These helpers abstract the details of the `winit` window creation and
//! application state management into a compact runtime that can be easily used.

pub mod app;
pub mod state;

pub use app::*;
pub use state::*;
