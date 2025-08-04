use crate::components::{Component, FirstPersonCameraController, RigidBodyComponent};
use crate::core::GameObjectId;
use crate::World;
use gilrs::Axis;
use log::warn;
use nalgebra::Vector3;
use num_traits::Zero;
use rapier3d::prelude::{vector, LockedAxes};
use std::cell::RefCell;
use std::rc::Rc;
use winit::keyboard::KeyCode;

pub struct FirstPersonMovementController {
    parent: GameObjectId,
    pub move_speed: f32,
    pub jump_factor: f32,
    rigid_body: Option<Rc<RefCell<Box<RigidBodyComponent>>>>,
    camera_controller: Option<Rc<RefCell<Box<FirstPersonCameraController>>>>,
    pub velocity: Vector3<f32>,
    pub sprint_multiplier: f32,
    velocity_interp_t: f32,
}

impl Component for FirstPersonMovementController {
    fn new(parent: GameObjectId) -> Self
    where
        Self: Sized,
    {
        FirstPersonMovementController {
            parent,
            move_speed: 5.0,
            jump_factor: 100.0,
            rigid_body: None,
            camera_controller: None,
            velocity: Vector3::zero(),
            sprint_multiplier: 2.0,
            velocity_interp_t: 0.06,
        }
    }

    fn init(&mut self) {
        let rigid = self.parent().get_component::<RigidBodyComponent>();
        if let Some(rigid) = rigid.clone() {
            if let Some(rigid) = rigid.borrow_mut().get_body_mut() {
                rigid.set_locked_axes(LockedAxes::ROTATION_LOCKED, false);
                rigid.enable_ccd(true);
            }
        }
        self.rigid_body = rigid;

        self.camera_controller = self
            .parent
            .get_child_component::<FirstPersonCameraController>();
    }

    fn update(&mut self) {
        let mut rigid = match &self.rigid_body {
            None => {
                warn!("Rigid body not set!");
                return;
            }
            Some(rigid) => rigid.borrow_mut(),
        };

        let body = match rigid.get_body_mut() {
            None => {
                warn!("Rigid body not in set");
                return;
            }
            Some(rigid) => rigid,
        };

        let world = World::instance();

        if !world.input.is_cursor_locked() {
            return;
        }

        let jumping = world.input.is_jump_down();
        if jumping {
            body.apply_impulse(vector![0.0, 0.2 * self.jump_factor, 0.0], true);
        }

        let mut speed_factor = self.move_speed;

        if world.input.is_sprinting() {
            speed_factor *= self.sprint_multiplier;
        }

        let mut target_velocity = Vector3::zero();

        let mut fb_movement: f32 = 0.;
        if world.input.is_key_pressed(KeyCode::KeyW) {
            target_velocity += self.parent.transform.forward();
            fb_movement += 1.;
        }

        if world.input.is_key_pressed(KeyCode::KeyS) {
            target_velocity -= self.parent.transform.forward();
            fb_movement -= 1.;
        }

        let mut lr_movement: f32 = 0.;
        if world.input.is_key_pressed(KeyCode::KeyA) {
            target_velocity -= self.parent.transform.right();
            lr_movement -= 1.;
        }

        if world.input.is_key_pressed(KeyCode::KeyD) {
            target_velocity += self.parent.transform.right();
            lr_movement += 1.;
        }

        let axis_x = world.input.gamepad.axis(Axis::LeftStickX);
        let axis_y = world.input.gamepad.axis(Axis::LeftStickY);
        if fb_movement.abs() < f32::EPSILON {
            target_velocity += self.parent.transform.forward() * axis_y;
            fb_movement = axis_y;
        }
        if lr_movement.abs() < f32::EPSILON {
            target_velocity += self.parent.transform.right() * axis_x;
            lr_movement = axis_x;
        }

        if target_velocity.magnitude() > 0.5 {
            target_velocity = target_velocity.normalize();
        }
        target_velocity *= speed_factor;
        self.velocity = self.velocity.lerp(&target_velocity, self.velocity_interp_t);

        if let Some(camera) = self.camera_controller.as_ref() {
            let mut camera = camera.borrow_mut();
            let world = World::instance();
            let delta_time = world.delta_time().as_secs_f32();
            camera.update_roll(
                -lr_movement * speed_factor * delta_time * 100.,
                4. - fb_movement.abs() * 2.,
            );
            camera.update_bob(self.velocity.magnitude());
            camera.vel = *body.linvel();
            if jumping {
                camera.signal_jump();
            }
        }

        let mut linvel = *body.linvel();
        linvel.x = self.velocity.x;
        linvel.z = self.velocity.z;

        body.set_linvel(linvel, true);
    }

    fn parent(&self) -> GameObjectId {
        self.parent
    }
}
