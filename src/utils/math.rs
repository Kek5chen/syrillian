use nalgebra::{Matrix3, Matrix4, RealField, Rotation3, SimdRealField, UnitQuaternion, Vector3};
use num_traits::Float;

pub trait ExtraMatrixMath {
    fn decompose(self) -> (Vector3<f32>, UnitQuaternion<f32>, Vector3<f32>);
}

pub fn matrix_to_quaternion(matrix: Matrix3<f32>) -> UnitQuaternion<f32> {
    UnitQuaternion::from_rotation_matrix(&Rotation3::from_matrix(&matrix))
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
