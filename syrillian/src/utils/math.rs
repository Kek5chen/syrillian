use nalgebra::{Matrix3, Matrix4, RealField, Rotation3, SimdRealField, UnitQuaternion, Vector3};
use num_traits::Float;

pub const VECTOR3_ID: Vector3<f32> = Vector3::new(0.0, 0.0, 0.0);
#[rustfmt::skip]
pub const MATRIX4_ID: Matrix4<f32> = Matrix4::new(
    1.0, 0.0, 0.0, 0.0, 
    0.0, 1.0, 0.0, 0.0, 
    0.0, 0.0, 1.0, 0.0, 
    0.0, 0.0, 0.0, 1.0,
);

pub trait ExtraMatrixMath {
    fn decompose(self) -> (Vector3<f32>, UnitQuaternion<f32>, Vector3<f32>);
}

pub fn matrix_to_quaternion(matrix: Matrix3<f32>) -> UnitQuaternion<f32> {
    UnitQuaternion::from_rotation_matrix(&Rotation3::from_matrix_eps(
        &matrix,
        f32::EPSILON,
        1000,
        Rotation3::identity(),
    ))
}

fn decompose_mat3(matrix: Matrix4<f32>) -> (Vector3<f32>, UnitQuaternion<f32>, Vector3<f32>) {
    let translation = matrix.column(3).xyz();

    let scale_x = matrix.column(0).xyz().norm();
    let scale_y = matrix.column(1).xyz().norm();
    let scale_z = matrix.column(2).xyz().norm();
    let scale = Vector3::new(scale_x, scale_y, scale_z);

    let rotation_matrix = Matrix3::from_columns(&[
        matrix.column(0).xyz() / scale_x,
        matrix.column(1).xyz() / scale_y,
        matrix.column(2).xyz() / scale_z,
    ]);

    let rotation = matrix_to_quaternion(rotation_matrix);

    (translation, rotation, scale)
}

impl ExtraMatrixMath for Matrix4<f32> {
    fn decompose(self) -> (Vector3<f32>, UnitQuaternion<f32>, Vector3<f32>) {
        decompose_mat3(self)
    }
}

pub trait QuaternionEuler<T> {
    fn euler_vector_deg(&self) -> Vector3<T>;
    fn euler_vector(&self) -> Vector3<T>;
    fn from_euler_angles_deg(roll: T, pitch: T, yaw: T) -> UnitQuaternion<T>;
}

impl<T: SimdRealField + RealField + Float> QuaternionEuler<T> for UnitQuaternion<T>
where
    T::Element: SimdRealField,
{
    fn euler_vector_deg(&self) -> Vector3<T> {
        let angles = self.euler_angles();
        Vector3::new(
            angles.0.to_degrees(),
            angles.1.to_degrees(),
            angles.2.to_degrees(),
        )
    }

    fn euler_vector(&self) -> Vector3<T> {
        let angles = self.euler_angles();
        Vector3::new(angles.0, angles.1, angles.2)
    }

    fn from_euler_angles_deg(roll: T, pitch: T, yaw: T) -> UnitQuaternion<T> {
        UnitQuaternion::from_euler_angles(roll.to_radians(), pitch.to_radians(), yaw.to_radians())
    }
}

pub trait FloatMathExt {
    fn lerp(self, other: Self, t: f32) -> Self;
}

impl FloatMathExt for f32 {
    fn lerp(self, other: Self, t: f32) -> Self {
        self * (1.0 - t) + other * t
    }
}
