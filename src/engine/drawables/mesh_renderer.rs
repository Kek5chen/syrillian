use crate::assets::{HShader, Mesh};
use crate::core::{Bone, GameObjectId, ModelUniform, Vertex3D};
use crate::drawables::Drawable;
use crate::engine::assets::HMesh;
use crate::engine::rendering::uniform::ShaderUniform;
use crate::engine::rendering::{DrawCtx, Renderer};
use crate::rendering::RuntimeMesh;
use crate::World;
use log::warn;
use nalgebra::{Matrix4, Vector3};
use std::sync::RwLockWriteGuard;
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

#[derive(Debug)]
pub struct RuntimeMeshData {
    mesh_data: ModelUniform,
    bone_data: BoneData,
    uniform: ShaderUniform<MeshUniformIndex>,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DebugVertexNormal {
    position: Vector3<f32>,
    normal: Vector3<f32>,
}

#[derive(Debug)]
pub struct MeshRenderer {
    mesh: HMesh,
    runtime_data: Option<RuntimeMeshData>,

    // Data needed for rendering the vertex normal lines in debug mode
    #[cfg(debug_assertions)]
    debug_data: Option<super::DebugRuntimePatternData>,
}

impl Drawable for MeshRenderer {
    fn setup(&mut self, renderer: &Renderer, world: &mut World) {
        self.setup_mesh_data(renderer, world);

        #[cfg(debug_assertions)]
        self.setup_debug_data(renderer, world);
    }

    fn update(
        &mut self,
        world: &mut World,
        parent: GameObjectId,
        renderer: &Renderer,
        outer_transform: &Matrix4<f32>,
    ) {
        self.update_mesh_data(world, parent, renderer, outer_transform);

        #[cfg(debug_assertions)]
        self.update_debug_data(world, renderer);
    }

    fn draw(&self, world: &mut World, ctx: &DrawCtx) {
        let Some(mesh) = ctx.frame.cache.mesh(self.mesh) else {
            return;
        };

        let Some(mesh_data) = world.assets.meshes.try_get(self.mesh) else {
            return;
        };

        let runtime_data = self
            .runtime_data
            .as_ref()
            .expect("Should be initialized in init");

        let mut pass = ctx.pass.write().unwrap();

        draw_mesh(ctx, &mesh, &mesh_data, &runtime_data, &mut pass, None);

        #[cfg(debug_assertions)]
        {
            if ctx.frame.debug.draw_edges {
                draw_mesh(
                    ctx,
                    &mesh,
                    &mesh_data,
                    &runtime_data,
                    &mut pass,
                    Some(HShader::DEBUG_EDGES),
                );
            }

            if ctx.frame.debug.draw_vertex_normals {
                let debug_data = self
                    .debug_data
                    .as_ref()
                    .expect("Should be initialized in init");

                draw_vertex_normals(ctx, &mesh_data, &runtime_data, &debug_data, &mut pass);
            }
        }
    }
}

impl MeshRenderer {
    pub fn new(mesh: HMesh) -> Box<MeshRenderer> {
        Box::new(MeshRenderer {
            mesh,
            runtime_data: None,

            #[cfg(debug_assertions)]
            debug_data: None,
        })
    }

    pub fn set_mesh(&mut self, mesh: HMesh) {
        self.mesh = mesh;
    }

    pub fn mesh(&self) -> HMesh {
        self.mesh
    }

