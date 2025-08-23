use crate::assets::{HMaterial, HMesh, HShader, Mesh, Shader, H};
use crate::components::BoneData;
use crate::core::ModelUniform;
use crate::rendering::proxies::SceneProxy;
use crate::rendering::uniform::ShaderUniform;
use crate::rendering::{AssetCache, GPUDrawCtx, Renderer, RuntimeMesh};
use crate::{must_pipeline, proxy_data, proxy_data_mut};
use nalgebra::Matrix4;
use std::any::Any;
use std::sync::RwLockWriteGuard;
use syrillian_macros::UniformIndex;
use wgpu::{IndexFormat, RenderPass};
use winit::window::Window;

#[cfg(debug_assertions)]
use crate::rendering::DebugRenderer;

#[repr(u8)]
#[derive(Copy, Clone, Debug, UniformIndex)]
pub enum MeshUniformIndex {
    MeshData = 0,
    BoneData = 1,
}

#[derive(Debug)]
pub struct RuntimeMeshData {
    pub mesh_data: ModelUniform,
    // TODO: Consider having a uniform like that, for every Transform by default in some way, or
    //       lazy-make / provide one by default.
    pub uniform: ShaderUniform<MeshUniformIndex>,
}

pub struct MeshSceneProxy {
    pub mesh: HMesh,
    pub materials: Vec<HMaterial>,
    pub bone_data: BoneData,
    pub bones_dirty: bool,
}

impl SceneProxy for MeshSceneProxy {
    fn setup_render(&mut self, renderer: &Renderer, local_to_world: &Matrix4<f32>) -> Box<dyn Any> {
        Box::new(self.setup_mesh_data(renderer, local_to_world))
    }

    fn update_render(&mut self, renderer: &Renderer, data: &mut dyn Any, _window: &Window, local_to_world: &Matrix4<f32>) {
        let data: &mut RuntimeMeshData = proxy_data_mut!(data);

        // TODO: Consider Rigid Body render isometry interpolation for mesh local to world

        if self.bones_dirty {
            renderer.state.queue.write_buffer(
                &data.uniform.buffer(MeshUniformIndex::BoneData),
                0,
                self.bone_data.as_bytes(),
            );
            self.bones_dirty = false;
        }

        data.mesh_data.model_mat = *local_to_world;

        renderer.state.queue.write_buffer(
            &data.uniform.buffer(MeshUniformIndex::MeshData),
            0,
            bytemuck::bytes_of(&data.mesh_data),
        );
    }

    fn render<'a>(&self, renderer: &Renderer, data: &dyn Any, ctx: &GPUDrawCtx, _local_to_world: &Matrix4<f32>) {
        let data: &RuntimeMeshData = proxy_data!(data);

        let Some(mesh) = renderer.cache.mesh(self.mesh) else {
            return;
        };

        let Some(mesh_data) = renderer.cache.meshes.store().try_get(self.mesh) else {
            return;
        };

        let mut pass = ctx.pass.write().unwrap();

        pass.set_bind_group(1, data.uniform.bind_group(), &[]);

        self.draw_mesh(ctx, &renderer.cache, &mesh, &mesh_data, &mut pass);

        #[cfg(debug_assertions)]
        if DebugRenderer::mesh_edges() {
            draw_edges(ctx, &renderer.cache, &mesh, &mut pass);
        }

        #[cfg(debug_assertions)]
        if DebugRenderer::mesh_vertex_normals() {
            draw_vertex_normals(ctx, &renderer.cache, &mesh, &mut pass);
        }
    }
}

impl MeshSceneProxy {
    fn draw_mesh(
        &self,
        ctx: &GPUDrawCtx,
        cache: &AssetCache,
        mesh: &RuntimeMesh,
        mesh_data: &Mesh,
        pass: &mut RwLockWriteGuard<RenderPass>,
    ) {
        let current_shader = HShader::DIM3;
        let shader = cache.shader_3d();

        must_pipeline!(pipeline = shader, ctx.pass_type => return);

        pass.set_pipeline(pipeline);

        pass.set_vertex_buffer(0, mesh.vertex_buffer().slice(..));
        if let Some(i_buffer) = mesh.indices_buffer() {
            pass.set_index_buffer(i_buffer.slice(..), IndexFormat::Uint32);
        }

        self.draw_materials(ctx, cache, mesh_data, pass, current_shader);
    }

