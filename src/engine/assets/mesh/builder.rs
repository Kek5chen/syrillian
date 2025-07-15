use crate::assets::{HMaterial, Mesh, MeshVertexData};
use crate::core::{Bones, Vertex3D};
use std::ops::Range;

#[derive(Debug, Clone)]
pub struct MeshBuilder {
    vertices: Vec<Vertex3D>,
    indices: Option<Vec<u32>>,
    single_material: Option<HMaterial>,
    material_ranges: Option<Vec<(HMaterial, Range<u32>)>>,
    bones: Option<Bones>,
}

impl MeshBuilder {
    pub fn new(vertices: Vec<Vertex3D>) -> Self {
        MeshBuilder {
            vertices,
            indices: None,
            single_material: None,
            material_ranges: None,
            bones: None,
        }
    }

    pub fn build(self) -> Mesh {
        let mut material_ranges = self.material_ranges.unwrap_or_default();

        if material_ranges.is_empty() {
            let vert_count = self
                .indices
                .as_ref()
                .map(|indices| indices.len())
                .unwrap_or_else(|| self.vertices.len());

            let mat = self.single_material.unwrap_or(HMaterial::FALLBACK);
            material_ranges.push((mat, 0u32..vert_count as u32));
        }

        Mesh {
            data: MeshVertexData::new(self.vertices, self.indices),
            material_ranges,
            bones: self.bones.unwrap_or_default(),
        }
    }

    pub fn with_bones(mut self, bones: Bones) -> Self {
        self.bones = Some(bones);
        self
    }

    // TODO: Move texturing to MeshRenderer
    pub fn with_one_texture(mut self, material: HMaterial) -> Self {
        self.single_material = Some(material);
        self.material_ranges = None;
        self
    }

    pub fn with_many_textures(mut self, materials: Vec<(HMaterial, Range<u32>)>) -> Self {
        self.material_ranges = Some(materials);
        self.single_material = None;
        self
    }

    pub fn with_indices(mut self, indices: Vec<u32>) -> Self {
        self.indices = Some(indices);
        self
    }
}

impl Into<Mesh> for MeshBuilder {
    fn into(self) -> Mesh {
        self.build()
    }
}
