//! A cache of hot GPU Runtime Data, uploaded from the [`AssetStore`]
//!
//! For more information please see module level documentation.

use crate::engine::assets::*;
use crate::engine::rendering::cache::generic_cache::Cache;
use crate::engine::rendering::State;
use crate::rendering::cache::font::FontAtlas;
use crate::rendering::cache::GpuTexture;
use crate::rendering::{RuntimeMaterial, RuntimeMesh, RuntimeShader};
use std::sync::Arc;
use wgpu::BindGroupLayout;

pub struct AssetCache {
    pub meshes: Cache<Mesh>,
    pub shaders: Cache<Shader>,
    pub textures: Cache<Texture>,
    pub materials: Cache<Material>,
    pub bgls: Cache<BGL>,
    pub fonts: Cache<Font>,
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
            fonts: Cache::new(store.fonts.clone(), device.clone(), queue.clone()),
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

    pub fn shader_2d(&self) -> Arc<RuntimeShader> {
        self.shaders.get(HShader::DIM2, self)
    }

    pub fn shader_post_process(&self) -> Arc<RuntimeShader> {
        self.shaders.get(HShader::POST_PROCESS, self)
    }

    pub fn texture(&self, handle: HTexture) -> Arc<GpuTexture> {
        self.textures.get(handle, self)
    }

    pub fn texture_fallback(&self) -> Arc<GpuTexture> {
        self.textures.get(HTexture::FALLBACK_DIFFUSE, self)
    }

    pub fn texture_opt(&self, handle: Option<HTexture>, alt: HTexture) -> Arc<GpuTexture> {
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

    pub fn bgl_empty(&self) -> Arc<BindGroupLayout> {
        self.bgls
            .try_get(HBGL::EMPTY, self)
            .expect("Light is a default layout")
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

    pub fn bgl_shadow(&self) -> Arc<BindGroupLayout> {
        self.bgls
            .try_get(HBGL::SHADOW, self)
            .expect("Shadow is a default layout")
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

    pub fn font(&self, handle: HFont) -> Option<Arc<FontAtlas>> {
        self.fonts.try_get(handle, self)
    }

    pub fn refresh_all(&self) -> usize {
        let mut refreshed_count = 0;

        refreshed_count += self.meshes.refresh_dirty();
        refreshed_count += self.shaders.refresh_dirty();
        refreshed_count += self.materials.refresh_dirty();
        refreshed_count += self.textures.refresh_dirty();
        refreshed_count += self.bgls.refresh_dirty();

        refreshed_count
    }
}