    fn draw_materials(
        &self,
        ctx: &GPUDrawCtx,
        cache: &AssetCache,
        mesh_data: &Mesh,
        pass: &mut RwLockWriteGuard<RenderPass>,
        current_shader: H<Shader>,
    ) {
        for (i, range) in mesh_data.material_ranges.iter().enumerate() {
            let h_mat = self
                .materials
                .get(i)
                .cloned()
                .unwrap_or(HMaterial::FALLBACK);
            let material = cache.material(h_mat);

            if material.shader != current_shader {
                let shader = cache.shader(material.shader);
                must_pipeline!(pipeline = shader, ctx.pass_type => continue);

                pass.set_pipeline(&pipeline);
            }

            pass.set_bind_group(2, material.uniform.bind_group(), &[]);

            if mesh_data.has_indices() {
                pass.draw_indexed(range.clone(), 0, 0..1);
            } else {
                pass.draw(range.clone(), 0..1);
            }
        }
    }

    fn setup_mesh_data(&mut self, renderer: &Renderer, local_to_world: &Matrix4<f32>) -> RuntimeMeshData {
        let device = &renderer.state.device;
        let model_bgl = renderer.cache.bgl_model();
        let mesh_data = ModelUniform::from_matrix(local_to_world);

        let uniform = ShaderUniform::<MeshUniformIndex>::builder(&model_bgl)
            .with_buffer_data(&mesh_data)
            .with_buffer_data_slice(self.bone_data.bones.as_slice())
            .build(device);

        RuntimeMeshData {
            mesh_data,
            uniform,
        }
    }
}

#[cfg(debug_assertions)]
fn draw_edges(
    ctx: &GPUDrawCtx,
    cache: &AssetCache,
    mesh: &RuntimeMesh,
    pass: &mut RwLockWriteGuard<RenderPass>,
) {
    use nalgebra::Vector4;
    use wgpu::ShaderStages;

    const COLOR: Vector4<f32> = Vector4::new(1.0, 0.0, 1.0, 1.0);

    let shader = cache.shader(HShader::DEBUG_EDGES);
    must_pipeline!(pipeline = shader, ctx.pass_type => return);

    pass.set_pipeline(pipeline);
    pass.set_vertex_buffer(0, mesh.vertex_buffer().slice(..));
    pass.set_push_constants(ShaderStages::FRAGMENT, 0, bytemuck::bytes_of(&COLOR));

    if let Some(i_buffer) = mesh.indices_buffer().as_ref() {
        pass.set_index_buffer(i_buffer.slice(..), IndexFormat::Uint32);
        pass.draw_indexed(0..mesh.indices_count(), 0, 0..1);
    } else {
        pass.draw(0..mesh.vertex_count(), 0..1);
    }
}

#[cfg(debug_assertions)]
fn draw_vertex_normals(
    ctx: &GPUDrawCtx,
    cache: &AssetCache,
    mesh: &RuntimeMesh,
    pass: &mut RwLockWriteGuard<RenderPass>,
) {
    pass.set_vertex_buffer(0, mesh.vertex_buffer().slice(..));

    let shader = cache.shader(HShader::DEBUG_VERTEX_NORMALS);
    must_pipeline!(pipeline = shader, ctx.pass_type => return);

    pass.set_pipeline(pipeline);

    if let Some(i_buffer) = mesh.indices_buffer().as_ref() {
        pass.set_index_buffer(i_buffer.slice(..), IndexFormat::Uint32);
        pass.draw_indexed(0..2, 0, 0..mesh.indices_count());
    } else {
        pass.draw(0..2, 0..mesh.vertex_count());
    }
}

