use crate::assets::{HShader, Mesh, Shader, H};
use crate::components::RigidBodyComponent;
use crate::core::{Bone, GameObjectId, ModelUniform, Vertex3D};
use crate::drawables::Drawable;
use crate::engine::assets::HMesh;
use crate::engine::rendering::uniform::ShaderUniform;
use crate::engine::rendering::{DrawCtx, Renderer};
use crate::rendering::{RuntimeMesh, RuntimeMeshData};
use crate::{must_pipeline, World};
use log::warn;
use nalgebra::{Matrix4, Vector3};
use std::sync::RwLockWriteGuard;
use syrillian_macros::UniformIndex;
use wgpu::{Buffer, IndexFormat, RenderPass};

#[repr(u8)]
#[derive(Copy, Clone, Debug, UniformIndex)]
pub enum MeshUniformIndex {
    MeshData = 0,
    BoneData = 1,
}

// FIXME: The shader currently only accepts one bone for some reason.
//        Having more than 1 bone will crash-...
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

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DebugVertexNormal {
    position: Vector3<f32>,
    normal: Vector3<f32>,
}

// TODO: Just use the same bind and add a array stride to the vertex normal shader
//       This is possible now that i discovered the builtin vertex / instance ids :)
//       No separate buffer, just share the same bind! wee
#[derive(Debug)]
#[cfg(debug_assertions)]
struct RuntimeDebugData {
    mesh_vertices_buf: wgpu::Buffer,
}

#[derive(Debug)]
pub struct MeshRenderer {
    mesh: HMesh,
    runtime_data: Option<RuntimeMeshData>,

    // Data needed for rendering debug stuff
    #[cfg(debug_assertions)]
    debug_data: Option<RuntimeDebugData>,
}

impl Drawable for MeshRenderer {
    fn setup(&mut self, renderer: &Renderer, world: &mut World, _parent: GameObjectId) {
        self.setup_mesh_data(renderer, world);

        #[cfg(debug_assertions)]
        self.setup_debug_data(renderer, world, _parent);
    }

    fn update(
        &mut self,
        world: &mut World,
        parent: GameObjectId,
        renderer: &Renderer,
        transform: &Matrix4<f32>,
    ) {
        self.update_mesh_data(world, parent, renderer, transform);

        #[cfg(debug_assertions)]
        self.update_debug_data(world, renderer, parent);
    }

