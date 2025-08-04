use crate::components::Component;
use crate::core::GameObjectId;
use crate::ensure_aligned;
use nalgebra::Vector3;

#[repr(C)]
#[derive(Copy, Clone, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PointLightUniform {
    pub pos: Vector3<f32>,
    pub radius: f32,
    pub color: Vector3<f32>,
    pub intensity: f32,
    pub specular_color: Vector3<f32>,
    pub specular_intensity: f32,
}

impl PointLightUniform {
    pub const fn zero() -> Self {
        PointLightUniform {
            pos: Vector3::new(0.0, 0.0, 0.0),
            radius: 0.0,
            color: Vector3::new(0.0, 0.0, 0.0),
            intensity: 0.0,
            specular_color: Vector3::new(0.0, 0.0, 0.0),
            specular_intensity: 0.0,
        }
    }
}

ensure_aligned!(PointLightUniform { pos, color, specular_color }, align <= 16 * 3 => size);

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
                specular_color: Vector3::new(1.0, 1.0, 1.0),
                specular_intensity: 1.0,
            },
        }
    }

    fn update(&mut self) {
        self.inner.pos = self.parent.transform.position();
    }

    fn parent(&self) -> GameObjectId {
        self.parent
    }
}

impl PointLightComponent {
    #[inline]
    pub fn radius(&self) -> f32 {
        self.inner.radius
    }

    #[inline]
    pub fn intensity(&self) -> f32 {
        self.inner.intensity
    }

    #[inline]
    pub fn color(&self) -> &Vector3<f32> {
        &self.inner.color
    }

    #[inline]
    pub fn set_radius(&mut self, radius: f32) {
        let radius = radius.max(0.0);
        self.inner.radius = radius;
    }

    #[inline]
    pub fn set_intensity(&mut self, intensity: f32) {
        let intensity = intensity.max(0.0);
        self.inner.intensity = intensity;
    }

    #[inline]
    pub fn set_color_rgb(&mut self, r: f32, g: f32, b: f32) {
        self.inner.color.x = r;
        self.inner.color.y = g;
        self.inner.color.z = b;
    }

    #[inline]
    pub fn set_color_rgb_vec(&mut self, color: Vector3<f32>) {
        self.inner.color = color;
    }

    pub(crate) fn update_inner_pos(&mut self) {
        self.inner.pos = self.parent.transform.position();
    }

    #[inline]
    pub(crate) fn inner(&self) -> &PointLightUniform {
        &self.inner
    }
}
