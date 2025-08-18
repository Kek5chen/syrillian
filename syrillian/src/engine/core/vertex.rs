use nalgebra::{Vector2, Vector3};
use static_assertions::const_assert_eq;
use wgpu::{BufferAddress, VertexAttribute, VertexFormat};

/// Convenience vertex used when constructing static meshes.
#[derive(Copy, Clone)]
pub struct SimpleVertex3D {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

impl SimpleVertex3D {
    /// Converts this simplified vertex into a full [`Vertex3D`].
    /// This is not recommended as the tangent and bitangent calculation is just a rought approximation.
    pub const fn upgrade(self) -> Vertex3D {
        let px = self.position[0];
        let py = self.position[1];
        let pz = self.position[2];
        let nx = self.normal[0];
        let ny = self.normal[1];
        let nz = self.normal[2];
        let u = self.uv[0];
        let v = self.uv[1];

        let world_up = if ny.abs() < 0.999 {
            [0.0, 1.0, 0.0]
        } else {
            [1.0, 0.0, 0.0]
        };

        let dot = nx * world_up[0] + ny * world_up[1] + nz * world_up[2];
        let tx = world_up[0] - nx * dot;
        let ty = world_up[1] - ny * dot;
        let tz = world_up[2] - nz * dot;
        let tangent = Vector3::new(tx, ty, tz);

        let position = Vector3::new(px, py, pz);
        let uv = Vector2::new(u, v);
        let normal = Vector3::new(nx, ny, nz);

        Vertex3D {
            position,
            uv,
            normal,
            tangent,
            bone_indices: [0xFF, 0xFF, 0xFF, 0xFF],
            bone_weights: [0.0, 0.0, 0.0, 0.0],
        }
    }
}

/// A fully featured vertex used for 3D rendering.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex3D {
    pub position: Vector3<f32>,
    pub uv: Vector2<f32>,
    pub normal: Vector3<f32>,
    pub tangent: Vector3<f32>,
    pub bone_indices: [u32; 4],
    pub bone_weights: [f32; 4],
}

impl Vertex3D {
    /// Creates a new vertex from individual attributes.
    pub fn new(
        position: Vector3<f32>,
        tex_coord: Vector2<f32>,
        normal: Vector3<f32>,
        tangent: Vector3<f32>,
        bone_indices: &[u32],
        bone_weights: &[f32],
    ) -> Self {
        Vertex3D {
            position,
            uv: tex_coord,
            normal,
            tangent,
            bone_indices: pad_to_four(bone_indices, 0x0),
            bone_weights: pad_to_four(bone_weights, 0.0),
        }
    }

    /// Returns a [`wgpu::VertexBufferLayout`] describing the layout of this vertex.
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
                    format: VertexFormat::Uint32x4,
                    offset: (VEC3_SIZE * 3 + VEC2_SIZE) as BufferAddress,
                    shader_location: 4,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: (VEC4_SIZE + VEC3_SIZE * 3 + VEC2_SIZE) as BufferAddress,
                    shader_location: 5,
                },
            ],
        };

        const_assert_eq!(size_of::<Vertex3D>(), vertex_layout_size(&LAYOUT));

        LAYOUT
    }

    pub const fn basic(pos: Vector3<f32>) -> Self {
        Vertex3D {
            position: pos,
            uv: Vector2::new(0.0, 0.0),
            normal: Vector3::new(0.0, 1.0, 0.0),
            tangent: Vector3::new(1.0, 0.0, 0.0),
            bone_indices: [0; 4],
            bone_weights: [0.0; 4],
        }
    }
}

pub type Vertex3DTuple<'a, IU, IF> = (
    Vector3<f32>,
    Vector2<f32>,
    Vector3<f32>,
    Vector3<f32>,
    Vector3<f32>,
    IU,
    IF,
);

impl<'a, IU: AsRef<[u32]>, IF: AsRef<[f32]>> From<Vertex3DTuple<'a, IU, IF>> for Vertex3D {
    fn from(value: Vertex3DTuple<IU, IF>) -> Self {
        Vertex3D::new(
            value.0,
            value.1,
            value.2,
            value.3,
            value.5.as_ref(),
            value.6.as_ref(),
        )
    }
}

/// Pads a slice to four elements using the provided default value.
fn pad_to_four<T: Copy>(input: &[T], default: T) -> [T; 4] {
    let mut arr = [default; 4];
    let count = input.len().min(4);
    arr[..count].copy_from_slice(&input[..count]);
    arr
}
