use bytemuck::{Pod, Zeroable};
use nalgebra::Vector3;

use crate::object::GameObjectId;

use super::Component;

#[repr(C)]
#[derive(Copy, Clone)]
pub(crate) struct ShaderPointlight {
    pub(crate) pos: Vector3<f32>,
    pub(crate) radius: f32,
    pub(crate) intensity: f32,
    pub(crate) color: Vector3<f32>,
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
                    radius: 1.0,
                    intensity: 1.0,
                    color: Vector3::new(255.0, 255.0, 255.0),
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

    pub fn set_radius(&mut self, radius: f32) {
        self.inner.radius = radius;
    }

    pub fn set_intensity(&mut self, intensity: f32) {
        self.inner.intensity = intensity;
    }

    pub(crate) fn update_inner_pos(&mut self) {
        self.inner.pos = self.parent.transform.position();
    }

    pub(crate) fn inner(&self) -> &ShaderPointlight {
        &self.inner
    }
}