    fn draw(&self, world: &World, ctx: &DrawCtx) {
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

        pass.set_bind_group(1, runtime_data.uniform.bind_group(), &[]);

        draw_mesh(ctx, &mesh, &mesh_data, &mut pass);

        #[cfg(debug_assertions)]
        {
            let debug_data = self
                .debug_data
                .as_ref()
                .expect("Should be initialized in init");

            if ctx.frame.debug.mesh_edges {
                draw_edges(ctx, &mesh, &mesh_data, &mut pass);
            }

            if ctx.frame.debug.vertex_normals {
                draw_vertex_normals(ctx, &mesh_data, &debug_data, &mut pass);
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
        transform: &Matrix4<f32>,
    ) {
        let runtime_data = match self.runtime_data.as_mut() {
            None => {
                self.setup_mesh_data(renderer, world);
                self.runtime_data.as_mut().unwrap()
            }
            Some(runtime_data) => runtime_data,
        };

        let mut world_m = *transform;

        if let Some(rb) = parent.get_component::<RigidBodyComponent>() {
            let iso = rb.render_isometry(world.physics.alpha);
            world_m = iso.to_homogeneous();
        }

        runtime_data.mesh_data.update(&world_m);

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
    fn update_debug_data(&mut self, world: &mut World, renderer: &Renderer, parent: GameObjectId) {
        match self.debug_data.as_mut() {
            None => {
                self.setup_debug_data(renderer, world, parent);
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
    fn setup_debug_data(&mut self, renderer: &Renderer, world: &mut World, _parent: GameObjectId) {
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

        let vertices_buf = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Vertex Normal Debug Vertices"),
            contents: bytemuck::cast_slice(&vertex_normal_data[..]),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });

        let debug_data = RuntimeDebugData {
            mesh_vertices_buf: vertices_buf,
        };

        self.debug_data = Some(debug_data);
    }

    pub fn mesh_data(&self) -> Option<&RuntimeMeshData> {
        self.runtime_data.as_ref()
    }
}

fn draw_mesh(
    ctx: &DrawCtx,
    mesh: &RuntimeMesh,
    mesh_data: &Mesh,
    pass: &mut RwLockWriteGuard<RenderPass>,
) {
    let i_buffer = mesh.indices_buf.as_ref();
    let current_shader = HShader::DIM3;
    let shader = ctx.frame.cache.shader_3d();

    must_pipeline!(pipeline = shader, ctx.pass_type => return);

    pass.set_pipeline(pipeline);

    pass.set_vertex_buffer(0, mesh.vertices_buf.slice(..));
    if let Some(i_buffer) = i_buffer {
        pass.set_index_buffer(i_buffer.slice(..), IndexFormat::Uint32);
    }

    draw_materials(ctx, mesh_data, pass, i_buffer, current_shader);
}

fn draw_materials(
    ctx: &DrawCtx,
    mesh_data: &Mesh,
    pass: &mut RwLockWriteGuard<RenderPass>,
    i_buffer: Option<&Buffer>,
    current_shader: H<Shader>,
) {
    for (h_mat, range) in mesh_data.material_ranges.iter().cloned() {
        let material = ctx.frame.cache.material(h_mat);

        if material.shader != current_shader {
            let shader = ctx.frame.cache.shader(material.shader);
            must_pipeline!(pipeline = shader, ctx.pass_type => continue);

            pass.set_pipeline(&pipeline);
        }

        pass.set_bind_group(2, material.uniform.bind_group(), &[]);

        if i_buffer.is_some() {
            pass.draw_indexed(range.clone(), 0, 0..1);
        } else {
            pass.draw(range.clone(), 0..1);
        }
    }
}

#[cfg(debug_assertions)]
fn draw_edges(
    ctx: &DrawCtx,
    mesh: &RuntimeMesh,
    mesh_data: &Mesh,
    pass: &mut RwLockWriteGuard<RenderPass>,
) {
    use nalgebra::Vector4;
    use wgpu::ShaderStages;

    const COLOR: Vector4<f32> = Vector4::new(1.0, 0.0, 1.0, 1.0);

    let shader = ctx.frame.cache.shader(HShader::DEBUG_EDGES);
    must_pipeline!(pipeline = shader, ctx.pass_type => return);

    pass.set_pipeline(pipeline);
    pass.set_vertex_buffer(0, mesh.vertices_buf.slice(..));
    pass.set_push_constants(ShaderStages::FRAGMENT, 0, bytemuck::bytes_of(&COLOR));

    if let Some(i_buffer) = mesh.indices_buf.as_ref() {
        pass.set_index_buffer(i_buffer.slice(..), IndexFormat::Uint32);
        pass.draw_indexed(0..mesh_data.indices_count() as u32, 0, 0..1);
    } else {
        pass.draw(0..mesh_data.vertex_count() as u32, 0..1);
    }
}

#[cfg(debug_assertions)]
fn draw_vertex_normals(
    ctx: &DrawCtx,
    mesh_data: &Mesh,
    debug_data: &RuntimeDebugData,
    pass: &mut RwLockWriteGuard<RenderPass>,
) {
    pass.set_vertex_buffer(0, debug_data.mesh_vertices_buf.slice(..));

    let shader = ctx.frame.cache.shader(HShader::DEBUG_VERTEX_NORMALS);
    must_pipeline!(pipeline = shader, ctx.pass_type => return);

    pass.set_pipeline(pipeline);

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
