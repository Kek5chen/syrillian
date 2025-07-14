mod asset_store;
mod bind_group_layout;
pub(crate) mod generic_store;
mod handle;
mod key;
mod material;
mod mesh;
pub mod scene_loader;
mod shader;
mod texture;

pub use self::asset_store::*;
pub use self::bind_group_layout::*;
pub use self::handle::*;
pub use self::material::*;
pub use self::mesh::*;
pub use self::shader::*;
pub use self::texture::*;

pub(crate) use self::generic_store::*;
pub(crate) use self::key::*;

pub type HMaterial = H<Material>;
pub type HShader = H<Shader>;
pub type HTexture = H<Texture>;
pub type HMesh = H<Mesh>;
