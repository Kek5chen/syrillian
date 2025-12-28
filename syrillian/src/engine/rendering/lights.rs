use crate::components::Component;
use crate::ensure_aligned;
use crate::utils::MATRIX4_ID;
use nalgebra::{Matrix4, SimdPartialOrd, Vector3};
use num_enum::TryFromPrimitive;
use syrillian_macros::UniformIndex;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightProxy {
    pub position: Vector3<f32>,
    pub _p0: u32,
    pub up: Vector3<f32>,
    pub radius: f32,
    pub direction: Vector3<f32>,
    pub range: f32,
    pub color: Vector3<f32>,
    pub intensity: f32,
    pub inner_angle: f32,
    pub outer_angle: f32,
    pub type_id: u32, // LightType
    pub shadow_map_id: u32,
    pub view_mat: Matrix4<f32>,
}

impl LightProxy {
    pub const fn dummy() -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            _p0: 0,
            up: Vector3::new(0.0, 0.0, 0.0),
            radius: 10.0,
            direction: Vector3::new(0.0, -1.0, 0.0),
            range: 10.0,
            color: Vector3::new(1.0, 1.0, 1.0),
            intensity: 1000.0,
            inner_angle: 0.0,
            outer_angle: 0.0,
            type_id: LightType::Point as u32,
            shadow_map_id: 0,
            view_mat: MATRIX4_ID,
        }
    }
}

ensure_aligned!(LightProxy { position, up, direction, color, view_mat }, align <= 16 * 9 => size);

pub trait Light: Component {
    fn light_type(&self) -> LightType;

    fn data(&self) -> &LightProxy;
    fn data_mut(&mut self, mark_dirty: bool) -> &mut LightProxy;

    fn mark_dirty(&mut self);
    fn is_dirty(&self) -> bool;

    fn set_range(&mut self, range: f32) {
        self.data_mut(true).range = range.max(0.);
    }

    fn set_intensity(&mut self, intensity: f32) {
        self.data_mut(true).intensity = intensity.max(0.);
    }

    fn set_color(&mut self, r: f32, g: f32, b: f32) {
        let light = self.data_mut(true);

        light.color.x = r.clamp(0., 1.);
        light.color.y = g.clamp(0., 1.);
        light.color.z = b.clamp(0., 1.);
    }

    fn set_color_vec(&mut self, color: &Vector3<f32>) {
        self.data_mut(true).color =
            color.simd_clamp(Vector3::new(0., 0., 0.), Vector3::new(1., 1., 1.));
    }

    fn set_inner_angle(&mut self, angle: f32) {
        let rad = angle.clamp(f32::EPSILON, 45. - f32::EPSILON).to_radians();
        self.data_mut(true).inner_angle = rad;
    }

    fn set_outer_angle(&mut self, angle: f32) {
        let rad = angle.clamp(f32::EPSILON, 45. - f32::EPSILON).to_radians();
        self.data_mut(true).outer_angle = rad;
    }
}

#[repr(u32)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, TryFromPrimitive)]
pub enum LightType {
    Point = 0,
    Sun = 1,
    Spot = 2,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, UniformIndex)]
pub enum LightUniformIndex {
    Count = 0,
    Lights = 1,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, UniformIndex)]
pub enum ShadowUniformIndex {
    ShadowMaps = 0,
    ShadowSampler = 1,
}
