//! Renderable components and helpers.
//!
//! The modules here provide abstractions that can be attached to game
//! objects in order to display meshes or images on screen.

pub mod drawable;
pub mod image;
pub mod mesh_renderer;

#[cfg(debug_assertions)]
pub mod camera_debug;

pub use drawable::*;
pub use image::*;
pub use mesh_renderer::*;

#[cfg(debug_assertions)]
pub use camera_debug::*;

#[derive(Debug)]
#[cfg(debug_assertions)]
pub struct DebugRuntimePatternData {
    vertices_buf: wgpu::Buffer,
}
