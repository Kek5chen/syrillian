use super::{BoneData, Drawable};
use crate::core::{GameObjectId, ModelUniform};
use crate::drawables::MeshUniformIndex;
use crate::engine::assets::HMaterial;
use crate::engine::rendering::cache::AssetCache;
use crate::engine::rendering::uniform::ShaderUniform;
use crate::engine::rendering::{DrawCtx, Renderer};
use crate::World;
use log::error;
use nalgebra::{Matrix4, Scale3, Translation3};
use wgpu::Device;
use winit::window::Window;

#[derive(Debug, Clone, Copy)]
pub enum ImageScalingMode {
    Absolute {
        left: u32,
        right: u32,
        top: u32,
        bottom: u32,
    },
    Relative {
        width: u32,
        height: u32,
        left: u32,
        right: u32,
        top: u32,
        bottom: u32,
    },
    RelativeStretch {
        left: f32,
        right: f32,
        top: f32,
        bottom: f32,
    },
}

#[derive(Debug)]
struct ImageGPUData {
    translation_data: ModelUniform,
    uniform: ShaderUniform<MeshUniformIndex>,
}

#[derive(Debug)]
pub struct Image {
    material: HMaterial,
    scaling: ImageScalingMode,
    gpu_data: Option<ImageGPUData>,
}

impl Image {
    pub fn new(material: HMaterial) -> Box<Image> {
        Box::new(Image {
            material,
            scaling: ImageScalingMode::Absolute {
                left: 0,
                right: 100,
                top: 0,
                bottom: 100,
            },
            gpu_data: None,
        })
    }

    pub fn new_with_size(material: HMaterial, scaling: ImageScalingMode) -> Box<Image> {
        Box::new(Image {
            material,
            scaling,
            gpu_data: None,
        })
    }

    pub fn scaling_mode(&self) -> ImageScalingMode {
        self.scaling
    }

    pub fn set_scaling_mode(&mut self, scaling: ImageScalingMode) {
        self.scaling = scaling;
    }
}

impl Drawable for Image {
    fn setup(&mut self, renderer: &Renderer, _world: &mut World, _parent: GameObjectId) {
        self.setup_model_data(&renderer.cache, &renderer.state.device);
    }

    fn update(
        &mut self,
        _world: &mut World,
        _parent: GameObjectId,
        renderer: &Renderer,
        _outer_transform: &Matrix4<f32>,
    ) {
        self.update_model_matrix(&renderer.state.queue, &renderer.window);
    }

    fn draw(&self, _world: &mut World, ctx: &DrawCtx) {
        let unit_square_runtime = ctx.frame.cache.mesh_unit_square();
        let material = ctx.frame.cache.material(self.material);
        let shader = ctx.frame.cache.shader_2d();

        let Some(gpu_data) = &self.gpu_data else {
            error!("Image GPU Data wasn't set up.");
            return;
        };

        let mut pass = ctx.pass.write().unwrap();

        pass.set_pipeline(&shader.pipeline);

        let vertex_buf_slice = unit_square_runtime.vertices_buf.slice(..);
        let material_bind_group = material.uniform.bind_group();
        let vertices_count = unit_square_runtime.vertices_num as u32;

        pass.set_vertex_buffer(0, vertex_buf_slice);
        pass.set_bind_group(1, gpu_data.uniform.bind_group(), &[]);
        pass.set_bind_group(2, material_bind_group, &[]);
        pass.draw(0..vertices_count, 0..1)
    }
}

impl Image {
    fn setup_model_data(&mut self, cache: &AssetCache, device: &Device) {
        let bgl = cache.bgl_model();

        let translation_data = ModelUniform::empty();

        let uniform = ShaderUniform::<MeshUniformIndex>::builder(&bgl)
            .with_buffer_data(&translation_data)
            .with_buffer_data_slice(&BoneData::DUMMY_BONE)
            .build(device);

        self.gpu_data = Some(ImageGPUData {
            translation_data,
            uniform,
        });
    }

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

    fn update_model_matrix(&mut self, queue: &wgpu::Queue, window: &Window) {
        let window_size = window.inner_size();
        let width = window_size.width as f32;
        let height = window_size.height as f32;

        let new_model_mat = self.calculate_model_matrix(width, height);

        let Some(gpu_data) = &mut self.gpu_data else {
            error!("GPU data not set");
            return;
        };
        gpu_data.translation_data.model_mat = new_model_mat;

        queue.write_buffer(
            gpu_data.uniform.buffer(MeshUniformIndex::MeshData),
            0,
            bytemuck::bytes_of(&gpu_data.translation_data),
        );
    }
}
