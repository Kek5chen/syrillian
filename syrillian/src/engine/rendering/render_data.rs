use crate::core::Transform;
use crate::ensure_aligned;
use crate::rendering::uniform::ShaderUniform;
use crate::utils::{MATRIX4_ID, VECTOR3_ID};
use nalgebra::{Matrix4, Vector2, Vector3};
use syrillian_macros::UniformIndex;
use wgpu::{BindGroupLayout, Device, Queue};

#[repr(C)]
#[derive(Default, Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub(crate) pos: Vector3<f32>,
    pub(crate) _padding: u32,
    pub(crate) view_mat: Matrix4<f32>,
    pub(crate) projection_mat: Matrix4<f32>,
    pub proj_view_mat: Matrix4<f32>,
}

ensure_aligned!(CameraUniform { pos, view_mat, projection_mat, proj_view_mat }, align <= 16 * 13 => size);

#[repr(C)]
#[derive(Default, Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SystemUniform {
    pub(crate) screen_size: Vector2<u32>,
    pub(crate) time: f32,
    pub(crate) delta_time: f32,
}

ensure_aligned!(SystemUniform { screen_size }, align <= 8 * 2 => size);

#[repr(u8)]
#[derive(Copy, Clone, Debug, UniformIndex)]
pub enum RenderUniformIndex {
    Camera = 0,
    System = 1,
}

pub struct RenderUniformData {
    pub camera_data: CameraUniform,
    pub system_data: SystemUniform,
    pub uniform: ShaderUniform<RenderUniformIndex>,
}

impl CameraUniform {
    pub const fn empty() -> Self {
        CameraUniform {
            pos: VECTOR3_ID,
            _padding: 0,
            view_mat: MATRIX4_ID,
            projection_mat: MATRIX4_ID,
            proj_view_mat: MATRIX4_ID,
        }
    }

    pub fn update_with_transform(&mut self, proj_matrix: &Matrix4<f32>, cam_transform: &Transform) {
        let pos = cam_transform.position();
        let view_mat = cam_transform
            .get_global_transform_matrix_ext(true)
            .inverse();

        self.update(proj_matrix, &pos, &view_mat.matrix());
    }

    pub fn update(
        &mut self,
        proj_matrix: &Matrix4<f32>,
        pos: &Vector3<f32>,
        view_matrix: &Matrix4<f32>,
    ) {
        self.pos = *pos;
        self.view_mat = *view_matrix;
        self.projection_mat = *proj_matrix;
        self.proj_view_mat = self.projection_mat * self.view_mat;
    }
}

impl SystemUniform {
    pub const fn empty() -> Self {
        SystemUniform {
            screen_size: Vector2::new(0, 0),
            time: 0.0,
            delta_time: 0.0,
        }
    }
}

impl RenderUniformData {
    pub fn empty(device: &Device, render_bgl: &BindGroupLayout) -> Self {
        let camera_data = CameraUniform::empty();
        let system_data = SystemUniform::empty();
        let uniform = ShaderUniform::<RenderUniformIndex>::builder(render_bgl)
            .with_buffer_data(&camera_data)
            .with_buffer_data(&system_data)
            .build(device);

        RenderUniformData {
            camera_data,
            system_data,
            uniform,
        }
    }

    pub fn upload_camera_data(&self, queue: &Queue) {
        queue.write_buffer(
            &self.uniform.buffer(RenderUniformIndex::Camera),
            0,
            bytemuck::bytes_of(&self.camera_data),
        );
    }

    pub fn upload_system_data(&self, queue: &Queue) {
        queue.write_buffer(
            &self.uniform.buffer(RenderUniformIndex::System),
            0,
            bytemuck::bytes_of(&self.system_data),
        );
    }
}
