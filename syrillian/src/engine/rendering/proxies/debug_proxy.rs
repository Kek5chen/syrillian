use crate::assets::{AssetStore, HMesh, HShader};
use crate::components::BoneData;
use crate::core::ModelUniform;
use crate::rendering::proxies::mesh_proxy::{MeshUniformIndex, RuntimeMeshData};
use crate::rendering::proxies::{PROXY_PRIORITY_SOLID, SceneProxy};
use crate::rendering::uniform::ShaderUniform;
use crate::rendering::{AssetCache, GPUDrawCtx, Renderer};
use crate::{must_pipeline, proxy_data, proxy_data_mut};
use log::warn;
use nalgebra::{Matrix4, Point3, Vector4};
use std::any::Any;
use syrillian_utils::debug_panic;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{Buffer, BufferUsages, Device, IndexFormat, Queue, ShaderStages};

#[derive(Debug)]
pub(crate) struct GPUDebugProxyData {
    line_data: Option<Buffer>,
    model_uniform: Option<RuntimeMeshData>,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DebugLine {
    pub start: Point3<f32>,
    pub end: Point3<f32>,
    pub start_color: Vector4<f32>,
    pub end_color: Vector4<f32>,
}

#[derive(Debug)]
pub struct DebugSceneProxy {
    pub lines: Vec<DebugLine>,
    pub meshes: Vec<HMesh>,
    pub color: Vector4<f32>,
    pub override_transform: Option<Matrix4<f32>>,
}

impl SceneProxy for DebugSceneProxy {
    fn setup_render(&mut self, renderer: &Renderer, model_mat: &Matrix4<f32>) -> Box<dyn Any> {
        let line_data = self.new_line_buffer(&renderer.state.device);
        let transform = self.override_transform.unwrap_or(*model_mat);
        let model_uniform =
            self.new_mesh_buffer(&renderer.cache, &renderer.state.device, &transform);

        Box::new(GPUDebugProxyData {
            line_data,
            model_uniform,
        })
    }

    fn update_render(
        &mut self,
        renderer: &Renderer,
        data: &mut dyn Any,
        local_to_world: &Matrix4<f32>,
    ) {
        let data: &mut GPUDebugProxyData = proxy_data_mut!(data);

        // TODO: Reuse or Resize buffer
        data.line_data = self.new_line_buffer(&renderer.state.device);

        let transform = self.override_transform.unwrap_or(*local_to_world);
        self.update_mesh_buffer(
            data,
            &renderer.cache,
            &renderer.state.device,
            &renderer.state.queue,
            &transform,
        );
    }

    fn render(
        &self,
        renderer: &Renderer,
        data: &dyn Any,
        ctx: &GPUDrawCtx,
        _local_to_world: &Matrix4<f32>,
    ) {
        let data = proxy_data!(data);
        let cache = &renderer.cache;
        self.render_lines(data, cache, ctx);
        self.render_meshes(data, cache, ctx)
    }

    fn priority(&self, _store: &AssetStore) -> u32 {
        PROXY_PRIORITY_SOLID
    }
}

impl Default for DebugSceneProxy {
    fn default() -> Self {
        Self {
            lines: vec![],
            meshes: vec![],
            color: Vector4::new(1.0, 1.0, 1.0, 1.0),
            override_transform: None,
        }
    }
}

impl DebugSceneProxy {
    fn new_line_buffer(&self, device: &Device) -> Option<Buffer> {
        if self.lines.is_empty() {
            return None;
        }

        Some(device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Debug Ray Data Buffer"),
            contents: bytemuck::cast_slice(&self.lines[..]),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        }))
    }

    fn new_mesh_buffer(
        &self,
        cache: &AssetCache,
        device: &Device,
        model_mat: &Matrix4<f32>,
    ) -> Option<RuntimeMeshData> {
        if self.meshes.is_empty() {
            return None;
        }

        let bgl = cache.bgl_model();
        let mesh_data = ModelUniform {
            model_mat: *model_mat,
        };
        let uniform = ShaderUniform::builder(&bgl)
            .with_buffer_data(&mesh_data)
            .with_buffer_data(&BoneData::DUMMY)
            .build(device);

        Some(RuntimeMeshData { mesh_data, uniform })
    }

    fn update_mesh_buffer(
        &self,
        data: &mut GPUDebugProxyData,
        cache: &AssetCache,
        device: &Device,
        queue: &Queue,
        model_mat: &Matrix4<f32>,
    ) {
        if self.meshes.is_empty() {
            return;
        }

        let model_uniform = match data.model_uniform.take() {
            None => self.new_mesh_buffer(cache, device, model_mat),
            Some(mut model_uniform) => {
                model_uniform.mesh_data.model_mat = *model_mat;
                let mesh_buffer = model_uniform.uniform.buffer(MeshUniformIndex::MeshData);
                queue.write_buffer(mesh_buffer, 0, bytemuck::bytes_of(&model_uniform.mesh_data));
                Some(model_uniform)
            }
        };

        data.model_uniform = model_uniform;
    }

    pub fn single_mesh(mesh: HMesh) -> Self {
        let mut proxy = Self::default();
        proxy.meshes.push(mesh);
        proxy
    }

    pub fn set_override_transform(&mut self, transform: Matrix4<f32>) {
        self.override_transform = Some(transform);
    }

    fn render_lines(&self, data: &GPUDebugProxyData, cache: &AssetCache, ctx: &GPUDrawCtx) {
        if self.lines.is_empty() {
            return;
        }

        let Some(line_buffer) = &data.line_data else {
            debug_panic!("Lines exist but line buffer was not prepared when rendering.");
            return;
        };

        let mut pass = ctx.pass.write().unwrap();

        pass.set_vertex_buffer(0, line_buffer.slice(..));

        let shader = cache.shader(HShader::DEBUG_LINES);
        must_pipeline!(pipeline = shader, ctx.pass_type => return);

        pass.set_pipeline(pipeline);

        pass.draw(0..2, 0..self.lines.len() as u32);
    }

    fn render_meshes(&self, data: &GPUDebugProxyData, cache: &AssetCache, ctx: &GPUDrawCtx) {
        if self.meshes.is_empty() {
            return;
        }

        let Some(data) = &data.model_uniform else {
            debug_panic!("Meshes exist but mesh buffer was not prepared when rendering.");
            return;
        };

        for mesh in self.meshes.iter().copied() {
            let Some(runtime_mesh) = cache.meshes.try_get(mesh, cache) else {
                warn!("Couldn't render {}", mesh.ident_fmt());
                continue;
            };

            let shader = cache.shader(HShader::DEBUG_EDGES);
            let groups = shader.bind_groups();
            must_pipeline!(pipeline = shader, ctx.pass_type => return);

            let mut pass = ctx.pass.write().unwrap();

            pass.set_pipeline(pipeline);
            pass.set_push_constants(ShaderStages::FRAGMENT, 0, bytemuck::bytes_of(&self.color));
            pass.set_bind_group(groups.render, ctx.render_bind_group, &[]);
            if let Some(idx) = groups.model {
                pass.set_bind_group(idx, data.uniform.bind_group(), &[]);
            }

            pass.set_vertex_buffer(0, runtime_mesh.vertex_buffer().slice(..));
            if let Some(idx_buf) = &runtime_mesh.indices_buffer() {
                pass.set_index_buffer(idx_buf.slice(..), IndexFormat::Uint32);
                pass.draw_indexed(0..runtime_mesh.indices_count(), 0, 0..1);
            } else {
                pass.draw(0..runtime_mesh.vertex_count(), 0..1);
            }
        }
    }
}
