use nalgebra::{UnitQuaternion, Vector3};
use slotmap::Key;
use syrillian::engine::core::{GameObjectId, Transform};

#[test]
fn local_position_and_translation() {
    let mut t = Transform::new(GameObjectId::null());
    assert_eq!(*t.local_position(), Vector3::new(0.0, 0.0, 0.0));
    t.set_local_position_vec(Vector3::new(1.0, 2.0, 3.0));
    assert_eq!(*t.local_position(), Vector3::new(1.0, 2.0, 3.0));
    t.translate(Vector3::new(1.0, -1.0, 0.5));
    assert_eq!(*t.local_position(), Vector3::new(2.0, 1.0, 3.5));
}

#[test]
fn rotation_and_scale() {
    let mut t = Transform::new(GameObjectId::null());
    let rot = UnitQuaternion::from_euler_angles(0.0, 1.0, 0.0);
    t.set_local_rotation(rot);
    assert_eq!(t.local_rotation().coords, rot.coords);
    t.set_uniform_local_scale(2.0);
    assert_eq!(*t.local_scale(), Vector3::new(2.0, 2.0, 2.0));
}