    fn update_mesh_data(
        &mut self,
        world: &mut World,
        parent: GameObjectId,
        renderer: &Renderer,
        outer_transform: &Matrix4<f32>,
    ) {
        let runtime_data = match self.runtime_data.as_mut() {
            None => {
                self.setup_mesh_data(renderer, world);
                self.runtime_data.as_mut().unwrap()
            }
            Some(runtime_data) => runtime_data,
        };

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

    #[cfg(debug_assertions)]
    fn update_debug_data(&mut self, world: &mut World, renderer: &Renderer) {
        match self.debug_data.as_mut() {
            None => {
                self.setup_debug_data(renderer, world);
                self.debug_data.as_mut().unwrap()
            }
            Some(debug_data) => debug_data,
        };
    }

    fn setup_mesh_data(&mut self, renderer: &Renderer, world: &mut World) -> bool {
        let Some(mesh) = world.assets.meshes.try_get(self.mesh) else {
            warn!("Mesh not found. Can't render");
            return false;
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
        true
    }

    #[cfg(debug_assertions)]
    fn setup_debug_data(&mut self, renderer: &Renderer, world: &mut World) {
        use wgpu::BufferUsages;
        use wgpu::util::{BufferInitDescriptor, DeviceExt};

        let Some(mesh) = world.assets.meshes.try_get(self.mesh) else {
            warn!("Mesh not found. Can't render");
            return;
        };

        let device = renderer.state.device.as_ref();

        let vertex_normal_data: Vec<DebugVertexNormal> = mesh
            .data
            .vertices
            .iter()
            .map(|v| v.into())
            .collect::<Vec<_>>();

        // instance buffer, 0 for base, 1 for tip of vertex normal line
        let pattern_buf = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Vertex Normal Debug Indices"),
            contents: bytemuck::cast_slice(&[0u32, 1u32]),
            usage: BufferUsages::VERTEX,
        });

        let vertices_buf = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Vertex Normal Debug Vertices"),
            contents: bytemuck::cast_slice(&vertex_normal_data[..]),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });

        let debug_data = super::DebugRuntimePatternData {
            pattern_buf,
            vertices_buf,
        };

        self.debug_data = Some(debug_data);
    }
}

fn draw_mesh(
    ctx: &DrawCtx,
    mesh: &RuntimeMesh,
    mesh_data: &Mesh,
    runtime_data: &RuntimeMeshData,
    pass: &mut RwLockWriteGuard<RenderPass>,
    shader_override: Option<HShader>,
) {
    pass.set_vertex_buffer(0, mesh.vertices_buf.slice(..));
    pass.set_bind_group(1, runtime_data.uniform.bind_group(), &[]);

    let i_buffer = mesh.indices_buf.as_ref();

    let shader_override = shader_override.map(|h| ctx.frame.cache.shader(h));
    let has_override = shader_override.is_some();
    if let Some(shader) = shader_override.as_ref() {
        pass.set_pipeline(&shader.pipeline);
    }

    for (h_mat, range) in &mesh_data.material_ranges {
        let material = ctx.frame.cache.material(*h_mat);

        if !has_override {
            let shader = ctx.frame.cache.shader(material.shader);
            pass.set_pipeline(&shader.pipeline);
        }

        pass.set_bind_group(2, material.uniform.bind_group(), &[]);

        if let Some(i_buffer) = i_buffer {
            pass.set_index_buffer(i_buffer.slice(..), IndexFormat::Uint32);
            pass.draw_indexed(range.clone(), 0, 0..1);
        } else {
            pass.draw(range.clone(), 0..1);
        }
    }
}

#[cfg(debug_assertions)]
fn draw_vertex_normals(
    ctx: &DrawCtx,
    mesh_data: &Mesh,
    runtime_data: &RuntimeMeshData,
    debug_data: &super::DebugRuntimePatternData,
    pass: &mut RwLockWriteGuard<RenderPass>,
) {
    pass.set_vertex_buffer(0, debug_data.pattern_buf.slice(..));
    pass.set_vertex_buffer(1, debug_data.vertices_buf.slice(..));
    pass.set_bind_group(1, runtime_data.uniform.bind_group(), &[]);

    let shader = ctx.frame.cache.shader(HShader::DEBUG_VERTEX_NORMALS);
    pass.set_pipeline(&shader.pipeline);

    pass.draw(0..2, 0..mesh_data.vertex_count() as u32);
}

impl From<&Vertex3D> for DebugVertexNormal {
    fn from(value: &Vertex3D) -> Self {
        DebugVertexNormal {
            position: value.position,
            normal: value.normal,
        }
    }
}
