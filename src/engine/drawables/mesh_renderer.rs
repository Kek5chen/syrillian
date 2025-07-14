use crate::World;
use crate::core::{Bone, GameObjectId, ModelUniform};
use crate::drawables::Drawable;
use crate::engine::assets::HMesh;
use crate::engine::rendering::uniform::ShaderUniform;
use crate::engine::rendering::{DrawCtx, Renderer};
use log::warn;
use nalgebra::Matrix4;
use syrillian_macros::UniformIndex;
use wgpu::IndexFormat;

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
    mesh: HMesh,
    runtime_data: Option<RuntimeMeshData>,
}

impl MeshRenderer {
    pub fn new(mesh: HMesh) -> Box<MeshRenderer> {
        Box::new(MeshRenderer {
            mesh,
            runtime_data: None,
        })
    }

    pub fn mesh(&self) -> HMesh {
        self.mesh
    }
}

impl Drawable for MeshRenderer {
    fn setup(&mut self, renderer: &Renderer, world: &mut World) {
        let Some(mesh) = world.assets.meshes.try_get(self.mesh) else {
            warn!("Mesh not found. Can't render");
            return;
        };

        let device = renderer.state.device.as_ref();
        let model_bgl = renderer.cache.bgl_model();
        let mesh_data = ModelUniform::empty();

        let bones = mesh.bones.as_slice().to_vec();

        let bone_data = BoneData { bones };

        let uniform = ShaderUniform::<MeshUniformIndex>::builder(&model_bgl)
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

    fn draw(&self, world: &mut World, ctx: &DrawCtx) {
        let Some(mesh) = ctx.frame.cache.mesh(self.mesh) else {
            return;
        };

        let Some(mesh_data) = world.assets.meshes.try_get(self.mesh) else {
            return;
        };

        let default_shader = ctx.frame.cache.shader_3d();

        let runtime_data = self
            .runtime_data
            .as_ref()
            .expect("Should be initialized in init");

        let mut pass = ctx.pass.write().unwrap();

        pass.set_vertex_buffer(0, mesh.vertices_buf.slice(..));
        pass.set_bind_group(1, runtime_data.uniform.bind_group(), &[]);

        let i_buffer = &mesh.indices_buf.as_ref();

        for (h_mat, range) in &mesh_data.material_ranges {
            let material = ctx.frame.cache.material(*h_mat);

            let shader = ctx
                .shader_override
                .or(material.shader)
                .map(|id| ctx.frame.cache.shader(id));

            let shader = shader.as_ref().unwrap_or(&default_shader);

            pass.set_pipeline(&shader.pipeline);
            pass.set_bind_group(2, material.uniform.bind_group(), &[]);

            if let Some(i_buffer) = i_buffer {
                pass.set_index_buffer(i_buffer.slice(..), IndexFormat::Uint32);
                pass.draw_indexed(range.clone(), 0, 0..1);
            } else {
                pass.draw(range.clone(), 0..1);
            }
        }
    }
}
