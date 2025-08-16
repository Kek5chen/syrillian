use crate::components::Component;
use crate::core::GameObjectId;
use crate::World;
use gilrs::{Axis, Button};
use nalgebra::{UnitQuaternion, Vector2, Vector3};
use winit::keyboard::KeyCode;

pub struct FreecamController {
    pub move_speed: f32,
    pub look_sensitivity: f32,
    parent: GameObjectId,
    pub yaw: f32,
    pub pitch: f32,
}

impl Component for FreecamController {
    fn new(parent: GameObjectId) -> Self
    where
        Self: Sized,
    {
        FreecamController {
            move_speed: 30.0f32,
            look_sensitivity: 0.1f32,
            parent,
            yaw: 0.0,
            pitch: 0.0,
        }
    }

    fn update(&mut self, world: &mut World) {
        let delta_time = world.delta_time().as_secs_f32();
        let transform = &mut self.parent().transform;

        let input = &world.input;

        let gamepad_delta = Vector2::new(-input.gamepad.axis(Axis::RightStickX), input.gamepad.axis(Axis::RightStickY)) * 100.;
        let delta = input.mouse_delta() + gamepad_delta;
        self.yaw += delta.x * self.look_sensitivity / 30.0;
        self.pitch += delta.y * self.look_sensitivity / 30.0;

        self.pitch = self.pitch.clamp(-89.0f32, 89.0f32);

        let yaw_rotation =
            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), self.yaw.to_radians());
        let pitch_rotation =
            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), self.pitch.to_radians());
        let rotation = yaw_rotation * pitch_rotation;

        transform.set_local_rotation(rotation);

        let mut fb_movement: f32 = 0.;
        if input.is_key_pressed(KeyCode::KeyW) {
            fb_movement += 1.;
        }

        if input.is_key_pressed(KeyCode::KeyS) {
            fb_movement -= 1.;
        }

        let mut lr_movement: f32 = 0.;
        if input.is_key_pressed(KeyCode::KeyA) {
            lr_movement -= 1.;
        }

        if input.is_key_pressed(KeyCode::KeyD) {
            lr_movement += 1.;
        }

        let mut ud_movement: f32 = 0.;
        if input.is_key_pressed(KeyCode::Space) {
            ud_movement = 1.0;
        }
        if input.is_key_pressed(KeyCode::ControlLeft) {
            ud_movement = -1.0;
        }

        let axis_x = input.gamepad.axis(Axis::LeftStickX);
        let axis_y = input.gamepad.axis(Axis::LeftStickY);
        let axis_z = input.gamepad.button(Button::RightTrigger2);
        if lr_movement.abs() < f32::EPSILON {
            lr_movement = axis_x;
        }
        if fb_movement.abs() < f32::EPSILON {
            fb_movement = axis_y;
        }
        if ud_movement.abs() < f32::EPSILON {
            let invert = input.gamepad.is_button_pressed(Button::East);
            if invert {
                ud_movement = -axis_z;
            } else {
                ud_movement = axis_z
            }
        }

        let mut direction = transform.right() * lr_movement
            + transform.up() * ud_movement
            + transform.forward() * fb_movement;

        let move_speed = if input.is_key_pressed(KeyCode::ShiftLeft) {
            self.move_speed * 3.0
        } else {
            let controller_extra_speed = input.gamepad.button(Button::LeftTrigger2) + (1. / 10.0) / 10.0;
            self.move_speed * (controller_extra_speed * 5.)
        };

        if direction.magnitude() > f32::EPSILON {
            direction.normalize_mut();
            transform.translate(direction * move_speed * delta_time);
        }
    }

    fn parent(&self) -> GameObjectId {
        self.parent
    }
}
