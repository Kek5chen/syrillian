use crate::World;
use crate::components::rope::RopeComponentError::*;
use crate::components::{Component, RigidBodyComponent};
use crate::core::GameObjectId;
use log::warn;
use rapier3d::prelude::*;
use snafu::{Snafu, ensure};

#[derive(Debug, Snafu)]
#[snafu(context(suffix(Err)))]
pub enum RopeComponentError {
    #[snafu(display("SpringComponent: Connector doesn't exist"))]
    InvalidConnector,
    #[snafu(display("SpringComponent: Parent doesn't have a rigid body"))]
    NoParentRigidBody,
    #[snafu(display("SpringComponent: Connector doesn't have a rigid body"))]
    NoConnectorRigidBody,
}

pub struct RopeComponent {
    parent: GameObjectId,
    connected: Option<GameObjectId>,
    handle: Option<ImpulseJointHandle>,
    length: f32,
}

impl Component for RopeComponent {
    fn new(parent: GameObjectId) -> Self
    where
        Self: Sized,
    {
        RopeComponent {
            parent,
            connected: None,
            handle: None,
            length: 10.0,
        }
    }

    fn update(&mut self) {}

    fn delete(&mut self) {
        if let Some(joint) = self.handle {
            World::instance()
                .physics
                .impulse_joint_set
                .remove(joint, false);
            self.handle = None;
            self.connected = None;
        }
    }

    fn parent(&self) -> GameObjectId {
        self.parent
    }
}

impl RopeComponent {
    pub fn connect_to(&mut self, body: GameObjectId) {
        if let Err(e) = self.try_connect_to(body) {
            warn!("{e}");
        }
    }

    pub fn try_connect_to(&mut self, body: GameObjectId) -> Result<(), RopeComponentError> {
        ensure!(body.exists(), InvalidConnectorErr);

        let self_rb = self
            .parent
            .get_component::<RigidBodyComponent>()
            .ok_or(NoParentRigidBody)?
            .borrow()
            .body_handle;
        let other_rb = body
            .get_component::<RigidBodyComponent>()
            .ok_or(NoConnectorRigidBody)?
            .borrow()
            .body_handle;

        let joint = RopeJoint::new(self.length);
        let handle = World::instance()
            .physics
            .impulse_joint_set
            .insert(self_rb, other_rb, joint, true);

        self.connected = Some(body);
        self.handle = Some(handle);

        Ok(())
    }

    pub fn disconnect(&mut self) {
        self.delete();
    }

    pub fn rope(&self) -> Option<&RopeJoint> {
        World::instance()
            .physics
            .impulse_joint_set
            .get(self.handle?)?
            .data
            .as_rope()
    }

    pub fn rope_mut(&self) -> Option<&mut RopeJoint> {
        World::instance()
            .physics
            .impulse_joint_set
            .get_mut(self.handle?, false)?
            .data
            .as_rope_mut()
    }

    pub fn set_length(&mut self, length: f32) {
        self.length = length;
        if let Some(rope) = self.rope_mut() {
            rope.set_max_distance(length);
        }
    }
}
