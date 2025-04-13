use bytemuck::{Pod, Zeroable};
use nalgebra::Vector3;

use crate::object::GameObjectId;

use super::Component;

#[repr(C)]
#[derive(Copy, Clone, Default)]
// Need padding for 16-bytes GPU Uniform alignment
pub(crate) struct ShaderPointlight {
    pub(crate) pos: Vector3<f32>,
    pub(crate) radius: f32,
    pub(crate) intensity: f32,
    pub(crate) _pad1: [u32; 3],
    pub(crate) color: Vector3<f32>,
    pub(crate) _pad2: u32,
}

unsafe impl Zeroable for ShaderPointlight {}
unsafe impl Pod for ShaderPointlight {}

pub struct PointLightComponent {
    parent: GameObjectId,
    inner: ShaderPointlight,
}

impl Component for PointLightComponent {
    fn new(parent: GameObjectId) -> Self
    where
        Self: Sized {
            PointLightComponent {
                parent,
                inner: ShaderPointlight {
                    pos: parent.transform.position(),
                    radius: 100.0,
                    intensity: 1.0,
                    _pad1: [0, 0, 0],
                    color: Vector3::new(1.0, 1.0, 1.0),
                    _pad2: 0,
                }
            }
    }

    fn get_parent(&self) -> GameObjectId {
        self.parent
    }
}

impl PointLightComponent {
    pub fn radius(&self) -> f32 {
        self.inner.radius
    }

    pub fn intensity(&self) -> f32 {
        self.inner.intensity
    }

    pub fn color(&self) -> &Vector3<f32> {
        &self.inner.color
    }

    pub fn set_radius(&mut self, radius: f32) {
        let radius = radius.max(0.0);
        self.inner.radius = radius;
    }

    pub fn set_intensity(&mut self, intensity: f32) {
        let intensity = intensity.max(0.0);
        self.inner.intensity = intensity;
    }

    pub fn set_color(&mut self, color: Vector3<f32>) {
        self.inner.color = color;
    }

    pub(crate) fn update_inner_pos(&mut self) {
        self.inner.pos = self.parent.transform.position();
    }

    pub(crate) fn inner(&self) -> &ShaderPointlight {
        &self.inner
    }
}
