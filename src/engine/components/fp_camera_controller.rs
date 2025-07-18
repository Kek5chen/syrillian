use crate::World;
use crate::components::Component;
use crate::core::GameObjectId;
use crate::utils::FloatMathExt;
use gilrs::Axis;
use nalgebra::{UnitQuaternion, Vector3};

pub struct FPCameraController {
    parent: GameObjectId,
    pub mouse_look_sensitivity: f32,
    pub controller_look_sensitivity: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub smooth_roll: f32,
    pub bob_x: f32,
    pub bob_y: f32,
    pub vel_y: f32,
    jumping: bool,
    jump_falling: bool,
    jump_bob: f32,
    jump_bob_interp: f32,
    jump_bob_interp_t: f32,
    jump_bob_interp_t_max: f32,
    pub jump_bob_max: f32,

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
            bob_x: 0.0,
            bob_y: 0.0,
            vel_y: 0.0,
            jumping: false,
            jump_falling: false,
            jump_bob: 0.0,
            jump_bob_interp: 0.0,
            jump_bob_interp_t: 0.,
            jump_bob_interp_t_max: 4.,
            jump_bob_max: 0.5,
            base_position: Vector3::zeros(),
        }
    }

    fn init(&mut self) {
        self.base_position = self.parent().transform.local_position().clone();
    }

    fn update(&mut self) {
        let input = &World::instance().input;
        let transform = &mut self.parent().transform;

        let delta_time = World::instance().delta_time().as_secs_f32();

        let right = transform.right();
        let up = Vector3::y();
        let bob_x = right * self.bob_x;
        let bob_y = up * self.bob_y;
        if self.jumping {
            if !self.jump_falling && self.vel_y <= 0. {
                self.jump_falling = true;
                self.jump_bob = -self.jump_bob_max;
                self.jump_bob_interp_t = 0.;
            } else if self.jump_falling && self.vel_y.abs() < f32::EPSILON * 10000. {
                self.jumping = false;
                self.jump_falling = false;
                self.jump_bob = 0.;
            }
        }

        self.jump_bob_interp_t = self.jump_bob_interp_t.lerp(self.jump_bob_interp_t_max, delta_time * 5.);
        self.jump_bob_interp = self.jump_bob_interp.lerp(self.jump_bob, self.jump_bob_interp_t * delta_time);
        self.jump_bob.lerp(0., 0.1);
        let vel_y = up * self.jump_bob_interp;

        transform.set_local_position_vec(self.base_position + bob_x + bob_y + vel_y);

        if !input.is_cursor_locked() {
            return;
        }

        let mouse_delta = input.mouse_delta();
        let controller_x =
            input.gamepad.axis(Axis::RightStickX) * self.controller_look_sensitivity * delta_time;
        let controller_y =
            input.gamepad.axis(Axis::RightStickY) * self.controller_look_sensitivity * delta_time;
        self.yaw += mouse_delta.x * self.mouse_look_sensitivity / 30.0 + controller_x;
        self.pitch += mouse_delta.y * self.mouse_look_sensitivity / 30.0 + controller_y;

        self.pitch = self.pitch.clamp(-89.0f32, 89.0f32);

        let yaw_rotation =
            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), self.yaw.to_radians());
        let pitch_rotation =
            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), self.pitch.to_radians());

        self.add_roll(mouse_delta.x, 3.);
        self.smooth_roll = self.smooth_roll.lerp(0., 10. * delta_time);
        let roll_rotation =
            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), self.smooth_roll.to_radians());

        transform.set_local_rotation(pitch_rotation * roll_rotation);
        self.bob_y = self.bob_y.lerp(0., 10. * delta_time);
        self.bob_x = self.bob_x.lerp(0., 10. * delta_time);

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
        self.bob_y = self.bob_y.lerp(time.sin() * 0.05, 0.5 * amount);
        self.bob_x = self.bob_x.lerp((time / 2.).sin() * 0.05, 0.5 * amount);
    }

    pub fn signal_jump(&mut self) {
        self.jumping = true;
        self.jump_falling = self.vel_y < f32::EPSILON;
        self.jump_bob = self.jump_bob_max;
    }
}
