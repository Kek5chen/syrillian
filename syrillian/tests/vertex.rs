use nalgebra::{Vector2, Vector3};
use syrillian::engine::core::Vertex3D;

#[test]
fn vertex_creation_pads_indices() {
    let v = Vertex3D::new(
        Vector3::new(0.0, 0.0, 0.0),
        Vector2::new(0.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
        &[1, 2],
        &[0.5, 0.5],
    );
    assert_eq!(v.bone_indices, [1, 2, 0xFF, 0xFF]);
    assert_eq!(v.bone_weights, [0.5, 0.5, 0.0, 0.0]);
}
