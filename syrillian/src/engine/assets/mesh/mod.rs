mod builder;

use crate::assets::mesh::builder::MeshBuilder;
use crate::assets::scene_loader::SceneLoader;
use crate::core::{Bones, Vertex3D};
use crate::engine::assets::generic_store::{HandleName, Store, StoreDefaults, StoreType};
use crate::engine::assets::{HMaterial, HMesh, H};
use crate::store_add_checked;
use crate::utils::UNIT_SQUARE_VERT;
use nalgebra::Point;
use std::fmt::Debug;
use std::ops::Range;

const CUBE_OBJ: &[u8] = include_bytes!("preset_meshes/cube.obj");
const DEBUG_ARROW: &[u8] = include_bytes!("preset_meshes/debug_arrow.obj");
const SPHERE: &[u8] = include_bytes!("preset_meshes/small_sphere.obj");

#[derive(Debug, Clone)]
pub struct Mesh {
    pub(crate) data: MeshVertexData<Vertex3D>,
    pub material_ranges: Vec<(HMaterial, Range<u32>)>,
    pub bones: Bones,
}

#[derive(Debug, Clone)]
pub struct MeshVertexData<T: Debug + Clone> {
    pub(crate) vertices: Vec<T>,
    pub(crate) indices: Option<Vec<u32>>,
}

impl Mesh {
    pub fn builder(vertices: Vec<Vertex3D>) -> MeshBuilder {
        MeshBuilder::new(vertices)
    }

    #[inline]
    pub fn vertex_count(&self) -> usize {
        self.data.vertices.len()
    }

    #[inline]
    pub fn indices_count(&self) -> usize {
        self.indices().map_or(0, <[u32]>::len)
    }

    #[inline]
    pub fn vertices(&self) -> &[Vertex3D] {
        &self.data.vertices
    }

    #[inline]
    pub fn indices(&self) -> Option<&[u32]> {
        self.data.indices.as_ref().map(|i| i.as_slice())
    }
}

impl MeshVertexData<Vertex3D> {
    pub fn new(vertices: Vec<Vertex3D>, indices: Option<Vec<u32>>) -> Self {
        MeshVertexData { vertices, indices }
    }

    pub fn make_triangle_indices(&self) -> Vec<[u32; 3]> {
        match &self.indices {
            None => (0u32..self.vertices.len() as u32)
                .collect::<Vec<_>>()
                .as_chunks()
                .0
                .to_vec(),
            Some(indices) => indices.as_chunks().0.to_vec(),
        }
    }

    pub fn make_point_cloud(&self) -> Vec<Point<f32, 3>> {
        self.vertices.iter().map(|v| v.position.into()).collect()
    }
}

impl H<Mesh> {
    const UNIT_SQUARE_ID: u32 = 0;
    const UNIT_CUBE_ID: u32 = 1;
    const DEBUG_ARROW_ID: u32 = 2;
    const SPHERE_ID: u32 = 3;

    pub const UNIT_SQUARE: HMesh = H::new(Self::UNIT_SQUARE_ID);
    pub const UNIT_CUBE: HMesh = H::new(Self::UNIT_CUBE_ID);
    pub const DEBUG_ARROW: HMesh = H::new(Self::DEBUG_ARROW_ID);
    pub const SPHERE: HMesh = H::new(Self::SPHERE_ID);
}

impl StoreDefaults for Mesh {
    fn populate(store: &mut Store<Self>) {
        let unit_square = Mesh::builder(UNIT_SQUARE_VERT.to_vec()).build();
        store_add_checked!(store, HMesh::UNIT_SQUARE_ID, unit_square);

        let unit_cube = SceneLoader::load_first_mesh_from_buffer(CUBE_OBJ, "obj")
            .expect("Cube Mesh load failed")
            .expect("Cube Mesh doesn't have a mesh");
        store_add_checked!(store, HMesh::UNIT_CUBE_ID, unit_cube);

        let debug_arrow = SceneLoader::load_first_mesh_from_buffer(DEBUG_ARROW, "obj")
            .ok()
            .flatten()
            .expect("Debug Arrow Mesh load failed");
        store_add_checked!(store, HMesh::DEBUG_ARROW_ID, debug_arrow);

        let sphere = SceneLoader::load_first_mesh_from_buffer(SPHERE, "obj")
            .ok()
            .flatten()
            .expect("Sphere Mesh load failed");
        store_add_checked!(store, HMesh::SPHERE_ID, sphere);
    }
}

impl StoreType for Mesh {
    fn name() -> &'static str {
        "Mesh"
    }

    fn ident_fmt(handle: H<Self>) -> HandleName<Self> {
        match handle.id() {
            HMesh::UNIT_SQUARE_ID => HandleName::Static("Unit Square"),
            HMesh::UNIT_CUBE_ID => HandleName::Static("Unit Cube"),
            _ => HandleName::Id(handle),
        }
    }
}
