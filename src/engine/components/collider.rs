use crate::World;
use crate::components::ColliderError::{
    DesyncedCollider, InvalidMesh, InvalidMeshRef, NoMeshRenderer,
};
use crate::components::{Component, RigidBodyComponent};
use crate::core::GameObjectId;
use crate::drawables::MeshRenderer;
use crate::engine::assets::Mesh;
use log::{trace, warn};
use nalgebra::Vector3;
use rapier3d::prelude::*;
use snafu::Snafu;

pub struct Collider3D {
    pub phys_handle: ColliderHandle,
    linked_to_body: Option<RigidBodyHandle>,
    parent: GameObjectId,
}

#[derive(Debug, Snafu)]
#[snafu(context(suffix(Err)))]
pub enum ColliderError {
    #[snafu(display(
        "Cannot use Mesh as Collider since no MeshRenderer is attached to the Object"
    ))]
    NoMeshRenderer,

    #[snafu(display("A mesh renderer was storing an invalid mesh reference"))]
    InvalidMeshRef,

    #[snafu(display("No collider was attached to the object"))]
    DesyncedCollider,

    #[snafu(display("The collider mesh was invalid"))]
    InvalidMesh,
}

impl Component for Collider3D {
    fn new(parent: GameObjectId) -> Self
    where
        Self: Sized,
    {
        let scale = parent.transform.scale();
        let shape = SharedShape::cuboid(scale.x, scale.y, scale.z);
        let collider = Self::default_collider(shape);
        let phys_handle = World::instance()
            .physics
            .collider_set
            .insert(collider.clone());

        Collider3D {
            phys_handle,
            linked_to_body: None,
            parent,
        }
    }

    fn update(&mut self) {
        let body_comp = (*self.parent).get_component::<RigidBodyComponent>();
        if let Some(body_comp) = body_comp {
            if self.linked_to_body.is_none() {
                self.link_to_rigid_body(Some(body_comp.borrow().body_handle));
                let coll = self.get_collider_mut().unwrap();
                coll.set_translation(Vector3::zeros());
                coll.set_rotation(Rotation::identity());
                // TODO: Sync Scale to coll
            } // the linked rigid body will control the collider or
        } else {
            // the collider just takes on the parent transformations
            let translation = self.parent.transform.position();
            let rotation = self.parent.transform.rotation();
            let coll = self.get_collider_mut().unwrap();
            coll.set_translation(translation);
            coll.set_rotation(rotation);
            // TODO: Sync Scale to coll
        }
    }

    fn delete(&mut self) {
        let world = World::instance();

        world.physics.collider_set.remove(
            self.phys_handle,
            &mut world.physics.island_manager,
            &mut world.physics.rigid_body_set,
            true,
        );
    }

    fn get_parent(&self) -> GameObjectId {
        self.parent
    }
}

impl Collider3D {
    pub fn get_collider(&self) -> Option<&Collider> {
        World::instance().physics.collider_set.get(self.phys_handle)
    }

    pub fn get_collider_mut(&mut self) -> Option<&mut Collider> {
        World::instance()
            .physics
            .collider_set
            .get_mut(self.phys_handle)
    }

    fn default_collider(shape: SharedShape) -> Collider {
        ColliderBuilder::new(shape)
            .density(1.0)
            .friction(0.999)
            .build()
    }

    pub fn link_to_rigid_body(&mut self, h_body: Option<RigidBodyHandle>) {
        let world = World::instance();

        world.physics.collider_set.set_parent(
            self.phys_handle,
            h_body,
            &mut world.physics.rigid_body_set,
        );

        self.linked_to_body = h_body;
    }

    pub fn use_mesh(&mut self) {
        if let Err(e) = self.try_use_mesh() {
            warn!("{e}");
        }
    }
    
    /// Same as Collider3D::use_mesh but without a warning. This is nice for guarantee-less iteration
    pub fn please_use_mesh(&mut self) {
        _ = self.try_use_mesh();
    }

    pub fn try_use_mesh(&mut self) -> Result<(), ColliderError> {
        let mesh_renderer = self
            .parent
            .drawable::<MeshRenderer>()
            .ok_or(NoMeshRenderer)?;

        let handle = mesh_renderer.mesh();
        let mesh = World::instance()
            .assets
            .meshes
            .try_get(handle)
            .ok_or(InvalidMeshRef)?;
        let collider = self.get_collider_mut().ok_or(DesyncedCollider)?;
        let collider_shape = SharedShape::mesh(&mesh).ok_or(InvalidMesh)?;

        collider.set_shape(collider_shape);

        Ok(())
    }
}

pub trait MeshShapeExtra<T> {
    fn mesh(mesh: &Mesh) -> Option<T>;
    fn mesh_convex_hull(mesh: &Mesh) -> Option<SharedShape>;
}

impl MeshShapeExtra<SharedShape> for SharedShape {
    fn mesh(mesh: &Mesh) -> Option<SharedShape> {
        trace!(
            "Loading collider mesh with {} vertices",
            mesh.data.vertices.len()
        );
        let vertices = mesh.data.make_point_cloud();
        let indices = mesh.data.make_triangle_indices();
        match SharedShape::trimesh(vertices, indices) {
            Ok(shape) => Some(shape),
            Err(e) => {
                warn!("Mesh could not be processed as a trimesh: {e}");
                None
            }
        }
    }

    fn mesh_convex_hull(mesh: &Mesh) -> Option<SharedShape> {
        let vertices = mesh.data.make_point_cloud();
        SharedShape::convex_hull(&vertices)
    }
}
