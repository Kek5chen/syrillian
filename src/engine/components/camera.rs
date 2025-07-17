use crate::components::Component;
use crate::core::{GameObjectId, Transform};
use crate::ensure_aligned;
use nalgebra::{Matrix4, Perspective3, Vector3};

pub struct CameraComponent {
    pub projection: Perspective3<f32>,
    fov: f32,
    near: f32,
    far: f32,
    width: f32,
    height: f32,
    parent: GameObjectId,
}

impl CameraComponent {
    /// Returns the fov in degrees
    pub fn fov(&self) -> f32 {
        self.fov
    }

    /// Sets the fov in degrees
    pub fn set_fov(&mut self, fov: f32) {
        self.fov = fov;
        self.regenerate();
    }

    pub fn near(&self) -> f32 {
        self.near
    }

    pub fn set_near(&mut self, near: f32) {
        self.near = near;
        self.regenerate();
    }

    pub fn far(&self) -> f32 {
        self.far
    }

    pub fn set_far(&mut self, far: f32) {
        self.far = far;
        self.regenerate();
    }

    pub fn regenerate(&mut self) {
        self.projection = Perspective3::new(
            self.width / self.height,
            self.fov.to_radians(),
            self.near,
            self.far,
        );
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
        self.regenerate();
    }
}

impl Component for CameraComponent {
    fn new(parent: GameObjectId) -> Self {
        CameraComponent {
            projection: Perspective3::new(800.0 / 600.0, 60f32.to_radians(), 0.01, 1000.0),
            fov: 60.0,
            near: 0.01,
            far: 1000.0,
            width: 800.0,
            height: 600.0,
            parent,
        }
    }

    fn init(&mut self) {
        self.get_parent().transform.set_compound_pos_first(true);
    }

    fn get_parent(&self) -> GameObjectId {
        self.parent
    }
}

#[repr(C)]
#[derive(Default, Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pos: Vector3<f32>,
    _padding: u32,
    view_mat: Matrix4<f32>,
    projection_mat: Matrix4<f32>,
    pub proj_view_mat: Matrix4<f32>,
}

ensure_aligned!(CameraUniform { pos, view_mat, projection_mat, proj_view_mat }, align <= 16 * 13 => size);

impl CameraUniform {
    pub fn empty() -> Self {
        CameraUniform {
            pos: Vector3::zeros(),
            _padding: 0,
            view_mat: Matrix4::identity(),
            projection_mat: Matrix4::identity(),
            proj_view_mat: Matrix4::identity(),
        }
    }

    pub fn update(&mut self, proj_matrix: &Perspective3<f32>, cam_transform: &Transform) {
        self.pos = cam_transform.position();
        self.view_mat = cam_transform
            .get_global_transform_matrix_ext(true)
            .inverse()
            .to_homogeneous();
        self.projection_mat = proj_matrix.to_homogeneous();
        self.proj_view_mat = self.projection_mat * self.view_mat;
    }
}
