use crate::engine::assets::*;
use crate::engine::rendering::State;
use crate::engine::rendering::cache::generic_cache::Cache;
use std::sync::Arc;
use wgpu::{BindGroupLayout, TextureView};
use crate::rendering::{RuntimeMaterial, RuntimeMesh, RuntimeShader};

pub struct AssetCache {
    meshes: Cache<Mesh>,
    shaders: Cache<Shader>,
    textures: Cache<Texture>,
    materials: Cache<Material>,
    bgls: Cache<BGL>,
}

impl AssetCache {
    pub fn new(store: Arc<AssetStore>, state: &State) -> Self {
        let device = &state.device;
        let queue = &state.queue;
        Self {
            meshes: Cache::new(store.meshes.clone(), device.clone(), queue.clone()),
            shaders: Cache::new(store.shaders.clone(), device.clone(), queue.clone()),
            textures: Cache::new(store.textures.clone(), device.clone(), queue.clone()),
            materials: Cache::new(store.materials.clone(), device.clone(), queue.clone()),
            bgls: Cache::new(store.bgls.clone(), device.clone(), queue.clone()),
        }
    }
    pub fn mesh(&self, handle: HMesh) -> Option<Arc<RuntimeMesh>> {
        self.meshes.try_get(handle, self)
    }

    pub fn mesh_unit_square(&self) -> Arc<RuntimeMesh> {
        self.meshes
            .try_get(HMesh::UNIT_SQUARE, self)
            .expect("Unit square is a default mesh")
    }

    pub fn shader(&self, handle: HShader) -> Arc<RuntimeShader> {
        self.shaders.get(handle, self).clone()
    }

    pub fn shader_3d(&self) -> Arc<RuntimeShader> {
        self.shaders.get(HShader::DIM3, self)
    }

    pub fn shader_post_process(&self) -> Arc<RuntimeShader> {
        self.shaders.get(HShader::POST_PROCESS, self)
    }

    pub fn texture(&self, handle: HTexture) -> Arc<TextureView> {
        self.textures.get(handle, self)
    }

    pub fn texture_fallback(&self) -> Arc<TextureView> {
        self.textures.get(HTexture::FALLBACK_DIFFUSE, self)
    }

    pub fn texture_opt(&self, handle: Option<HTexture>, alt: HTexture) -> Arc<TextureView> {
        match handle {
            None => self.textures.get(alt, self),
            Some(handle) => self.textures.get(handle, self),
        }
    }

    pub fn material(&self, handle: HMaterial) -> Arc<RuntimeMaterial> {
        self.materials.get(handle, self)
    }

    pub fn bgl(&self, handle: HBGL) -> Option<Arc<BindGroupLayout>> {
        self.bgls.try_get(handle, self)
    }

    pub fn bgl_model(&self) -> Arc<BindGroupLayout> {
        self.bgls
            .try_get(HBGL::MODEL, self)
            .expect("Model is a default layout")
    }

    pub fn bgl_render(&self) -> Arc<BindGroupLayout> {
        self.bgls
            .try_get(HBGL::RENDER, self)
            .expect("Render is a default layout")
    }

    pub fn bgl_light(&self) -> Arc<BindGroupLayout> {
        self.bgls
            .try_get(HBGL::LIGHT, self)
            .expect("Light is a default layout")
    }

    pub fn bgl_material(&self) -> Arc<BindGroupLayout> {
        self.bgls
            .try_get(HBGL::MATERIAL, self)
            .expect("Material is a default layout")
    }

    pub fn bgl_post_process(&self) -> Arc<BindGroupLayout> {
        self.bgls
            .try_get(HBGL::POST_PROCESS, self)
            .expect("Post Process is a default layout")
    }
}
