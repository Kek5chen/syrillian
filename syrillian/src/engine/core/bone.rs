use crate::ensure_aligned;
use nalgebra::Matrix4;
use std::collections::HashMap;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Bone {
    pub(crate) transform: Matrix4<f32>,
}

ensure_aligned!(Bone { transform }, align <= 16 * 4 => size);

#[derive(Debug, Default, Clone)]
pub struct Bones {
    /// Index-aligned bone names.
    pub names: Vec<String>,
    /// Parent bone index; None for roots.
    pub parents: Vec<Option<usize>>,
    pub children: Vec<Vec<usize>>,
    pub roots: Vec<usize>,
    pub inverse_bind: Vec<Matrix4<f32>>,
    pub bind_global: Vec<Matrix4<f32>>,
    pub bind_local: Vec<Matrix4<f32>>,
    /// Fast lookup from name to index.
    pub index_of: HashMap<String, usize>,
}

impl Bones {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    pub fn index(&self, name: &str) -> Option<usize> {
        self.index_of.get(name).copied()
    }

    pub fn as_slice(&self) -> &[Matrix4<f32>] {
        self.inverse_bind.as_slice()
    }

    pub fn none() -> Bones {
        Bones::default()
    }
}
