use crate::components::Component;
use crate::core::GameObjectId;
use crate::ensure_aligned;
use nalgebra::Vector3;

#[repr(C)]
#[derive(Copy, Clone, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct PointLightUniform {
    pub(crate) pos: Vector3<f32>,
    pub(crate) radius: f32,
    pub(crate) color: Vector3<f32>,
    pub(crate) intensity: f32,
}

ensure_aligned!(PointLightUniform { pos, color }, align <= 16 * 2 => size);

pub struct PointLightComponent {
    parent: GameObjectId,
    inner: PointLightUniform,
}

impl Component for PointLightComponent {
    fn new(parent: GameObjectId) -> Self
    where
        Self: Sized,
    {
        PointLightComponent {
            parent,
            inner: PointLightUniform {
                pos: parent.transform.position(),
                radius: 100.0,
                color: Vector3::new(1.0, 1.0, 1.0),
                intensity: 1.0,
            },
        }
    }

    fn update(&mut self) {
        self.inner.pos = self.parent.transform.position();
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
        self.inner.pos = self.parent.transform.position();
    }

    pub(crate) fn inner(&self) -> &PointLightUniform {
        &self.inner
    }
}
