use nalgebra::{Matrix4, Vector3, Vector4};
use std::ops::Mul;

#[derive(Debug, Copy, Clone)]
pub struct BoundingSphere {
    pub center: Vector3<f32>,
    pub radius: f32,
}

impl<F: Into<f32>> Mul<F> for BoundingSphere {
    type Output = BoundingSphere;

    fn mul(self, rhs: F) -> Self::Output {
        let rhs = rhs.into();
        BoundingSphere {
            center: self.center,
            radius: self.radius * rhs,
        }
    }
}

impl BoundingSphere {
    pub fn empty() -> Self {
        Self {
            center: Vector3::zeros(),
            radius: 0.0,
        }
    }

    pub fn transformed(&self, transform: &Matrix4<f32>) -> Self {
        let pos = transform * Vector4::new(self.center.x, self.center.y, self.center.z, 1.0);
        let w = if pos.w.abs() > f32::EPSILON {
            pos.w
        } else {
            1.0
        };

        let center = Vector3::new(pos.x / w, pos.y / w, pos.z / w);

        let sx = Vector3::new(transform[(0, 0)], transform[(1, 0)], transform[(2, 0)]).norm();
        let sy = Vector3::new(transform[(0, 1)], transform[(1, 1)], transform[(2, 1)]).norm();
        let sz = Vector3::new(transform[(0, 2)], transform[(1, 2)], transform[(2, 2)]).norm();
        let scale = sx.max(sy).max(sz);

        Self {
            center,
            radius: self.radius * scale,
        }
    }
}
