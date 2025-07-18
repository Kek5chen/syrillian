use crate::components::Component;
use crate::core::{GameObjectId, Transform};
use crate::input::InputManager;
use crate::utils::FloatMathExt;
use crate::World;
use gilrs::Axis;
use nalgebra::{UnitQuaternion, Vector2, Vector3};

/// All tweakable parameters for the FPS Camera
#[derive(Debug, Clone)]
pub struct FPSCameraConfig {
    /// Mouse sensitivity coefficient. Default: X & Y = 0.6
    pub mouse_sensitivity: Vector2<f32>,
    /// Gamepad (right stick) sensitivity coefficient. Default: X & Y = 1.0
    pub controller_sensitivity: Vector2<f32>,
    /// Maximum up-down (pitch) angle. Default: 89.9
    pub max_pitch: f32,
    /// Maximum tilt (in degrees) when turning. Default: 3.0
    pub max_roll: f32,
    /// Bobbing amplitude on X and Y axes. Default: X = 0.05, Y = 0.05, Z = 0.0
    pub bob_amplitude: Vector3<f32>,
    /// Interpolation speed for bobbing and roll. Default: 10.0
    pub smoothing_speed: f32,
    /// Vertical bob on jump. Default: 0.5
    pub jump_bob_height: f32,
    /// How fast jump bob resets. Default: 5.0
    pub jump_bob_speed: f32,
}

#[derive(Debug)]
pub struct FirstPersonCameraController {
    parent: GameObjectId,
    pub config: FPSCameraConfig,

    yaw: f32,
    pitch: f32,
    smooth_roll: f32,
    bob_offset: Vector3<f32>,

    pub vel_y: f32,

    jump_offset: f32,
    jump_bob_interp: f32,
    jump_bob_interp_t: f32,
    is_jumping: bool,
    is_falling: bool,

    pub base_position: Vector3<f32>,
}

impl Default for FPSCameraConfig {
    fn default() -> Self {
        // Make sure to change the document comments if you change these
        FPSCameraConfig {
            mouse_sensitivity: Vector2::new(0.6, 0.6),
            controller_sensitivity: Vector2::new(1.0, 1.0),
            max_pitch: 89.9,
            max_roll: 3.0,
            bob_amplitude: Vector3::new(0.05, 0.05, 0.0),
            smoothing_speed: 10.0,
            jump_bob_height: 0.5,
            jump_bob_speed: 5.0,
        }
    }
}

impl Component for FirstPersonCameraController {
    fn new(parent: GameObjectId) -> Self
    where
        Self: Sized,
    {
        FirstPersonCameraController {
            parent,
            config: FPSCameraConfig::default(),
            yaw: 0.0,
            pitch: 0.0,
            smooth_roll: 0.0,
            bob_offset: Vector3::zeros(),

            vel_y: 0.0,

            jump_offset: 0.0,
            jump_bob_interp: 0.0,
            jump_bob_interp_t: 0.,
            is_jumping: false,
            is_falling: false,

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

        self.calculate_jump_bob(delta_time);
        self.update_jump_bob(transform);

        if !input.is_cursor_locked() {
            return;
        }

        let mouse_delta = input.mouse_delta();
        self.calculate_rotation(input, delta_time, mouse_delta);
        self.update_rotation(transform, delta_time, mouse_delta);
    }

    fn parent(&self) -> GameObjectId {
        self.parent
    }
}

impl FirstPersonCameraController {
    pub fn update_roll(&mut self, delta: f32, max: f32) {
        self.smooth_roll = (self.smooth_roll + delta / 70.0).clamp(-max, max);
    }

    pub fn update_bob(&mut self, amount: f32, speed_factor: f32) {
        let time = World::instance().time().as_secs_f32();
        let mul = (speed_factor / 2.).clamp(0.0, 2.0);
        let sin_tx = (time * 5. * mul).sin();
        let sin_ty = (time * 10. * mul).sin();
        let target = Vector3::new(
            sin_tx * self.config.bob_amplitude.x * mul,
            sin_ty * self.config.bob_amplitude.y * mul,
            0.0,
        );

        self.bob_offset = self.bob_offset.lerp(&target, 0.5 * amount);
    }

    pub fn signal_jump(&mut self) {
        self.is_jumping = true;
        self.is_falling = self.vel_y < f32::EPSILON;
        self.jump_offset = self.config.jump_bob_height;
    }

    fn update_rotation(
        &mut self,
        transform: &mut Transform,
        delta_time: f32,
        mouse_delta: &Vector2<f32>,
    ) {
        let yaw_rotation =
            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), self.yaw.to_radians());
        let pitch_rotation =
            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), self.pitch.to_radians());

        self.update_roll(mouse_delta.x, self.config.max_roll);
        self.smooth_roll = self
            .smooth_roll
            .lerp(0., self.config.smoothing_speed * delta_time);
        let roll_rotation =
            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), self.smooth_roll.to_radians());

        transform.set_local_rotation(pitch_rotation * roll_rotation);
        self.bob_offset = self
            .bob_offset
            .lerp(&Vector3::zeros(), self.config.smoothing_speed * delta_time);

        if let Some(mut parent) = self.parent().parent {
            parent.transform.set_local_rotation(yaw_rotation);
        }
    }

    fn calculate_rotation(
        &mut self,
        input: &InputManager,
        delta_time: f32,
        mouse_delta: &Vector2<f32>,
    ) {
        let controller_x = -input.gamepad.axis(Axis::RightStickX)
            * self.config.controller_sensitivity.x * 100.
            * delta_time;
        let controller_y = input.gamepad.axis(Axis::RightStickY)
            * self.config.controller_sensitivity.y * 100.
            * delta_time;
        let mouse_x = mouse_delta.x * self.config.mouse_sensitivity.x / 30.0;
        let mouse_y = mouse_delta.y * self.config.mouse_sensitivity.y / 30.0;
        let max_pitch = self.config.max_pitch;

        self.yaw += mouse_x + controller_x;
        self.pitch = (self.pitch + mouse_y + controller_y).clamp(-max_pitch, max_pitch);
    }

    fn calculate_jump_bob(&mut self, delta_time: f32) {
        if self.is_jumping {
            if !self.is_falling && self.vel_y <= 0. {
                self.is_falling = true;
                self.jump_offset = -self.config.jump_bob_height;
                self.jump_bob_interp_t = 0.;
            } else if self.is_falling && self.vel_y.abs() < f32::EPSILON * 10000. {
                self.is_jumping = false;
                self.is_falling = false;
                self.jump_offset = 0.;
            }
        }

        self.jump_bob_interp_t = self
            .jump_bob_interp_t
            .lerp(self.config.jump_bob_speed, delta_time * 5.);
        self.jump_bob_interp = self
            .jump_bob_interp
            .lerp(self.jump_offset, self.jump_bob_interp_t * delta_time);
        self.jump_offset.lerp(0., 0.1);
    }

    fn update_jump_bob(&mut self, transform: &mut Transform) {
        let right = transform.right();
        let up = Vector3::y();
        let bob_offset =
            (right * self.bob_offset.x) + up * (self.bob_offset.y + self.jump_bob_interp);

        transform.set_local_position_vec(self.base_position + bob_offset);
    }
}
