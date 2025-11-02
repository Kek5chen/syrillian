use nalgebra::{Matrix4, UnitQuaternion, Vector3};
use syrillian::utils::math::{ExtraMatrixMath, FloatMathExt, QuaternionEuler, light_range};

#[test]
fn matrix_decompose_recovers_transform_components() {
    let translation = Vector3::new(3.0, -2.0, 5.0);
    let rotation = UnitQuaternion::from_euler_angles(0.3, -0.5, 1.2);
    let scale = Vector3::new(2.0, 1.5, 0.75);

    let transform = Matrix4::new_translation(&translation)
        * rotation.to_homogeneous()
        * Matrix4::new_nonuniform_scaling(&scale);

    let (t, r, s) = transform.decompose();

    assert!((t - translation).norm() < 1e-5);
    assert!(r.angle_to(&rotation) < 1e-5);
    assert!((s - scale).norm() < 1e-5);
}

#[test]
fn quaternion_euler_round_trip_degrees() {
    let q = UnitQuaternion::from_euler_angles(0.1, -0.2, 0.3);
    let deg = q.euler_vector_deg();

    let rebuilt = UnitQuaternion::from_euler_angles_deg(deg.x, deg.y, deg.z);
    assert!(q.angle_to(&rebuilt) < 1e-5);
}

#[test]
fn float_math_lerp_interpolates_between_values() {
    let start = 10.0_f32;
    let end = 20.0_f32;

    assert!((start.lerp(end, 0.0) - start).abs() < 1e-6);
    assert!((start.lerp(end, 0.5) - 15.0).abs() < 1e-6);
    assert!((start.lerp(end, 1.0) - end).abs() < 1e-6);
}

#[test]
fn light_range_handles_constant_and_quadratic_terms() {
    // constant-only attenuation within threshold
    assert_eq!(light_range(50.0, 10.0, 0.0, 0.0, 100.0), Some(0.0));

    // quadratic attenuation that should return a positive finite distance
    let range = light_range(100.0, 1.0, 0.7, 0.2, 1.0).expect("range should exist");
    assert!(range >= 0.0);
}
