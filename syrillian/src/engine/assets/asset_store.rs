//! The [`AssetStore`] is used to store "raw" data, like meshes, images (textues) etc.
//!
//! It exists to cleanly differentiate between GPU state, and plain-old-data.
//! You can safely add stuff to any store as you wish, and then request to use it
//! when rendering. The [`AssetCache`](crate::rendering::AssetCache) is the other side of this component
//! which you will interact with to retrieve the instantiated- hot GPU data.
//!
//! See module level documentation for more info.

use crate::engine::assets::*;
use std::sync::Arc;

pub struct AssetStore {
    pub meshes: Arc<Store<Mesh>>,
    pub shaders: Arc<Store<Shader>>,
    pub textures: Arc<Store<Texture>>,
    pub materials: Arc<Store<Material>>,
    pub bgls: Arc<Store<BGL>>,
    pub fonts: Arc<Store<Font>>,
}

impl AssetStore {
    pub fn new() -> Arc<AssetStore> {
        Arc::new(AssetStore {
            meshes: Arc::new(Store::populated()),
            shaders: Arc::new(Store::populated()),
            textures: Arc::new(Store::populated()),
            materials: Arc::new(Store::populated()),
            bgls: Arc::new(Store::populated()),
            fonts: Arc::new(Store::empty()),
        })
    }
}
