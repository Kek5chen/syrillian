use crate::utils::math::QuaternionEuler;
use rapier3d::prelude::*;

use crate::components::Component;
use crate::core::GameObjectId;
use crate::World;

pub struct RigidBodyComponent {
    parent: GameObjectId,
    pub body_handle: RigidBodyHandle,
    kinematic: bool,
}

impl Component for RigidBodyComponent {
    fn new(parent: GameObjectId) -> Self {
        let initial_translation = parent.transform.position();
        let initial_rotation = parent.transform.rotation().euler_vector();
        let rigid_body = RigidBodyBuilder::dynamic()
            .user_data(parent.0 as u128)
            .translation(initial_translation)
            .rotation(initial_rotation)
            .build();

        let body_handle = World::instance().physics.rigid_body_set.insert(rigid_body);

        RigidBodyComponent {
            parent,
            body_handle,
            kinematic: false,
        }
    }

    fn late_update(&mut self) {
        let rb = World::instance()
            .physics
            .rigid_body_set
            .get_mut(self.body_handle);
        if let Some(rb) = rb {
            if rb.is_dynamic() {
                rb.set_translation(self.parent.transform.position(), false);
                rb.set_rotation(self.parent.transform.rotation(), false);
            } else if rb.is_kinematic() {
                rb.set_next_kinematic_translation(self.parent.transform.position());
                rb.set_next_kinematic_rotation(self.parent.transform.rotation());
            }
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
                self.parent().transform.set_position_vec(*rb.translation());
                self.parent().transform.set_rotation(*rb.rotation());
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
            false,
        );
    }

    fn parent(&self) -> GameObjectId {
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

    pub fn set_kinematic(&mut self, kinematic: bool) {
        let rb = self.get_body_mut().expect("Rigid body de-synced");
        if kinematic {
            rb.set_body_type(RigidBodyType::KinematicPositionBased, false);
        } else {
            rb.set_body_type(RigidBodyType::Dynamic, false);
        }
        self.kinematic = kinematic;
    }
}
