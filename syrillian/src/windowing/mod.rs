//! Platform windowing and event loop utilities.
//!
//! These helpers abstract the details of the `winit` window creation and
//! application state management into a compact runtime that can be easily used.

pub mod app;
pub mod game_thread;
pub mod state;

pub use app::*;
pub use state::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct RenderTargetId(pub u64);

impl RenderTargetId {
    pub const PRIMARY: Self = Self(0);

    pub const fn get(self) -> u64 {
        self.0
    }

    pub const fn is_primary(self) -> bool {
        self.get() == Self::PRIMARY.get()
    }
}
