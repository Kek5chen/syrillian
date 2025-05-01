use aligned::{Aligned, A16};
use bytemuck::{Pod, Zeroable};
use nalgebra::Vector3;

use crate::object::GameObjectId;

use super::Component;

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub(crate) struct ShaderPointlight {
    pub(crate) pos: Aligned<A16, Vector3<f32>>,
    pub(crate) color: Aligned<A16, Vector3<f32>>,
    pub(crate) radius: f32,
    pub(crate) intensity: f32, 
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
                    pos: Aligned(parent.transform.position()),
                    color: Aligned(Vector3::new(1.0, 1.0, 1.0)),
                    radius: 100.0,
                    intensity: 1.0,
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
        self.inner.color = Aligned(color);
    }

    pub(crate) fn update_inner_pos(&mut self) {
        self.inner.pos = Aligned(self.parent.transform.position());
    }

    pub(crate) fn inner(&self) -> &ShaderPointlight {
        &self.inner
    }
}
