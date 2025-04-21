use log::error;
use nalgebra::Matrix4;
use wgpu::{Device, IndexFormat, Queue, RenderPass};

use crate::asset_management::materialmanager::{MaterialId, RuntimeMaterial, FALLBACK_MATERIAL_ID};
use crate::asset_management::meshmanager::MeshId;
use crate::asset_management::ShaderId;
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
        world.assets.meshes.init_runtime_mesh(self.mesh);

        let material_ids: Vec<MaterialId> = world
            .assets
            .meshes
            .get_raw_mesh(self.mesh)
            .expect("Mesh must exist")
            .material_ranges
            .iter()
            .map(|(id, _)| *id)
            .collect();

        unsafe {
            let world = world as *mut World;
            for mat_id in material_ids {
                (*world)
                    .assets
                    .materials
                    .init_runtime_material_id(&mut (*world), mat_id,)
                    .expect("Runtime material should be initialized..");
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

    fn draw(&self, world: &mut World, rpass: &mut RenderPass, default_pipeline: Option<ShaderId>) {
        unsafe {
            let world = world as *mut World;

            let runtime_mesh = (*world)
                .assets
                .meshes
                .get_runtime_mesh(self.mesh)
                .expect("Runtime mesh should be initialized before calling draw.");

            let mesh = (*world)
                .assets
                .meshes
                .get_raw_mesh(self.mesh)
                .expect("Normal mesh should be set");

            let default_shader = (*world)
                .assets
                .shaders
                .get_shader(default_pipeline)
                .unwrap_or_else(|| {
                    error!("Passed in Default Pipeline is not available");
                    (*world)
                        .assets
                        .shaders
                        .get_shader(None)
                        .expect("Fallback shader should always exist")
            });

            rpass.set_vertex_buffer(0, runtime_mesh.data.vertices_buf.slice(..));
            rpass.set_bind_group(1, &runtime_mesh.data.model_bind_group, &[]);

            let i_buffer = &runtime_mesh.data.indices_buf.as_ref();

            for (mat_id, range) in &mesh.material_ranges {
                let runtime_material = match (*world)
                    .assets
                    .materials
                    .get_runtime_material(*mat_id) {
                    None => (*world).assets.materials.get_runtime_material(FALLBACK_MATERIAL_ID).unwrap(),
                    Some(mat) => mat,
                };

                let shader = (*world)
                    .assets
                    .shaders
                    .get_shader(runtime_material.shader)
                    .unwrap_or(default_shader);
                rpass.set_pipeline(&shader.pipeline);
                rpass.set_bind_group(2, &runtime_material.bind_group, &[]);
                if let Some(i_buffer) = i_buffer {
                    rpass.set_index_buffer(i_buffer.slice(..), IndexFormat::Uint32);
                    rpass.draw_indexed(range.clone(), 0, 0..1);
                } else {
                    rpass.draw(range.clone(), 0..1);
                }
            }
        }
    }
}
