use nalgebra::Vector3;

use crate::World;
use crate::components::Component;
use crate::core::GameObjectId;

pub struct GravityComp {
    pub acceleration_per_sec: f32,
    pub velocity: f32,
    pub max_acceleration: f32,
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
        let delta_time = World::instance().delta_time().as_secs_f32();

        self.velocity = (self.velocity - self.acceleration_per_sec * delta_time)
            .clamp(-self.max_acceleration, self.max_acceleration);
        let transform = &mut self.parent().transform;
        transform.translate(Vector3::new(0.0, self.velocity, 0.0));
    }

    fn parent(&self) -> GameObjectId {
        self.parent
    }
}
