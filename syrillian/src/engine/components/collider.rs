use crate::components::ColliderError::{
    DesyncedCollider, InvalidMesh, InvalidMeshRef, NoMeshRenderer,
};
use crate::components::{Component, RigidBodyComponent};
use crate::core::GameObjectId;
use crate::drawables::MeshRenderer;
use crate::engine::assets::Mesh;
use crate::World;
use log::{trace, warn};
use nalgebra::{Point3, Vector3};
use rapier3d::prelude::*;
use snafu::Snafu;

#[cfg(debug_assertions)]
use crate::assets::HShader;
#[cfg(debug_assertions)]
use crate::core::{ModelUniform, Vertex3D};
#[cfg(debug_assertions)]
use crate::drawables::{BoneData, MeshUniformIndex};
#[cfg(debug_assertions)]
use crate::rendering::uniform::ShaderUniform;
#[cfg(debug_assertions)]
use crate::rendering::{DrawCtx, Renderer};
#[cfg(debug_assertions)]
use nalgebra::Matrix4;
#[cfg(debug_assertions)]
use std::sync::RwLockWriteGuard;
#[cfg(debug_assertions)]
use wgpu::{IndexFormat, RenderPass};

#[derive(Debug)]
#[cfg(debug_assertions)]
struct ColliderDebugData {
    vertex_buf: wgpu::Buffer,
    index_buf: wgpu::Buffer,
    collider_indices_count: u32,
}

// Lol
//
// Okay, so.. Either a MeshRenderer is already assigned, which is likely.
// Then use that uniform as it'll auto update for us.
// If we don't have a MeshRenderer, we have to update ourselves.
// This will not handle the case where the backed uniform goes missing and needs to be
// phased out for something different. But I imagine the use cases where this will work
// are far overgrowing the opposite.
#[cfg(debug_assertions)]
enum PotentiallyPiggyBackedModelData {
    Backed(ShaderUniform<MeshUniformIndex>),
    Owned(ModelUniform, ShaderUniform<MeshUniformIndex>),
}

#[cfg(debug_assertions)]
impl PotentiallyPiggyBackedModelData {
    fn uniform(&self) -> &ShaderUniform<MeshUniformIndex> {
        match self {
            PotentiallyPiggyBackedModelData::Backed(uniform)
            | PotentiallyPiggyBackedModelData::Owned(_, uniform) => uniform,
        }
    }
}

pub struct Collider3D {
    pub phys_handle: ColliderHandle,
    linked_to_body: Option<RigidBodyHandle>,
    parent: GameObjectId,

    #[cfg(debug_assertions)]
    enable_debug_render: bool,

    #[cfg(debug_assertions)]
    collider_buffers: Option<ColliderDebugData>,
    #[cfg(debug_assertions)]
    model_data: Option<PotentiallyPiggyBackedModelData>,
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
        let shape = SharedShape::cuboid(scale.x / 2., scale.y / 2., scale.z / 2.);
        let collider = Self::default_collider(parent, shape);
        let phys_handle = World::instance()
            .physics
            .collider_set
            .insert(collider.clone());

