use log::error;
use nalgebra::Matrix4;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{BindGroupDescriptor, BindGroupEntry, BufferUsages, IndexFormat, RenderPass};

use crate::asset_management::bindgroup_layout_manager::MODEL_UBGL_ID;
use crate::asset_management::materialmanager::{MaterialId, FALLBACK_MATERIAL_ID};
use crate::asset_management::meshmanager::MeshId;
use crate::asset_management::{Bone, ShaderId};
use crate::drawables::drawable::Drawable;
use crate::object::{GameObjectId, ModelData};
use crate::renderer::Renderer;
use crate::world::World;

#[derive(Debug, Default, Clone)]
pub struct BoneData {
    pub(crate) bones: Vec<Bone>,
}

impl BoneData {
    pub fn as_bytes(&self) -> &[u8] {
        const DUMMY_BONE: Bone = Bone {
            transform: Matrix4::new(
                1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                0.0, 0.0, 0.0, 1.0
            )
        };

        if self.bones.is_empty() {
            bytemuck::bytes_of(&DUMMY_BONE)
        } else {
            bytemuck::cast_slice(&self.bones[..])
        }
    }
}

pub struct RuntimeMeshRenderData {
    mesh_data:        ModelData,
    mesh_data_buffer: wgpu::Buffer,

    bone_data:        BoneData,
    bone_data_buffer: wgpu::Buffer,

    model_bind_group: wgpu::BindGroup,
}


pub struct MeshRenderer {
    mesh: MeshId,
    runtime_data: Option<RuntimeMeshRenderData>,
}

impl MeshRenderer {
    pub fn new(mesh: MeshId) -> Box<MeshRenderer> {
        Box::new(MeshRenderer { 
            mesh,
            runtime_data: None,
        })
    }
    
    pub fn mesh(&self) -> MeshId {
        self.mesh
    }
}

impl Drawable for MeshRenderer {
    fn setup(
        &mut self,
        renderer: &Renderer,
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

        let device = renderer.state.device.as_ref();

        let mesh_data = ModelData::empty();
        let mesh_data_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Model Buffer"),
            contents: bytemuck::bytes_of(&mesh_data),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let bones = world.assets.meshes
            .get_raw_mesh(self.mesh)
            .expect("Mesh must exist")
            .bones
            .bones()
            .to_vec();

        let bone_data = BoneData {
            bones,
        };
        
        let bone_data_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Model Bone Data"),
            contents: bone_data.as_bytes(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let model_bind_group_layout = world.assets.bind_group_layouts
            .get_bind_group_layout(MODEL_UBGL_ID)
            .expect("Model BGL should exist");

        let model_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Model Bind Group"),
            layout: model_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: mesh_data_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: bone_data_buffer.as_entire_binding(),
                }
            ],
        });

       let runtime_data = RuntimeMeshRenderData {
            mesh_data,
            mesh_data_buffer,

            bone_data,
            bone_data_buffer,

            model_bind_group,
        };

       self.runtime_data = Some(runtime_data);
    }

    fn update(
        &mut self,
        _world: &mut World,
        parent: GameObjectId,
        renderer: &Renderer,
        outer_transform: &Matrix4<f32>,
    ) {
        let runtime_data = self.runtime_data.as_mut().expect("Should be initialized in init");

        runtime_data
            .mesh_data
            .update(parent, outer_transform);

        renderer.state.queue.write_buffer(
            &runtime_data.mesh_data_buffer,
            0,
            bytemuck::bytes_of(&runtime_data.mesh_data),
        );

        if !runtime_data.bone_data.bones.is_empty() {
            renderer.state.queue.write_buffer(
                &runtime_data.bone_data_buffer,
                0,
                runtime_data.bone_data.as_bytes(),
            );
        }
    }

    fn draw(&self, world: &mut World, rpass: &mut RenderPass, renderer: &Renderer, shader_override: Option<ShaderId>) {
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
                .get_shader(renderer.current_pipeline)
                .unwrap_or_else(|| {
                    error!("Passed in Default Pipeline is not available");
                    (*world)
                        .assets
                        .shaders
                        .get_shader(None)
                        .expect("Fallback shader should always exist")
            });

            let runtime_data = self.runtime_data.as_ref().expect("Should be initialized in init");

            rpass.set_vertex_buffer(0, runtime_mesh.data.vertices_buf.slice(..));
            rpass.set_bind_group(1, &runtime_data.model_bind_group, &[]);

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
                    .get_shader(shader_override.or(runtime_material.shader))
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
