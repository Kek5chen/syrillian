use nalgebra::{Vector2, Vector3, Vector4};

pub const VEC2_SIZE: usize = size_of::<Vector2<f32>>();
pub const VEC3_SIZE: usize = size_of::<Vector3<f32>>();
pub const VEC4_SIZE: usize = size_of::<Vector4<f32>>();

pub const WGPU_VEC2_SIZE: usize = 8;
pub const WGPU_VEC3_SIZE: usize = 16;
pub const WGPU_VEC4_SIZE: usize = 16;

pub const fn layout_size(layout: &wgpu::VertexBufferLayout) -> usize {
    let mut sum: u64 = 0;
    let mut i = 0;

    while i < layout.attributes.len() {
        sum += layout.attributes[i].format.size();
        i += 1;
    }

    sum as usize
}
