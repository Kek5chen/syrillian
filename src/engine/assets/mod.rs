mod bind_group_layout;
pub mod scene_loader;

mod asset_store;
pub(crate) mod generic_store;

mod material;
mod mesh;
mod shader;
mod texture;

mod handle;
mod key;

pub use self::asset_store::*;
pub use self::bind_group_layout::*;
pub use self::material::*;
pub use self::mesh::*;
pub use self::shader::*;
pub use self::texture::*;
pub use self::handle::*;

pub(crate) use self::generic_store::*;
pub(crate) use self::key::*;

pub type HMaterial = H<Material>;
pub type HShader = H<Shader>;
pub type HTexture = H<Texture>;
pub type HMesh = H<Mesh>;
