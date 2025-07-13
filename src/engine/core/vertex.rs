use nalgebra::{Vector2, Vector3};
use static_assertions::const_assert_eq;
use wgpu::{BufferAddress, VertexAttribute, VertexFormat};

#[derive(Copy, Clone)]
pub struct SimpleVertex3D {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

impl SimpleVertex3D {
    pub const fn upgrade(self) -> Vertex3D {
        Vertex3D {
            position: Vector3::new(self.position[0], self.position[1], self.position[2]),
            tex_coord: Vector2::new(self.uv[0], self.uv[1]),
            normal: Vector3::new(self.normal[0], self.normal[1], self.normal[2]),
            tangent: Vector3::new(0.0, 0.0, 0.0),
            bitangent: Vector3::new(0.0, 0.0, 0.0),
            bone_indices: [0xFF, 0xFF, 0xFF, 0xFF],
            bone_weights: [0.0, 0.0, 0.0, 0.0],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex3D {
    pub position: Vector3<f32>,
    pub tex_coord: Vector2<f32>,
    pub normal: Vector3<f32>,
    pub tangent: Vector3<f32>,
    pub bitangent: Vector3<f32>,
    pub bone_indices: [u32; 4],
    pub bone_weights: [f32; 4],
}

impl Vertex3D {
    pub fn new(
        position: Vector3<f32>,
        tex_coord: Vector2<f32>,
        normal: Vector3<f32>,
        tangent: Vector3<f32>,
        bitangent: Vector3<f32>,
        bone_indices: &[u32],
        bone_weights: &[f32],
    ) -> Self {
        Vertex3D {
            position,
            tex_coord,
            normal,
            tangent,
            bitangent,
            bone_indices: pad_to_four(bone_indices, 0xFF),
            bone_weights: pad_to_four(bone_weights, 0.0),
        }
    }
    pub const fn continuous_descriptor<'a>() -> wgpu::VertexBufferLayout<'a> {
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
                    offset: VEC3_SIZE,
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
                    offset: (VEC4_SIZE + VEC3_SIZE * 4 + VEC2_SIZE) as BufferAddress,
                    shader_location: 6,
                },
            ],
        };

        const_assert_eq!(size_of::<Vertex3D>(), vertex_layout_size(&LAYOUT));

        LAYOUT
    }
}

fn pad_to_four<T: Copy>(input: &[T], default: T) -> [T; 4] {
    let mut arr = [default; 4];
    let count = input.len().min(4);
    arr[..count].copy_from_slice(&input[..count]);
    arr
}
