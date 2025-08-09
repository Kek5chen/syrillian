//! GPU rendering backend built on top of `wgpu`.
//!
//! The rendering module is responsible for preparing GPU resources,
//! building command buffers and drawing the world each frame.
//!
//! As a user, you don't usually need to interact with the things in here,
//! besides the [`AssetCache`].
//!
//! See documentation of [`World`](crate::World) to find out how to add GPU data.
//!
//! To retrieve meshes, or other things, you'll use Handles, defined as [`H<T>`](crate::assets::H),
//! but for cleanliness it's appropriate to use the types like [`HMesh`](crate::assets::HMesh).
//!
//! These handles are indices into the [`AssetStore`](crate::assets::AssetStore), and serve as a
//! combined handle into the [`AssetCache`]. The [`AssetStore`](crate::assets::AssetStore) is
//! where you can put your raw data, which is then initialized by the AssetCache on the GPU.
//!
//! You'll usually only interact with the Cache or something like that, in a
//! [`Drawable`](crate::drawables::Drawable), which is syrillians abstraction for
//! "components" / systems, that know how to "render stuff".
//!
//! In a [`Drawable`](crate::drawables::Drawable) you'll want to interact with the
//! [`DrawCtx`] object that contains all info for the frame, and inner-frame draw call.
//!
//! This is how you'd interact with the asset cache in a [`Drawable`](crate::drawables::Drawable)
//!
//! ```rust
//! use syrillian::assets::{HMaterial, HShader};
//! use syrillian::drawables::Drawable;
//! use syrillian::rendering::DrawCtx;
//! use syrillian::World;
//!
//! struct Something {
//!     material: HMaterial,
//! }
//!
//! impl Drawable for Something {
//!     fn draw(&self, _world: &mut World, ctx: &DrawCtx) {
//!         let unit_square = ctx.frame.cache.mesh_unit_square();
//!         let shader = ctx.frame.cache.shader(HShader::DIM3);
//!         let material = ctx.frame.cache.material(self.material);
//!         let predefined = ctx.frame.cache.material(HMaterial::DEFAULT);
//!     }
//! }
//! ```

pub mod cache;
mod context;
mod error;
mod offscreen_surface;
mod post_process_pass;
pub mod renderer;
pub mod state;
pub(crate) mod uniform;
pub mod lights;

pub use cache::*;
pub use context::*;
pub(crate) use renderer::*;
pub(crate) use state::*;
