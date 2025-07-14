use crate::ensure_aligned;
use nalgebra::Matrix4;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Bone {
    pub(crate) transform: Matrix4<f32>,
}

ensure_aligned!(Bone { transform }, align <= 16 * 4 => size);

impl From<&russimp_ng::bone::Bone> for Bone {
    fn from(value: &russimp_ng::bone::Bone) -> Self {
        let m = value.offset_matrix;
        Bone {
            #[rustfmt::skip]
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

    pub fn as_slice(&self) -> &[Bone] {
        &self.raw
    }

    pub fn none() -> Bones {
        Bones::default()
    }
}
