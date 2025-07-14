use crate::engine::assets::generic_store::Store;
use crate::engine::assets::material::Material;
use crate::engine::assets::*;
use std::sync::Arc;

pub struct AssetStore {
    pub meshes: Arc<Store<Mesh>>,
    pub shaders: Arc<Store<Shader>>,
    pub textures: Arc<Store<Texture>>,
    pub materials: Arc<Store<Material>>,
    pub bgls: Arc<Store<BGL>>,
}

impl AssetStore {
    pub fn empty() -> Arc<AssetStore> {
        Arc::new(AssetStore {
            meshes: Arc::new(Store::populated()),
            shaders: Arc::new(Store::populated()),
            textures: Arc::new(Store::populated()),
            materials: Arc::new(Store::populated()),
            bgls: Arc::new(Store::populated()),
        })
    }
}
