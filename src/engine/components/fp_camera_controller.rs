use gilrs::Axis;
use crate::World;
use crate::components::Component;
use crate::core::GameObjectId;
use nalgebra::{UnitQuaternion, Vector3};
use crate::utils::FloatMathExt;

pub struct FPCameraController {
    parent: GameObjectId,
    pub mouse_look_sensitivity: f32,
    pub controller_look_sensitivity: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub smooth_roll: f32,
    pub bob: f32,
    pub base_position: Vector3<f32>,
}

impl Component for FPCameraController {
    fn new(parent: GameObjectId) -> Self
    where
        Self: Sized,
    {
        FPCameraController {
            parent,
            mouse_look_sensitivity: 0.6,
            controller_look_sensitivity: 1.0,
            yaw: 0.0,
            pitch: 0.0,
            smooth_roll: 0.0,
            bob: 0.0,
            base_position: Vector3::zeros(),
        }
    }

    fn init(&mut self) {
        self.base_position = self.parent().transform.local_position().clone();
    }

    fn update(&mut self) {
        let input = &World::instance().input;

        if !input.is_cursor_locked() {
            return;
        }

        let delta_time = World::instance().delta_time().as_secs_f32();

        let transform = &mut self.parent().transform;

        let mouse_delta = input.mouse_delta();
        let controller_x = input.gamepad.axis(Axis::RightStickX) * self.controller_look_sensitivity * delta_time;
        let controller_y = input.gamepad.axis(Axis::RightStickY) * self.controller_look_sensitivity * delta_time;
        self.yaw += mouse_delta.x * self.mouse_look_sensitivity / 30.0 + controller_x;
        self.pitch += mouse_delta.y * self.mouse_look_sensitivity / 30.0 + controller_y;

        self.pitch = self.pitch.clamp(-89.0f32, 89.0f32);

        let yaw_rotation =
            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), self.yaw.to_radians());
        let pitch_rotation =
            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), self.pitch.to_radians());

        self.add_roll(mouse_delta.x, 3.);
        self.smooth_roll = self.smooth_roll.lerp(0., 10. * delta_time);
        let roll_rotation = UnitQuaternion::from_axis_angle(&Vector3::z_axis(), self.smooth_roll.to_radians());

        transform.set_local_rotation(pitch_rotation * roll_rotation);
        transform.set_local_position_vec(self.base_position + Vector3::y() * self.bob);
        self.bob = self.bob.lerp(0., 10. * delta_time);
        
        if let Some(mut parent) = self.parent().parent {
            parent.transform.set_local_rotation(yaw_rotation);
        }
    }

    fn parent(&self) -> GameObjectId {
        self.parent
    }
}

impl FPCameraController {
    pub fn add_roll(&mut self, delta: f32, max: f32) {
        self.smooth_roll = (self.smooth_roll + delta / 70.0).clamp(-max, max);
    }

    pub fn do_bob(&mut self, amount: f32) {
        let time = World::instance().time().as_secs_f32() * 10.;
        self.bob = self.bob.lerp(time.sin() * 0.05, 0.5 * amount);
    }
}