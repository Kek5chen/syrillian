use log::warn;
use nalgebra::{Isometry3, Point3};
use rapier3d::prelude::{FixedJoint, FixedJointBuilder, ImpulseJointHandle};
use snafu::{Snafu, ensure};

use crate::{World, components::{Component, NewComponent, RigidBodyComponent}, core::GameObjectId};

#[derive(Debug, Snafu)]
#[snafu(context(suffix(Err)))]
pub enum FixedJointComponentError {
    #[snafu(display("FixedJointComponent: Connector doesn't exist"))]
    InvalidConnector,
    #[snafu(display("FixedJointComponent: Parent doesn't have a rigid body"))]
    NoParentRigidBody,
    #[snafu(display("FixedJointComponent: Connector doesn't have a rigid body"))]
    NoConnectorRigidBody,
}

pub struct FixedJointConnnector {
    parent: GameObjectId,
    connected: Option<GameObjectId>,
    handle: Option<ImpulseJointHandle>,
    local_anchor1: Point3<f32>,
    local_anchor2: Point3<f32>,
    local_frame1: Isometry3<f32>,
    local_frame2: Isometry3<f32>,
}

impl NewComponent for FixedJointConnnector {
    fn new(parent: GameObjectId) -> Self {
        FixedJointConnnector { 
            parent, 
            connected: None, 
            handle: None, 
            local_anchor1: Point3::origin(), 
            local_anchor2: Point3::origin(), 
            local_frame1: Isometry3::identity(), 
            local_frame2: Isometry3::identity() 
        }
    }
}

impl Component for FixedJointConnnector {
    fn delete(&mut self, world: &mut crate::World) {
        self.disconnect(world);
    }
}

impl FixedJointConnnector {
    pub fn connect_to(&mut self, body: GameObjectId) {
        if let Err(e) = self.try_connect_to(body) {
            warn!("{e}");
        }
    }
    
    pub fn try_connect_to(&mut self, body: GameObjectId) -> Result<(), FixedJointComponentError> {
        ensure!(body.exists(), InvalidConnectorErr);
        
        let self_rb = self
            .parent
            .get_component::<RigidBodyComponent>()
            .ok_or(NoParentRigidBodyErr)
            .unwrap()
            .body_handle;
        
        let other_rb = body
            .get_component::<RigidBodyComponent>()
            .ok_or(NoConnectorRigidBodyErr)
            .unwrap()
            .body_handle;
        
        let joint = FixedJointBuilder::new()
            .local_anchor1(self.local_anchor1)
            .local_anchor2(self.local_anchor2)
            .local_frame1(self.local_frame1)
            .local_frame2(self.local_frame2)
            .build();
        
        let handle = self
            .parent
            .world()
            .physics
            .impulse_joint_set
            .insert(self_rb, other_rb, joint, true);
        
        self.connected = Some(body);
        self.handle = Some(handle);
        
        Ok(())
    }
    
    pub fn disconnect(&mut self, world: &mut World) {
        if let Some(joint) = self.handle {
            world.physics.impulse_joint_set.remove(joint, false);
            self.handle = None;
            self.connected = None;
        }
    }
    
    pub fn joint(&self) -> Option<&FixedJoint> {
        self
            .parent
            .world()
            .physics
            .impulse_joint_set
            .get(self.handle?)?
            .data
            .as_fixed()
    }
    
    pub fn joint_mut(&self) -> Option<&mut FixedJoint> {
        self
            .parent
            .world()
            .physics
            .impulse_joint_set
            .get_mut(self.handle?, false)?
            .data
            .as_fixed_mut()
    }
    
    pub fn set_local_anchor1(&mut self, anchor: Point3<f32>) {
        self.local_anchor1 = anchor;
        if let Some(joint) = self.joint_mut() {
            joint.set_local_anchor1(anchor);
        }
    }
    
    pub fn set_local_anchor2(&mut self, anchor: Point3<f32>) {
        self.local_anchor2 = anchor;
        if let Some(joint) = self.joint_mut() {
            joint.set_local_anchor2(anchor);
        }
    }
    
    pub fn set_local_frame1(&mut self, frame: Isometry3<f32>) {
        self.local_frame1 = frame;
        if let Some(joint) = self.joint_mut() {
            joint.set_local_frame1(frame);
        }
    }
    
    pub fn set_local_frame2(&mut self, frame: Isometry3<f32>) {
        self.local_frame2 = frame;
        if let Some(joint) = self.joint_mut() {
            joint.set_local_frame2(frame);
        }
    }
}