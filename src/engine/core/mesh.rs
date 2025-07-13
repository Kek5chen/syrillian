use nalgebra::Point;
use crate::core::Vertex3D;

#[derive(Debug)]
pub struct MeshVertexData<T> {
    pub(crate) vertices: Vec<T>,
    pub(crate) indices: Option<Vec<u32>>,
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

