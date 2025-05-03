use aligned::{Aligned, A16};
use bytemuck::{Pod, Zeroable};
use nalgebra::Vector3;
use static_assertions::const_assert_eq;

use crate::object::GameObjectId;

use super::Component;

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub(crate) struct ShaderPointLight {
    pub(crate) pos: Aligned<A16, Vector3<f32>>,
    pub(crate) color: Vector3<f32>,
    pub(crate) radius: f32,
    pub(crate) intensity: f32, 
}

const_assert_eq!(size_of::<ShaderPointLight>(), 48);

unsafe impl Zeroable for ShaderPointLight {}
unsafe impl Pod for ShaderPointLight {}

pub struct PointLightComponent {
    parent: GameObjectId,
    inner: ShaderPointLight,
}

impl Component for PointLightComponent {
    fn new(parent: GameObjectId) -> Self
    where
        Self: Sized {
            PointLightComponent {
                parent,
                inner: ShaderPointLight {
                    pos: Aligned(parent.transform.position()),
                    color: Vector3::new(1.0, 1.0, 1.0),
                    radius: 100.0,
                    intensity: 1.0,
                }
            }
    }

    fn update(&mut self) {
        self.inner.pos = Aligned(self.parent.transform.position());
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

    pub fn set_color_rgb(&mut self, color: Vector3<f32>) {
        self.inner.color = color;
    }

    pub(crate) fn update_inner_pos(&mut self) {
        self.inner.pos = Aligned(self.parent.transform.position());
    }

    pub(crate) fn inner(&self) -> &ShaderPointLight {
        &self.inner
    }
}
