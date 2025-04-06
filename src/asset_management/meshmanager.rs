use std::collections::HashMap;
use std::rc::Rc;

use wgpu::Device;
use crate::asset_management::bindgroup_layout_manager::MODEL_UBGL_ID;
use crate::asset_management::mesh::{Mesh, RuntimeMesh};
use crate::world::World;

pub type MeshId = usize;

#[derive(Debug)]
pub struct MeshItem {
    raw: Box<Mesh>,
    runtime: Option<RuntimeMesh>,
}

#[derive(Debug, Default)]
pub struct MeshManager {
    meshes: HashMap<MeshId, MeshItem>,
    next_id: MeshId,
    device: Option<Rc<Device>>,
}

#[allow(dead_code)]
impl MeshManager {
    pub fn invalidate_runtime(&mut self) {
        self.meshes
            .values_mut()
            .for_each(|m| m.runtime = None);

        self.device = None;
    }

    pub fn init_runtime(&mut self, device: Rc<Device>) {
        self.device = Some(device);
    }

    pub fn add_mesh(&mut self, mesh: Box<Mesh>) -> MeshId {
        let id = self.next_id;

        self.meshes.insert(
            id,
            MeshItem {
                raw: mesh,
                runtime: None,
            },
        );
        self.next_id += 1;

        id
    }

    pub fn get_mesh_internal_mut(&mut self, id: MeshId) -> Option<&mut MeshItem> {
        self.meshes.get_mut(&id)
    }

    pub fn get_raw_mesh(&self, id: MeshId) -> Option<&Mesh> {
        self.meshes.get(&id).map(|m| m.raw.as_ref())
    }

    pub fn get_runtime_mesh(&self, id: MeshId) -> Option<&RuntimeMesh> {
        self.meshes.get(&id).map(|m| m.runtime.as_ref())?
    }

    pub fn get_runtime_mesh_mut(&mut self, id: MeshId) -> Option<&mut RuntimeMesh> {
        self.get_runtime_mesh_or_init_mut(id)
    }

    pub fn init_runtime_mesh(&mut self, id: MeshId) {
        self.get_runtime_mesh_or_init(id);
    }

    pub fn get_runtime_mesh_or_init(&mut self, id: MeshId) -> Option<&RuntimeMesh> {
        let mesh = self.meshes.get_mut(&id)?;
        if mesh.runtime.is_some() {
            return mesh.runtime.as_ref();
        }

        let model_bgl = World::instance().assets.bind_group_layouts.get_bind_group_layout(MODEL_UBGL_ID).unwrap();
        let runtime_mesh = mesh.raw.as_mut().init_runtime(
            self.device.as_ref().unwrap(),
            model_bgl
        );
        mesh.runtime = Some(runtime_mesh);
        mesh.runtime.as_ref()
    }

    pub fn get_runtime_mesh_or_init_mut(&mut self, id: MeshId) -> Option<&mut RuntimeMesh> {
        let mesh = self.meshes.get_mut(&id)?;
        if mesh.runtime.is_some() {
            return mesh.runtime.as_mut();
        }

        let model_bgl = World::instance().assets.bind_group_layouts.get_bind_group_layout(MODEL_UBGL_ID).unwrap();
        let runtime_mesh = mesh.raw.as_mut().init_runtime(
            self.device.as_ref().unwrap(),
            model_bgl,
        );
        mesh.runtime = Some(runtime_mesh);
        mesh.runtime.as_mut()
    }
}
