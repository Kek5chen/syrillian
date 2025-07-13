use nalgebra::{Vector2, Vector3, Vector4};
use static_assertions::const_assert_eq;

pub const VEC2_SIZE: u64 = size_of::<Vector2<f32>>() as u64;
pub const VEC3_SIZE: u64 = size_of::<Vector3<f32>>() as u64;
pub const VEC4_SIZE: u64 = size_of::<Vector4<f32>>() as u64;

pub const WGPU_VEC2_ALIGN: u64 = 8;
pub const WGPU_VEC3_ALIGN: u64 = 16;
pub const WGPU_VEC4_ALIGN: u64 = 16;

const_assert_eq!(VEC2_SIZE, WGPU_VEC2_ALIGN);
const_assert_eq!(VEC3_SIZE + 4, WGPU_VEC4_ALIGN);
const_assert_eq!(VEC4_SIZE, WGPU_VEC4_ALIGN);

pub const fn vertex_layout_size(layout: &wgpu::VertexBufferLayout) -> usize {
    let mut sum: u64 = 0;
    let mut i = 0;

    while i < layout.attributes.len() {
        sum += layout.attributes[i].format.size();
        i += 1;
    }

    sum as usize
}
