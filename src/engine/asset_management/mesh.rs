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

    pub(crate) fn init_runtime(&mut self, device: &Device) -> RuntimeMesh {
        let v_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("3D Object Vertex Buffer"),
            contents: bytemuck::cast_slice(self.data.vertices.as_slice()),
            usage: BufferUsages::VERTEX,
        });
        let i_buffer = self.data.indices.as_ref().map(|indices| {
            device.create_buffer_init(&BufferInitDescriptor {
                label: Some("3D Object Index Buffer"),
                contents: bytemuck::cast_slice(indices.as_slice()),
                usage: BufferUsages::INDEX,
            })
        });

        let runtime_mesh_data = RuntimeMeshData {
            vertices_buf: v_buffer,
            vertices_num: self.data.vertices.len(),
            indices_buf: i_buffer,
            indices_num: self
                .data
                .indices
                .as_ref()
                .map(|i| i.len())
                .unwrap_or_default(),
        };

        RuntimeMesh {
            data: runtime_mesh_data,
        }
    }
}
