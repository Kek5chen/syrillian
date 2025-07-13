use nalgebra::Vector3;
use rapier3d::prelude::*;
use std::time::{Duration, Instant};

pub struct PhysicsSimulator {
    pub gravity: Vector3<Real>,
    pub rigid_body_set: RigidBodySet,
    pub collider_set: ColliderSet,
    pub integration_parameters: IntegrationParameters,
    pub physics_pipeline: PhysicsPipeline,
    pub island_manager: IslandManager,
    pub broad_phase: Box<dyn BroadPhase>,
    pub narrow_phase: NarrowPhase,
    pub impulse_joint_set: ImpulseJointSet,
    pub multibody_joint_set: MultibodyJointSet,
    pub ccd_solver: CCDSolver,
    pub query_pipeline: QueryPipeline,
    pub physics_hooks: (),
    pub event_handler: (),
    pub last_update: Instant,
    pub timestep: Duration,
}

const EARTH_GRAVITY: f32 = 9.81;

impl Default for PhysicsSimulator {
    fn default() -> Self {
        PhysicsSimulator {
            gravity: Vector3::new(0.0, -EARTH_GRAVITY, 0.0),
            rigid_body_set: RigidBodySet::default(),
            collider_set: ColliderSet::default(),
            integration_parameters: IntegrationParameters::default(),
            physics_pipeline: PhysicsPipeline::default(),
            island_manager: IslandManager::default(),
            broad_phase: Box::<DefaultBroadPhase>::default(),
            narrow_phase: NarrowPhase::default(),
            impulse_joint_set: ImpulseJointSet::default(),
            multibody_joint_set: MultibodyJointSet::default(),
            ccd_solver: CCDSolver::default(),
            query_pipeline: QueryPipeline::default(),
            physics_hooks: (),
            event_handler: (),
            last_update: Instant::now(),
            timestep: Duration::from_millis(1000 / 60),
        }
    }
}

impl PhysicsSimulator {
    pub fn step(&mut self) {
        self.physics_pipeline.step(
            &self.gravity,
            &self.integration_parameters,
            &mut self.island_manager,
            self.broad_phase.as_mut(),
            &mut self.narrow_phase,
            &mut self.rigid_body_set,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.ccd_solver,
            Some(&mut self.query_pipeline),
            &(), // no hooks yet
            &(), // no events yet
        );
        self.query_pipeline.update(&self.collider_set)
    }
}
