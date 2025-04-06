use std::cell::RefCell;
use std::rc::Rc;
use log::warn;
use nalgebra::Vector3;
use num_traits::Zero;
use rapier3d::prelude::{vector, LockedAxes};
use winit::keyboard::KeyCode;
use winit::window::CursorGrabMode;
use syrillian::components::{Component, RigidBodyComponent};
use syrillian::object::GameObjectId;
use syrillian::world::World;

pub struct PlayerMovement {
	parent: GameObjectId,
	move_speed: f32,
	jump_factor: f32,
	damping_factor: f32,
	rigid_body: Option<Rc<RefCell<Box<RigidBodyComponent>>>>,
	velocity: Vector3<f32>,
	sprint_multiplier: f32,
}

impl Component for PlayerMovement {
	unsafe fn new(parent: GameObjectId) -> Self
	where
		Self: Sized
	{
		PlayerMovement {
			parent,
			move_speed: 2.0,
			damping_factor: 1.5,
			jump_factor: 10.0,
			rigid_body: None,
			velocity: Vector3::zero(),
			sprint_multiplier: 10.0,
		}
	}

	unsafe fn init(&mut self) {
    let rigid = unsafe {
        self.get_parent().get_component::<RigidBodyComponent>()
    };
		if let Some(rigid) = rigid.clone() {
			if let Some(rigid) = rigid
				.borrow_mut()
				.get_body_mut()	{
				rigid.set_locked_axes(LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Y | LockedAxes::ROTATION_LOCKED_Z, false);
				rigid.enable_ccd(true);
			}
		}
		self.rigid_body = rigid;
	}

	unsafe fn update(&mut self) {
		let mut rigid = match &self.rigid_body {
			None => {
				warn!("Rigid body not set!");
				return;
			}
			Some(rigid) => rigid.borrow_mut()
		};

		let body = match rigid.get_body_mut() {
			None => {
				warn!("Rigid body not in set");
				return;
			}
			Some(rigid) => rigid,
		};

		let world= World::instance();

		if world.input.get_mouse_mode() == CursorGrabMode::None {
			return;
		}

		self.velocity /= self.damping_factor;

		if world.input.is_key_down(KeyCode::Space) {
			body.apply_impulse(vector![0.0, 0.2 * self.jump_factor, 0.0], true);
		}

		let mut factor = self.move_speed;

		if world.input.is_key_pressed(KeyCode::ShiftLeft) {
			factor *= self.sprint_multiplier;
		}
		
		let mut base_vel = Vector3::zero();

		if world.input.is_key_pressed(KeyCode::KeyW) {
			base_vel += self.parent.transform.forward();
		}

		if world.input.is_key_pressed(KeyCode::KeyS) {
			base_vel -= self.parent.transform.forward();
		}

		if world.input.is_key_pressed(KeyCode::KeyA) {
			base_vel -= self.parent.transform.right();
		}

		if world.input.is_key_pressed(KeyCode::KeyD) {
			base_vel += self.parent.transform.right();
		}
		
		if base_vel.magnitude() > 0.5 {
			base_vel = base_vel.normalize();
		}
		self.velocity += base_vel * factor;
		
		let mut linvel = *body.linvel();
		linvel.x = self.velocity.x;
		linvel.z = self.velocity.z;
		
		body.set_linvel(linvel, true);
	}

	unsafe fn get_parent(&self) -> GameObjectId {
		self.parent
	}
}
