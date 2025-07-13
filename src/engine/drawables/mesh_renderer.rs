use crate::World;
use crate::asset_management::{FALLBACK_MATERIAL_ID, MODEL_UBGL_ID, MaterialId, MeshId, ShaderId};
use crate::core::{Bone, GameObjectId, ModelUniform};
use crate::drawables::Drawable;
use crate::engine::rendering::Renderer;
use crate::engine::rendering::uniform::ShaderUniform;
use nalgebra::Matrix4;
use syrillian_macros::UniformIndex;
use wgpu::{IndexFormat, RenderPass};

#[repr(u8)]
#[derive(Copy, Clone, Debug, UniformIndex)]
pub enum MeshUniformIndex {
    MeshData = 0,
    BoneData = 1,
}

#[derive(Debug, Default, Clone)]
pub struct BoneData {
    pub(crate) bones: Vec<Bone>,
}

impl BoneData {
    #[rustfmt::skip]
    pub const DUMMY_BONE: [Bone; 1] = [Bone {
        transform: Matrix4::new(
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0
        )
    }];

    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(self.as_slice())
    }

    pub fn as_slice(&self) -> &[Bone] {
        if self.bones.is_empty() {
            &Self::DUMMY_BONE
        } else {
            &self.bones[..]
        }
    }
}

pub struct RuntimeMeshData {
    mesh_data: ModelUniform,
    bone_data: BoneData,
    uniform: ShaderUniform<MeshUniformIndex>,
}

pub struct MeshRenderer {
    mesh: MeshId,
    runtime_data: Option<RuntimeMeshData>,
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
    fn setup(&mut self, renderer: &Renderer, world: &mut World) {
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
                    .init_runtime_material_id(&mut (*world), mat_id)
                    .expect("Runtime material should be initialized..");
            }
        }

        let device = renderer.state.device.as_ref();

        let model_bgl = world
            .assets
            .bind_group_layouts
            .get_bind_group_layout(MODEL_UBGL_ID)
            .expect("Model BGL should exist");

        let mesh_data = ModelUniform::empty();

        let bones = world
            .assets
            .meshes
            .get_raw_mesh(self.mesh)
            .expect("Mesh must exist")
            .bones
            .bones()
            .to_vec();

        let bone_data = BoneData { bones };

        let uniform = ShaderUniform::<MeshUniformIndex>::builder(model_bgl)
            .with_buffer_data(&mesh_data)
            .with_buffer_data_slice(bone_data.as_slice())
            .build(device);

        let runtime_data = RuntimeMeshData {
            mesh_data,
            bone_data,
            uniform,
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
        let runtime_data = self
            .runtime_data
            .as_mut()
            .expect("Should be initialized in init");

        runtime_data.mesh_data.update(parent, outer_transform);

        renderer.state.queue.write_buffer(
            &runtime_data.uniform.buffer(MeshUniformIndex::MeshData),
            0,
            bytemuck::bytes_of(&runtime_data.mesh_data),
        );

        if !runtime_data.bone_data.bones.is_empty() {
            renderer.state.queue.write_buffer(
                &runtime_data.uniform.buffer(MeshUniformIndex::BoneData),
                0,
                runtime_data.bone_data.as_bytes(),
            );
        }
    }

    fn draw(
        &self,
        world: &mut World,
        rpass: &mut RenderPass,
        renderer: &Renderer,
        shader_override: Option<ShaderId>,
    ) {
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
                .get_shader(renderer.current_pipeline);

            let runtime_data = self
                .runtime_data
                .as_ref()
                .expect("Should be initialized in init");

            rpass.set_vertex_buffer(0, runtime_mesh.data.vertices_buf.slice(..));
            rpass.set_bind_group(1, runtime_data.uniform.bind_group(), &[]);

            let i_buffer = &runtime_mesh.data.indices_buf.as_ref();

            for (mat_id, range) in &mesh.material_ranges {
                let runtime_material = (*world)
                    .assets
                    .materials
                    .get_runtime_material(*mat_id)
                    .unwrap_or_else(|| {
                        (*world)
                            .assets
                            .materials
                            .get_runtime_material(FALLBACK_MATERIAL_ID)
                            .unwrap()
                    });

                let shader = shader_override
                    .or(runtime_material.shader)
                    .map(|id| (*world).assets.shaders.get_shader(Some(id)))
                    .unwrap_or(default_shader);

                rpass.set_pipeline(&shader.pipeline);
                rpass.set_bind_group(2, runtime_material.uniform.bind_group(), &[]);

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