        Collider3D {
            phys_handle,
            linked_to_body: None,
            parent,

            #[cfg(debug_assertions)]
            enable_debug_render: true,
            #[cfg(debug_assertions)]
            collider_buffers: None,
            #[cfg(debug_assertions)]
            model_data: None,
        }
    }

    fn update(&mut self, _world: &mut World) {
        let body_comp = (*self.parent).get_component::<RigidBodyComponent>();
        if let Some(body_comp) = body_comp {
            if self.linked_to_body.is_none() {
                self.link_to_rigid_body(Some(body_comp.body_handle));
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

    #[cfg(debug_assertions)]
    fn update_draw(
        &mut self,
        _world: &mut World,
        renderer: &Renderer,
        outer_transform: &Matrix4<f32>,
    ) {
        if self.collider_buffers.is_none() {
            self.generate_collider_data(&renderer.state.device);
        }

        let model_data = match &mut self.model_data {
            Some(data) => data,
            None => {
                self.setup_model(renderer);
                match &mut self.model_data {
                    Some(data) => data,
                    None => {
                        warn!("Model data could not be initialized in Collider3D");
                        return;
                    }
                }
            }
        };

        match model_data {
            PotentiallyPiggyBackedModelData::Owned(model, uniform) => {
                model.update(self.parent, outer_transform);

                renderer.state.queue.write_buffer(
                    uniform.buffer(MeshUniformIndex::MeshData),
                    0,
                    bytemuck::bytes_of(model),
                );
            }
            _ => (),
        }
    }

    #[cfg(debug_assertions)]
    fn draw(&self, _world: &World, ctx: &DrawCtx) {
        if !ctx.frame.debug.colliders_edges || !self.enable_debug_render {
            return;
        }

        let Some(collider_buf) = &self.collider_buffers else {
            warn!("Collider draw data not initialized for {}.", self.parent.name);
            return;
        };

        let Some(model_data) = &self.model_data else {
            warn!("Model data not set in Collider3D before draw call");
            return;
        };

        let mut pass = ctx.pass.write().unwrap();

        draw_collider_edges(ctx, collider_buf, &model_data.uniform(), &mut pass)
    }

    fn delete(&mut self, world: &mut World) {
        world.physics.collider_set.remove(
            self.phys_handle,
            &mut world.physics.island_manager,
            &mut world.physics.rigid_body_set,
            false,
        );
    }

    fn parent(&self) -> GameObjectId {
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

    fn default_collider(parent: GameObjectId, shape: SharedShape) -> Collider {
        ColliderBuilder::new(shape)
            .density(1.0)
            .friction(0.999)
            .user_data(parent.as_ffi() as u128)
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

    #[cfg(debug_assertions)]
    pub fn set_debug_render(&mut self, enabled: bool) {
        self.enable_debug_render = enabled;
    }

    #[cfg(debug_assertions)]
    pub fn is_debug_render_enabled(&self) -> bool {
        self.enable_debug_render
    }

    #[cfg(debug_assertions)]
    fn setup_model(&mut self, renderer: &Renderer) {
        let mesh_data = if let Some(mesh_data) = self
            .parent
            .drawable::<MeshRenderer>()
            .and_then(|renderer| renderer.mesh_data())
            .map(|mesh_data| mesh_data.uniform.clone())
        {
            PotentiallyPiggyBackedModelData::Backed(mesh_data)
        } else {
            let model_bgl = renderer.cache.bgl_model();
            let model = ModelUniform::empty();
            let uniform = ShaderUniform::<MeshUniformIndex>::builder(&model_bgl)
                .with_buffer_data(&model)
                .with_buffer_data_slice(&BoneData::DUMMY_BONE)
                .build(&renderer.state.device);

            PotentiallyPiggyBackedModelData::Owned(model, uniform)
        };

        self.model_data = Some(mesh_data);
    }

    /// Returns Vertices and Indices
    #[cfg(debug_assertions)]
    fn generate_collider_data(&mut self, device: &wgpu::Device) {
        use wgpu::BufferUsages;
        use wgpu::util::{BufferInitDescriptor, DeviceExt};

        let Some(collider) = self.get_collider() else {
            warn!("No collider attached to Collider 3D component");
            return;
        };

        let (vertices, indices) = collider.shared_shape().to_trimesh();

        let vertices: Vec<_> = vertices.iter().map(|v| Vertex3D::basic(v.coords)).collect();

        let vertex_bytes = bytemuck::cast_slice(&vertices[..]);

        if cfg!(debug_assertions) {
            let vertex_bytes_size = size_of_val(vertex_bytes) as i64;
            let expected_vertex_bytes_size = (indices
                .iter()
                .flatten()
                .max()
                .map(|i| *i as i64)
                .unwrap_or(-1)
                + 1)
                * size_of::<Vertex3D>() as i64;

            assert_eq!(vertex_bytes_size, expected_vertex_bytes_size);
        }

        let vertex_buf = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Collider Debug Vertex Buffer"),
            contents: vertex_bytes,
            usage: BufferUsages::VERTEX,
        });
        let index_buf = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Collider Debug Index Buffer"),
            contents: bytemuck::cast_slice(&indices[..]),
            usage: BufferUsages::INDEX,
        });

        let data = ColliderDebugData {
            vertex_buf,
            index_buf,
            collider_indices_count: indices.len() as u32 * 3,
        };

        self.collider_buffers = Some(data);
    }
}

