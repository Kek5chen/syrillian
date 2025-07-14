use crate::core::{Bones, Vertex3D};
use crate::engine::assets::generic_store::{HandleName, Store, StoreDefaults, StoreType};
use crate::engine::assets::{H, HMaterial, HMesh};
use crate::store_add_checked;
use crate::utils::{CUBE_IDX, CUBE_VERT, UNIT_SQUARE_VERT};
use nalgebra::Point;
use std::fmt::Debug;
use std::ops::Range;

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
    pub fn new(
        vertices: Vec<Vertex3D>,
        indices: Option<Vec<u32>>,
        material_ranges: Option<Vec<(HMaterial, Range<u32>)>>,
        bones: Bones,
    ) -> Self {
        let mut material_ranges = material_ranges.unwrap_or_default();

        if material_ranges.is_empty() {
            let vert_count = indices
                .as_ref()
                .map(|indices| indices.len())
                .unwrap_or_else(|| vertices.len());

            material_ranges.push((HMaterial::FALLBACK, 0u32..vert_count as u32));
        }

        Mesh {
            data: MeshVertexData::<Vertex3D> { vertices, indices },
            material_ranges,
            bones,
        }
    }

    #[inline]
    pub fn vertex_count(&self) -> usize {
        self.data.vertices.len()
    }

    #[inline]
    pub fn indices_count(&self) -> usize {
        self.indices().map(<[u32]>::len).unwrap_or(0)
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
    pub fn make_triangle_indices(&self) -> Vec<[u32; 3]> {
        match &self.indices {
            None => (0u32..self.vertices.len() as u32)
                .collect::<Vec<_>>()
                .chunks_exact(3)
                .map(|chunk| [chunk[0], chunk[1], chunk[2]])
                .collect::<Vec<[u32; 3]>>(),
            Some(indices) => indices
                .chunks_exact(3)
                .map(|chunk| [chunk[0], chunk[1], chunk[2]])
                .collect(),
        }
    }

    pub fn make_point_cloud(&self) -> Vec<Point<f32, 3>> {
        self.vertices
            .iter()
            .map(|v| v.position.into())
            .map(|v: Point<f32, 3>| v * 1.0f32)
            .clone()
            .collect()
    }
}

impl H<Mesh> {
    const UNIT_SQUARE_ID: u32 = 0;
    const UNIT_CUBE_ID: u32 = 1;

    pub const UNIT_SQUARE: HMesh = H::new(Self::UNIT_SQUARE_ID);
    pub const UNIT_CUBE: HMesh = H::new(Self::UNIT_CUBE_ID);
}

impl StoreDefaults for Mesh {
    fn populate(store: &mut Store<Self>) {
        let unit_square = Mesh::new(UNIT_SQUARE_VERT.to_vec(), None, None, Bones::none());
        let unit_cube = Mesh::new(CUBE_VERT.into(), Some(CUBE_IDX.into()), None, Bones::none());

        store_add_checked!(store, HMesh::UNIT_SQUARE_ID, unit_square);
        store_add_checked!(store, HMesh::UNIT_CUBE_ID, unit_cube);
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
