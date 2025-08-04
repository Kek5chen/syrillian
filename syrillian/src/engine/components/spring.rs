use self::SpringComponentError::*;
use crate::components::{Component, RigidBodyComponent};
use crate::core::GameObjectId;
use crate::World;
use log::warn;
use rapier3d::prelude::*;
use snafu::{ensure, Snafu};

#[derive(Debug, Snafu)]
#[snafu(context(suffix(Err)))]
pub enum SpringComponentError {
    #[snafu(display("SpringComponent: Connector doesn't exist"))]
    InvalidConnector,
    #[snafu(display("SpringComponent: Parent doesn't have a rigid body"))]
    NoParentRigidBody,
    #[snafu(display("SpringComponent: Connector doesn't have a rigid body"))]
    NoConnectorRigidBody,
}

pub struct SpringComponent {
    parent: GameObjectId,
    connected: Option<GameObjectId>,
    handle: Option<ImpulseJointHandle>,
    rest_length: f32,
    stiffness: f32,
    damping: f32,
}

impl Component for SpringComponent {
    fn new(parent: GameObjectId) -> Self
    where
        Self: Sized,
    {
        SpringComponent {
            parent,
            connected: None,
            handle: None,
            rest_length: 10.0,
            stiffness: 10.0,
            damping: 1.0,
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

impl SpringComponent {
    pub fn connect_to(&mut self, body: GameObjectId) {
        if let Err(e) = self.try_connect_to(body) {
            warn!("{e}");
        }
    }

    pub fn try_connect_to(&mut self, body: GameObjectId) -> Result<(), SpringComponentError> {
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

        let joint = SpringJoint::new(self.rest_length, self.stiffness, self.damping);
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

    pub fn spring(&self) -> Option<&SpringJoint> {
        let spring = &World::instance()
            .physics
            .impulse_joint_set
            .get(self.handle?)?
            .data;
        unsafe { std::mem::transmute(spring) }
    }

    pub fn spring_mut(&self) -> Option<&mut SpringJoint> {
        let spring = &mut World::instance()
            .physics
            .impulse_joint_set
            .get_mut(self.handle?, false)?
            .data;
        unsafe { std::mem::transmute(spring) }
    }

    pub fn set_rest_length(&mut self, length: f32) {
        self.rest_length = length;
        self.refresh_spring();
    }

    pub fn set_stiffness(&mut self, stiffness: f32) {
        self.stiffness = stiffness;
        self.refresh_spring();
    }

    pub fn set_damping(&mut self, damping: f32) {
        self.damping = damping;
        self.refresh_spring();
    }

    fn refresh_spring(&mut self) {
        if let Some(spring) = self.spring_mut() {
            spring.data.set_motor_position(JointAxis::LinX, self.rest_length, self.stiffness, self.damping);
        } else {
            warn!("Failed to refresh spring data")
        }
    }
}
