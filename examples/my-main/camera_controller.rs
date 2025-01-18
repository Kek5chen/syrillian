use nalgebra::{UnitQuaternion, Vector3};
use winit::window::CursorGrabMode;
use syrillian::components::Component;
use syrillian::object::GameObjectId;
use syrillian::world::World;

pub struct CameraController {
	parent: GameObjectId,
	look_sensitivity: f32,
	yaw: f32,
	pitch: f32,
}

impl Component for CameraController {
	unsafe fn new(parent: GameObjectId) -> Self
	where
		Self: Sized,
	{
		CameraController {
			parent,
			look_sensitivity: 0.3f32,
			yaw: 0.0,
			pitch: 0.0,
		}
	}

	unsafe fn update(&mut self) {
		let input = &World::instance().input;

		if input.get_mouse_mode() == CursorGrabMode::None {
			return;
		}

		let transform = &mut self.get_parent().transform;

		let mouse_delta = input.get_mouse_delta(); 
		self.yaw += mouse_delta.x * self.look_sensitivity / 30.0;
		self.pitch += mouse_delta.y * self.look_sensitivity / 30.0;

		self.pitch = self.pitch.clamp(-89.0f32, 89.0f32);

		let yaw_rotation = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), self.yaw.to_radians());
		let pitch_rotation = UnitQuaternion::from_axis_angle(&Vector3::x_axis(), self.pitch.to_radians());

		transform.set_local_rotation(pitch_rotation);
		if let Some(mut parent) = self.get_parent().parent {
			parent.transform.set_local_rotation(yaw_rotation);
		}
	}

	unsafe fn get_parent(&self) -> GameObjectId {
		self.parent
	}
}
