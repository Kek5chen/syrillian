use nalgebra::Vector3;

use crate::components::Component;
use crate::object::GameObjectId;
use crate::world::World;

pub struct GravityComp {
    pub acceleration_per_sec: f32,
    velocity: f32,
    max_acceleration: f32,
    parent: GameObjectId,
}

impl Component for GravityComp {
    fn new(parent: GameObjectId) -> Self {
        GravityComp {
            acceleration_per_sec: 9.80665,
            velocity: 0.0,
            max_acceleration: 100.0,
            parent,
        }
    }

    fn update(&mut self) {
        let delta_time = World::instance().get_delta_time().as_secs_f32();

        self.velocity = (self.velocity - self.acceleration_per_sec * delta_time)
            .clamp(-self.max_acceleration, self.max_acceleration);
        let transform = &mut self.get_parent().transform;
        transform.translate(Vector3::new(0.0, self.velocity, 0.0));
    }

    fn get_parent(&self) -> GameObjectId {
        self.parent
    }
}
