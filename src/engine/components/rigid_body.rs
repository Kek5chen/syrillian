use crate::utils::math::QuaternionEuler;
use rapier3d::prelude::*;

use crate::World;
use crate::components::Component;
use crate::core::GameObjectId;

pub struct RigidBodyComponent {
    parent: GameObjectId,
    pub body_handle: RigidBodyHandle,
}

impl Component for RigidBodyComponent {
    fn new(parent: GameObjectId) -> Self {
        let initial_translation = parent.transform.position();
        let initial_rotation = parent.transform.rotation().euler_vector();
        let rigid_body = RigidBodyBuilder::dynamic()
            .translation(initial_translation)
            .rotation(initial_rotation)
            .build();

        let body_handle = World::instance().physics.rigid_body_set.insert(rigid_body);

        RigidBodyComponent {
            parent,
            body_handle,
        }
    }

    fn late_update(&mut self) {
        let rb = World::instance()
            .physics
            .rigid_body_set
            .get_mut(self.body_handle);
        if let Some(rb) = rb {
            rb.set_translation(self.parent.transform.position(), false);
            rb.set_rotation(self.parent.transform.rotation(), false);
        } else {
            todo!("de-synced - remake_rigid_body();")
        }
    }

    fn post_update(&mut self) {
        let rb = World::instance()
            .physics
            .rigid_body_set
            .get_mut(self.body_handle);
        if let Some(rb) = rb {
            if rb.is_dynamic() {
                self.get_parent()
                    .transform
                    .set_position_vec(*rb.translation());
                self.get_parent().transform.set_rotation(*rb.rotation());
            }
        }
    }

    fn delete(&mut self) {
        let world = World::instance();

        world.physics.rigid_body_set.remove(
            self.body_handle,
            &mut world.physics.island_manager,
            &mut world.physics.collider_set,
            &mut world.physics.impulse_joint_set,
            &mut world.physics.multibody_joint_set,
            true,
        );
    }

    fn get_parent(&self) -> GameObjectId {
        self.parent
    }
}

impl RigidBodyComponent {
    pub fn get_body(&self) -> Option<&RigidBody> {
        World::instance()
            .physics
            .rigid_body_set
            .get(self.body_handle)
    }

    pub fn get_body_mut(&mut self) -> Option<&mut RigidBody> {
        World::instance()
            .physics
            .rigid_body_set
            .get_mut(self.body_handle)
    }
}
