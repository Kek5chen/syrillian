use nalgebra::Matrix4;
use wgpu::{Device, IndexFormat, Queue, RenderPass};

use crate::asset_management::materialmanager::{RuntimeMaterial, FALLBACK_MATERIAL_ID};
use crate::asset_management::mesh::RuntimeMesh;
use crate::asset_management::meshmanager::MeshId;
use crate::drawables::drawable::Drawable;
use crate::object::GameObjectId;
use crate::world::World;

pub struct MeshRenderer {
    mesh: MeshId,
}

impl MeshRenderer {
    pub fn new(mesh: MeshId) -> Box<MeshRenderer> {
        Box::new(MeshRenderer { mesh })
    }
    
    pub fn mesh(&self) -> MeshId {
        self.mesh
    }
}

impl Drawable for MeshRenderer {
    fn setup(
        &mut self,
        _device: &Device,
        _queue: &Queue,
        world: &mut World,
    ) {
        unsafe {
            let world: *mut World = world;
            (*world)
                .assets
                .meshes
                .init_runtime_mesh(self.mesh);
            let mesh = (*world)
                .assets
                .meshes
                .get_raw_mesh(self.mesh)
                .expect("Normal mesh should be set");

            for (mat_id, _) in &mesh.material_ranges {
                (*world).assets.materials.init_runtime_material_id(
                    &mut *world,
                    *mat_id,
                ).expect("Runtime material should be initialized..");
            }
        }
    }

    fn update(
        &mut self,
        world: &mut World,
        parent: GameObjectId,
        queue: &Queue,
        outer_transform: &Matrix4<f32>,
    ) {
        // TODO: Meshes should be able to be shared. Give ModelData to the MeshRenderer
        let runtime_mesh = world
            .assets
            .meshes
            .get_runtime_mesh_mut(self.mesh)
            .expect("Runtime mesh should be initialized before calling update.");
        runtime_mesh
            .data
            .model_data
            .update(parent, outer_transform);
        queue.write_buffer(
            &runtime_mesh.data.model_data_buffer,
            0,
            bytemuck::cast_slice(&[runtime_mesh.data.model_data]),
        )
    }

    unsafe fn draw(&self, world: &mut World, rpass: &mut RenderPass) {
        let runtime_mesh: *const RuntimeMesh = world
            .assets
            .meshes
            .get_runtime_mesh(self.mesh)
            .expect("Runtime mesh should be initialized before calling draw.");

        let mesh = world
            .assets
            .meshes
            .get_raw_mesh(self.mesh)
            .expect("Normal mesh should be set");

        rpass.set_vertex_buffer(0, (*runtime_mesh).data.vertices_buf.slice(..));
        rpass.set_bind_group(1, &(*runtime_mesh).data.model_bind_group, &[]);
        if let Some(i_buffer) = (*runtime_mesh).data.indices_buf.as_ref() {
            for (mat_id, range) in &(*mesh).material_ranges {
                let material: *const RuntimeMaterial = match world
                    .assets
                    .materials
                    .get_runtime_material(*mat_id) {
                    None => world.assets.materials.get_runtime_material(FALLBACK_MATERIAL_ID).unwrap(),
                    Some(mat) => mat,
                };

                rpass.set_bind_group(2, &(*material).bind_group, &[]);
                rpass.set_index_buffer(i_buffer.slice(..), IndexFormat::Uint32);
                rpass.draw_indexed(range.clone(), 0, 0..1);
            }
        } else {
            for (mat_id, range) in &(*mesh).material_ranges {
                let material: *const RuntimeMaterial = match world
                    .assets
                    .materials
                    .get_runtime_material(*mat_id) {
                    None => world.assets.materials.get_runtime_material(FALLBACK_MATERIAL_ID).unwrap(),
                    Some(mat) => mat,
                };

                rpass.set_bind_group(2, &(*material).bind_group, &[]);
                rpass.draw(range.clone(), 0..1);
            }
        }
    }
}
