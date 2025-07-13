use std::ops::Range;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{BufferUsages, Device};

use crate::asset_management::materialmanager::{FALLBACK_MATERIAL_ID, MaterialId};
use crate::core::{Bones, MeshVertexData, Vertex3D};

#[allow(dead_code)]
#[derive(Debug)]
pub struct RuntimeMeshData {
    pub(crate) vertices_buf: wgpu::Buffer,
    pub(crate) vertices_num: usize,
    pub(crate) indices_buf: Option<wgpu::Buffer>,
    pub(crate) indices_num: usize,
}

#[derive(Debug)]
pub struct Mesh {
    pub(crate) data: MeshVertexData<Vertex3D>,
    pub material_ranges: Vec<(MaterialId, Range<u32>)>,
    pub bones: Bones,
}

#[derive(Debug)]
pub struct RuntimeMesh {
    pub data: RuntimeMeshData,
}

impl Mesh {
    pub fn new(
        vertices: Vec<Vertex3D>,
        indices: Option<Vec<u32>>,
        material_ranges: Option<Vec<(MaterialId, Range<u32>)>>,
        bones: Bones,
    ) -> Box<Mesh> {
        let mut material_ranges = material_ranges.unwrap_or_default();

        if material_ranges.is_empty() {
            if let Some(indices) = &indices {
                material_ranges.push((FALLBACK_MATERIAL_ID, 0u32..indices.len() as u32))
            } else {
                material_ranges.push((FALLBACK_MATERIAL_ID, 0u32..vertices.len() as u32))
            }
        }

        Box::new(Mesh {
            data: MeshVertexData::<Vertex3D> { vertices, indices },
            material_ranges,
            bones,
        })
    }

    #[inline]
    pub fn vertex_count(&self) -> usize {
        self.data.vertices.len()
    }

    #[inline]
    pub fn indices_count(&self) -> usize {
        self
            .indices()
            .map(<[u32]>::len)
            .unwrap_or(0)
    }

    #[inline]
    pub fn vertices(&self) -> &[Vertex3D] {
        &self.data.vertices
    }

    #[inline]
    pub fn indices(&self) -> Option<&[u32]> {
        self.data.indices.as_ref().map(|i| i.as_slice())
    }

    pub(crate) fn init_runtime(&mut self, device: &Device) -> RuntimeMesh {
        RuntimeMesh {
            data: RuntimeMeshData::new(self, device),
        }
    }
}

impl RuntimeMeshData {
    fn new(mesh: &Mesh, device: &Device) -> Self {
        let vertices_num = mesh.vertex_count();
        let indices_num = mesh.indices_count();

        let vertices_buf = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("3D Object Vertex Buffer"),
            contents: bytemuck::cast_slice(mesh.vertices()),
            usage: BufferUsages::VERTEX,
        });

        let indices_buf = mesh.indices().map(|indices| {
            device.create_buffer_init(&BufferInitDescriptor {
                label: Some("3D Object Index Buffer"),
                contents: bytemuck::cast_slice(indices),
                usage: BufferUsages::INDEX,
            })
        });

        RuntimeMeshData {
            vertices_buf,
            vertices_num,
            indices_buf,
            indices_num,
        }
    }
}