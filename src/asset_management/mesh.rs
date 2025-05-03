use std::ops::Range;

use bytemuck::{Pod, Zeroable};
use nalgebra::{Matrix4, Point, Vector2, Vector3};
use static_assertions::const_assert_eq;
use wgpu::{BufferAddress, BufferUsages, Device, VertexAttribute, VertexFormat};
use wgpu::util::{BufferInitDescriptor, DeviceExt};

use crate::asset_management::materialmanager::{FALLBACK_MATERIAL_ID, MaterialId};

#[derive(Copy, Clone)]
pub struct SimpleVertex3D {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

impl SimpleVertex3D {
    pub const fn upgrade(self) -> Vertex3D {
        Vertex3D {
            position:  Vector3::new(self.position[0], self.position[1], self.position[2]),
            tex_coord: Vector2::new(self.uv[0], self.uv[1]),
            normal:    Vector3::new(self.normal[0], self.normal[1], self.normal[2]),
            tangent:   Vector3::new(0.0, 0.0, 0.0),
            bitangent: Vector3::new(0.0, 0.0, 0.0),
            bone_indicies: [ 0xFF, 0xFF, 0xFF, 0xFF ],
            bone_weights:  [ 0.0, 0.0, 0.0, 0.0 ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Vertex3D {
    pub position:      Vector3<f32>,
    pub tex_coord:     Vector2<f32>,
    pub normal:        Vector3<f32>,
    pub tangent:       Vector3<f32>,
    pub bitangent:     Vector3<f32>,
    pub bone_indicies: [u32; 4],
    pub bone_weights:  [f32; 4],
}

impl Vertex3D {
    pub fn new(
        position:      Vector3<f32>,
        tex_coord:     Vector2<f32>,
        normal:        Vector3<f32>,
        tangent:       Vector3<f32>,
        bitangent:     Vector3<f32>,
        bone_indicies: &[u32],
        bone_weights:  &[f32],
    ) -> Self {
        Vertex3D {
            position,
            tex_coord,
            normal,
            tangent,
            bitangent,
            bone_indicies: pad_to_four(bone_indicies, 0xFF),
            bone_weights: pad_to_four(bone_weights, 0.0),
        }
    }
}

fn pad_to_four<T: Copy>(input: &[T], default: T) -> [T; 4] {
    let mut arr = [default; 4];
    let count = input.len().min(4);
    arr[..count].copy_from_slice(&input[..count]);
    arr
}

unsafe impl Zeroable for Vertex3D {}
unsafe impl Pod      for Vertex3D {}

#[allow(dead_code)]
#[derive(Debug)]
pub struct RuntimeMeshData {
    pub(crate) vertices_buf: wgpu::Buffer,
    pub(crate) vertices_num: usize,
    pub(crate) indices_buf:  Option<wgpu::Buffer>,
    pub(crate) indices_num:  usize,
}


#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Bone {
    pub(crate) transform: Matrix4<f32>,
}

unsafe impl Zeroable for Bone {}
unsafe impl Pod for Bone {}

impl From<&russimp_ng::bone::Bone> for Bone {
    fn from(value: &russimp_ng::bone::Bone) -> Self {
        let m = value.offset_matrix;
        Bone {
            transform: Matrix4::new(
                m.a1, m.a2, m.a3, m.a4, 
                m.b1, m.b2, m.b3, m.b4, 
                m.c1, m.c2, m.c3, m.c4, 
                m.d1, m.d2, m.d3, m.d4, 
            ),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Bones {
    pub names: Vec<String>,
    pub raw: Vec<Bone>,
}

impl Bones {
    pub fn name(&self, idx: usize) -> Option<&str> {
        self.names.get(idx).map(|s| s.as_str())
    }

    pub fn base_offset(&self, idx: usize) -> Option<&Matrix4<f32>> {
        self.raw.get(idx).map(|b| &b.transform)
    }

    pub fn names(&self) -> &[String] {
        &self.names
    }

    pub fn bones(&self) -> &[Bone] {
        &self.raw
    }

    pub fn none() -> Bones {
        Bones::default()
    }
}

#[derive(Debug)]
pub struct MeshVertexData<T> {
    pub(crate) vertices: Vec<T>,
    pub(crate) indices: Option<Vec<u32>>, 
}

#[derive(Debug)]
pub struct Mesh {
    pub(crate) data:     MeshVertexData<Vertex3D>,
    pub material_ranges: Vec<(MaterialId, Range<u32>)>,
    pub bones:           Bones,
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

    pub(crate) fn init_runtime(
        &mut self,
        device: &Device,
    ) -> RuntimeMesh {
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


impl Vertex3D {
    pub fn continuous_descriptor<'a>() -> wgpu::VertexBufferLayout<'a> {
        use crate::utils::sizes::*;

        const LAYOUT: wgpu::VertexBufferLayout = wgpu::VertexBufferLayout {
            array_stride: size_of::<Vertex3D>() as BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                VertexAttribute {
                    format: VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x2,
                    offset: VEC3_SIZE as BufferAddress,
                    shader_location: 1,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x3,
                    offset: (VEC3_SIZE + VEC2_SIZE) as BufferAddress,
                    shader_location: 2,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x3,
                    offset: (VEC3_SIZE * 2 + VEC2_SIZE) as BufferAddress,
                    shader_location: 3,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x3,
                    offset: (VEC3_SIZE * 3 + VEC2_SIZE) as BufferAddress,
                    shader_location: 4,
                },
                VertexAttribute {
                    format: VertexFormat::Uint32x4,
                    offset: (VEC3_SIZE * 4 + VEC2_SIZE) as BufferAddress,
                    shader_location: 5,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: (VEC3_SIZE * 4 + VEC2_SIZE + VEC4_SIZE) as BufferAddress,
                    shader_location: 6,
                },
            ],
        };

        const_assert_eq!(size_of::<Vertex3D>(), layout_size(&LAYOUT));

        LAYOUT
    }
}
