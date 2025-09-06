use crate::assets::{AssetStore, HMaterial};
use crate::components::{BoneData, ImageScalingMode};
use crate::core::ModelUniform;
use crate::rendering::proxies::mesh_proxy::{MeshUniformIndex, RuntimeMeshData};
use crate::rendering::proxies::{PROXY_PRIORITY_2D, SceneProxy};
use crate::rendering::uniform::ShaderUniform;
use crate::rendering::{GPUDrawCtx, RenderPassType, Renderer};
use crate::{proxy_data, proxy_data_mut};
use nalgebra::{Matrix4, Scale3, Translation3};
use std::any::Any;
use winit::window::Window;

#[derive(Debug)]
pub struct ImageSceneProxy {
    pub translation: Matrix4<f32>,
    pub material: HMaterial,
    pub scaling: ImageScalingMode,
    pub dirty: bool,
}

impl SceneProxy for ImageSceneProxy {
    fn setup_render(&mut self, renderer: &Renderer, local_to_world: &Matrix4<f32>) -> Box<dyn Any> {
        let bgl = renderer.cache.bgl_model();
        let mesh_data = ModelUniform::from_matrix(local_to_world);
        let uniform = ShaderUniform::builder(&bgl)
            .with_buffer_data(&mesh_data)
            .with_buffer_data(&BoneData::DUMMY)
            .build(&renderer.state.device);

        Box::new(RuntimeMeshData { mesh_data, uniform })
    }

    fn update_render(
        &mut self,
        renderer: &Renderer,
        data: &mut dyn Any,
        window: &Window,
        _local_to_world: &Matrix4<f32>,
    ) {
        let data: &mut RuntimeMeshData = proxy_data_mut!(data);

        let window_size = window.inner_size();
        let width = window_size.width as f32;
        let height = window_size.height as f32;

        data.mesh_data.model_mat = self.calculate_model_matrix(width, height);

        renderer.state.queue.write_buffer(
            data.uniform.buffer(MeshUniformIndex::MeshData),
            0,
            bytemuck::bytes_of(&data.mesh_data),
        );
    }

    fn render<'a>(
        &self,
        renderer: &Renderer,
        data: &dyn Any,
        ctx: &GPUDrawCtx,
        _local_to_world: &Matrix4<f32>,
    ) {
        if ctx.pass_type == RenderPassType::Shadow {
            return; // Don't render shadows for 2D
        }

        let data: &RuntimeMeshData = proxy_data!(data);

        let unit_square_runtime = renderer.cache.mesh_unit_square();
        let material = renderer.cache.material(self.material);
        let shader = renderer.cache.shader_2d();

        let mut pass = ctx.pass.write().unwrap();

        pass.set_pipeline(shader.solid_pipeline());

        let vertex_buf_slice = unit_square_runtime.vertex_buffer().slice(..);
        let material_bind_group = material.uniform.bind_group();
        let vertices_count = unit_square_runtime.vertex_count();

        pass.set_vertex_buffer(0, vertex_buf_slice);
        pass.set_bind_group(1, data.uniform.bind_group(), &[]);
        pass.set_bind_group(2, material_bind_group, &[]);
        pass.draw(0..vertices_count, 0..1)
    }

    fn priority(&self, _store: &AssetStore) -> u32 {
        PROXY_PRIORITY_2D
    }
}

impl ImageSceneProxy {
    #[rustfmt::skip]
    fn calculate_model_matrix_absolute(&self, window_width: f32, window_height: f32) -> Matrix4<f32> {
        let ImageScalingMode::Absolute { left, right, top, bottom } = self.scaling else {
            return Matrix4::zeros();
        };

        if right <= left || top <= bottom {
            return Matrix4::zeros();
        }

        let left   = (left   as f32 / window_width)  * 2.0 - 1.0;
        let right  = (right  as f32 / window_width)  * 2.0 - 1.0;
        let bottom = (bottom as f32 / window_height) * 2.0 - 1.0;
        let top    = (top    as f32 / window_height) * 2.0 - 1.0;

        let sx = (right - left) * 0.5;
        let sy = (top - bottom) * 0.5;

        // clip space
        let tx = (right + left) * 0.5;
        let ty = (top + bottom) * 0.5;

        Translation3::new(tx, ty, 0.0).to_homogeneous()
            * Scale3::new(sx, sy, 1.0).to_homogeneous()
    }

    #[rustfmt::skip]
    fn calculate_model_matrix_relative(&self) -> Matrix4<f32> {
        let ImageScalingMode::Relative {
            width, height, left, right, top, bottom,
        } = self.scaling
        else {
            return Matrix4::zeros();
        };

        if right <= left || top <= bottom {
            return Matrix4::zeros();
        }

        let width = width as f32;
        let height = height as f32;

        let left   = (left   as f32 / width)  * 2.0 - 1.0;
        let right  = (right  as f32 / width)  * 2.0 - 1.0;
        let bottom = (bottom as f32 / height) * 2.0 - 1.0;
        let top    = (top    as f32 / height) * 2.0 - 1.0;

        let sx = (right - left) * 0.5;
        let sy = (top - bottom) * 0.5;

        // clip space
        let tx = (right + left) * 0.5;
        let ty = (top + bottom) * 0.5;

        Translation3::new(tx, ty, 0.0).to_homogeneous()
            * Scale3::new(sx, sy, 1.0).to_homogeneous()
    }

    #[rustfmt::skip]
    fn calculate_model_matrix_relative_stretch(&self) -> Matrix4<f32> {
        let ImageScalingMode::RelativeStretch { left, right, top, bottom } = self.scaling else {
            return Matrix4::zeros();
        };

        if right <= left || top <= bottom {
            return Matrix4::zeros();
        }

        let sx = right - left;
        let sy = top - bottom;

        let tx = left + right - 1.0;
        let ty = bottom + top - 1.0;

        Translation3::new(tx, ty, 0.0).to_homogeneous()
            * Scale3::new(sx, sy, 1.0).to_homogeneous()
    }

    #[rustfmt::skip]
    fn calculate_model_matrix(&self, window_width: f32, window_height: f32) -> Matrix4<f32> {
        match self.scaling {
            ImageScalingMode::Absolute {..} => self.calculate_model_matrix_absolute(window_width, window_height),
            ImageScalingMode::Relative {..} => self.calculate_model_matrix_relative(),
            ImageScalingMode::RelativeStretch {..} => self.calculate_model_matrix_relative_stretch(),
        }
    }
}
