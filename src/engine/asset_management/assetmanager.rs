use std::rc::Rc;

use wgpu::{Device, Queue};

use crate::asset_management::{MaterialManager, TextureManager};
use crate::asset_management::bindgroup_layout_manager::BindGroupLayoutManager;
use crate::asset_management::meshmanager::MeshManager;
use crate::asset_management::shadermanager::ShaderManager;

#[derive(Default)]
pub struct AssetManager {
    pub textures: TextureManager,
    pub shaders: ShaderManager,
    pub materials: MaterialManager,
    pub meshes: MeshManager,
    pub bind_group_layouts: BindGroupLayoutManager,
}

impl AssetManager {
    pub fn invalidate(&mut self) {
        self.bind_group_layouts.invalidate_runtime();
        self.textures.invalidate_runtime();
        self.shaders.invalidate_runtime();
        self.materials.invalidate_runtime();
        self.meshes.invalidate_runtime();
    }

    pub fn init_runtime(&mut self, device: Rc<Device>, queue: Rc<Queue>) {
        self.bind_group_layouts.init_runtime(device.clone());
        self.textures.init_runtime(device.clone(), queue.clone());
        self.shaders.init_runtime(device.clone());
        self.materials.init_runtime(device.clone(), queue.clone());
        self.meshes.init_runtime(device.clone());
    }
}