pub trait MeshShapeExtra<T> {
    fn mesh(mesh: &Mesh) -> Option<T>;
    fn mesh_convex_hull(mesh: &Mesh) -> Option<SharedShape>;
    fn local_aabb_mesh(&self) -> (Vec<Point3<f32>>, Vec<[u32; 3]>);
    fn to_trimesh(&self) -> (Vec<Point3<f32>>, Vec<[u32; 3]>);
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

    fn local_aabb_mesh(&self) -> (Vec<Point3<f32>>, Vec<[u32; 3]>) {
        let aabb = self.compute_local_aabb();
        aabb.to_trimesh()
    }

    fn to_trimesh(&self) -> (Vec<Point3<f32>>, Vec<[u32; 3]>) {
        trace!("[Collider] Type: {:?}", self.as_typed_shape());
        match self.as_typed_shape() {
            TypedShape::Ball(s) => s.to_trimesh(10, 10),
            TypedShape::Cuboid(s) => s.to_trimesh(),
            TypedShape::Capsule(s) => s.to_trimesh(10, 10),
            TypedShape::Segment(_) => self.local_aabb_mesh(),
            TypedShape::Triangle(s) => (s.vertices().to_vec(), vec![[0, 1, 2]]),
            TypedShape::Voxels(s) => s.to_trimesh(),
            TypedShape::TriMesh(s) => (s.vertices().to_vec(), s.indices().to_vec()),
            TypedShape::Polyline(_) => self.local_aabb_mesh(),
            TypedShape::HalfSpace(_) => self.local_aabb_mesh(),
            TypedShape::HeightField(s) => s.to_trimesh(),
            TypedShape::Compound(_) => self.local_aabb_mesh(),
            TypedShape::ConvexPolyhedron(s) => s.to_trimesh(),
            TypedShape::Cylinder(s) => s.to_trimesh(10),
            TypedShape::Cone(s) => s.to_trimesh(10),
            TypedShape::RoundCuboid(_) => self.local_aabb_mesh(),
            TypedShape::RoundTriangle(_) => self.local_aabb_mesh(),
            TypedShape::RoundCylinder(_) => self.local_aabb_mesh(),
            TypedShape::RoundCone(_) => self.local_aabb_mesh(),
            TypedShape::RoundConvexPolyhedron(_) => self.local_aabb_mesh(),
            TypedShape::Custom(_) => self.local_aabb_mesh(),
        }
    }
}

#[cfg(debug_assertions)]
fn draw_collider_edges(
    ctx: &DrawCtx,
    debug_data: &ColliderDebugData,
    model_uniform: &ShaderUniform<MeshUniformIndex>,
    pass: &mut RwLockWriteGuard<RenderPass>,
) {
    use nalgebra::Vector4;
    use wgpu::ShaderStages;

    const COLOR: Vector4<f32> = Vector4::new(0.0, 1.0, 0.2, 1.0);

    let shader = ctx.frame.cache.shader(HShader::DEBUG_EDGES);
    crate::must_pipeline!(pipeline = shader, ctx.pass_type => return);

    pass.set_pipeline(pipeline);
    pass.set_push_constants(ShaderStages::FRAGMENT, 0, bytemuck::bytes_of(&COLOR));
    pass.set_bind_group(1, model_uniform.bind_group(), &[]);

    pass.set_vertex_buffer(0, debug_data.vertex_buf.slice(..));
    pass.set_index_buffer(debug_data.index_buf.slice(..), IndexFormat::Uint32);
    pass.draw_indexed(0..debug_data.collider_indices_count, 0, 0..1);
}
