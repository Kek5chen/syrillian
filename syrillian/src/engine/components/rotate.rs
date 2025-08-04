use nalgebra::{UnitQuaternion, Vector3};

use crate::components::Component;
use crate::core::GameObjectId;
use crate::World;

pub struct RotateComponent {
    pub rotate_speed: f32,
    pub iteration: f32,
    parent: GameObjectId,
    pub y_rot: f32,
    pub scale_coefficient: f32,
    default_scale: Vector3<f32>,
}

impl Component for RotateComponent {
    fn new(parent: GameObjectId) -> Self
    where
        Self: Sized,
    {
        RotateComponent {
            rotate_speed: 50.0f32,
            iteration: 0.0,
            parent,
            y_rot: 0.0,
            scale_coefficient: 0.0,
            default_scale: parent.transform.scale(),
        }
    }

    fn update(&mut self) {
        let transform = &mut self.parent().transform;
        let delta_time = World::instance().delta_time().as_secs_f32();

        let x_angle_radians = (self.iteration / 100.0).sin() * 45.0f32.to_radians();
        let x_rotation = UnitQuaternion::from_axis_angle(&Vector3::x_axis(), x_angle_radians);

        self.y_rot += self.rotate_speed.to_radians() * delta_time;
        let y_rotation = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), self.y_rot);

        let combined_rotation = y_rotation * x_rotation;

        transform.set_rotation(combined_rotation);
        if self.scale_coefficient > f32::EPSILON {
            transform.set_nonuniform_local_scale(
                self.default_scale * (self.iteration.sin() * self.scale_coefficient + 1.),
            );
        }
        self.iteration += delta_time;
    }

    fn parent(&self) -> GameObjectId {
        self.parent
    }
}
