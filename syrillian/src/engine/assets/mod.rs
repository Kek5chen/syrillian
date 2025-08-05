//! Asset loading and management utilities.
//!
//! Assets such as meshes, textures and shaders are stored in type specific
//! stores and referenced through handles. This module also exposes helper
//! functionality for loading scenes.
//!
//! Example on how to interact with the store:
//! ```rust
//! use syrillian::assets::{HMaterial, Material};
//! use syrillian::prefabs::CubePrefab;
//! use syrillian::World;
//!
//! fn update(world: &mut World) {
//!     // make a Material
//!     let material: Material = Material::builder()
//!         .name("Test Material".to_string())
//!         .build();
//!
//!     // add an asset
//!     let material: HMaterial = world.assets.materials.add(material);
//!
//!     // Spawn a cube prefab with the material
//!     let cube_prefab = CubePrefab::new(material);
//!     let cube = world.spawn(&cube_prefab);
//! }
//! ```
//!
//! To see how you can use an asset on the GPU, check [`AssetCache`](crate::rendering::AssetCache)

pub mod scene_loader;

mod asset_store;
pub(crate) mod generic_store;

mod bind_group_layout;
mod material;
mod mesh;
mod shader;
mod texture;
mod font;

mod handle;
mod key;

pub use self::asset_store::*;
pub use self::handle::*;

pub use self::bind_group_layout::*;
pub use self::font::*;
pub use self::material::*;
pub use self::mesh::*;
pub use self::shader::*;
pub use self::texture::*;

pub use generic_store::StoreType;

pub(crate) use self::generic_store::*;
pub(crate) use self::key::*;


pub type HBGL = H<BGL>;
pub type HMaterial = H<Material>;
pub type HMesh = H<Mesh>;
pub type HShader = H<Shader>;
pub type HTexture = H<Texture>;
pub type HFont = H<Font>;
