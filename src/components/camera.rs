use aligned::{Aligned, A16};
use bytemuck::{Pod, Zeroable};
use nalgebra::{Affine3, Matrix4, Perspective3, Vector3};

use crate::components::Component;
use crate::object::GameObjectId;
use crate::transform::Transform;

pub struct CameraComponent {
    pub projection: Perspective3<f32>,
    parent: GameObjectId,
}

impl CameraComponent {
    pub fn resize(&mut self, width: f32, height: f32) {
        self.projection = Perspective3::new(width / height, 60f32.to_radians(), 0.01, 1000.0);
    }
}

impl Component for CameraComponent {
    fn new(parent: GameObjectId) -> Self {
        CameraComponent {
            projection: Perspective3::new(800.0 / 600.0, 60f32.to_radians(), 0.01, 1000.0),
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
#[derive(Default, Debug, Copy, Clone)]
pub struct CameraData {
    pos: Aligned<A16, Vector3<f32>>,
    view_mat: Affine3<f32>,
    projection_mat: Matrix4<f32>,
    pub proj_view_mat: Matrix4<f32>,
}

impl CameraData {
    pub fn empty() -> Self {
        CameraData {
            pos: Aligned(Vector3::zeros()),
            view_mat: Affine3::identity(),
            projection_mat: Matrix4::identity(),
            proj_view_mat: Matrix4::identity(),
        }
    }

    pub fn update(&mut self, proj_matrix: &Perspective3<f32>, cam_transform: &Transform) {
        self.pos = Aligned(cam_transform.position());
        self.view_mat = cam_transform.get_global_transform_matrix_ext(true).inverse();
        self.projection_mat = proj_matrix.to_homogeneous();
        self.proj_view_mat = self.projection_mat * self.view_mat.to_homogeneous();
    }
}

unsafe impl Zeroable for CameraData {}
unsafe impl Pod for CameraData {}
