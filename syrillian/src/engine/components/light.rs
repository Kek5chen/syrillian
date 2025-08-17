use crate::components::Component;
use crate::core::GameObjectId;
use crate::rendering::lights::{Light, LightHandle, LightType};
use crate::utils::FloatMathExt;
use crate::World;
use std::marker::PhantomData;
use syrillian_utils::debug_panic;

pub trait LightTypeTrait {
    fn type_id() -> LightType;
}

pub struct Point;
pub struct Sun;
pub struct Spot;

pub struct LightComponent<L: LightTypeTrait + 'static> {
    parent: GameObjectId,
    handle: LightHandle,

    target_inner_angle: f32,
    target_outer_angle: f32,
    pub inner_angle_t: f32,
    pub outer_angle_t: f32,
    pub tween_enabled: bool,

    light_type: PhantomData<L>,
}

pub type PointLightComponent = LightComponent<Point>;
pub type SunLightComponent = LightComponent<Sun>;
pub type SpotLightComponent = LightComponent<Spot>;

impl LightTypeTrait for Sun {
    fn type_id() -> LightType {
        LightType::Sun
    }
}

impl LightTypeTrait for Point {
    fn type_id() -> LightType {
        LightType::Point
    }
}

impl LightTypeTrait for Spot {
    fn type_id() -> LightType {
        LightType::Spot
    }
}

impl<L: LightTypeTrait + 'static> Component for LightComponent<L> {
    fn new(parent: GameObjectId) -> Self
    where
        Self: Sized,
    {
        let world = World::instance();
        let handle = world.lights.register();

        const DEFAULT_INNER_ANGLE: f32 = 5.0f32.to_radians();
        const DEFAULT_OUTER_ANGLE: f32 = 30.0f32.to_radians();

        if let Some(light) = world.lights.get_mut(handle) {
            let type_id = L::type_id();
            light.type_id = type_id as u32;
            if type_id == LightType::Spot {
                light.inner_angle = DEFAULT_INNER_ANGLE;
                light.outer_angle = DEFAULT_OUTER_ANGLE;
                light.range = 100.0;
                light.intensity = 100.0;
            }
        } else {
            debug_panic!("Light wasn't created");
        }

        LightComponent {
            parent,
            handle,
            target_inner_angle: DEFAULT_INNER_ANGLE,
            target_outer_angle: DEFAULT_OUTER_ANGLE,
            inner_angle_t: 1.0,
            outer_angle_t: 1.0,
            tween_enabled: false,
            light_type: PhantomData,
        }
    }

    fn update(&mut self, world: &mut World) {
        let delta = world.delta_time().as_secs_f32();
        let Some(data) = world.lights.get_mut(self.handle) else {
            debug_panic!("Light disappeared");
            return;
        };

        data.position = self.parent.transform.position();
        data.direction = self.parent.transform.forward();
        data.up = self.parent.transform.up();

        if self.tween_enabled {
            data.outer_angle = data.outer_angle.lerp(self.target_outer_angle, self.outer_angle_t * delta);
            data.inner_angle = data.inner_angle.lerp(self.target_inner_angle, self.inner_angle_t * delta);
        }

        data.view_mat = self.parent.transform.view_matrix_rigid().to_matrix();
    }

    #[inline]
    fn parent(&self) -> GameObjectId {
        self.parent
    }
}

impl<L: LightTypeTrait + 'static> Light for LightComponent<L> {
    #[inline]
    fn light_handle(&self) -> LightHandle {
        self.handle
    }

    #[inline]
    fn light_type(&self) -> LightType {
        L::type_id()
    }
}

impl<L: LightTypeTrait + 'static> LightComponent<L> {
    pub fn set_outer_angle_tween_target(&mut self, angle: f32) {
        let rad = angle.clamp(f32::EPSILON, 45. - f32::EPSILON).to_radians();
        self.target_outer_angle = rad;
    }

    pub fn set_inner_angle_tween_target(&mut self, angle: f32) {
        let rad = angle.clamp(f32::EPSILON, 45. - f32::EPSILON).to_radians();
        self.target_inner_angle = rad;
    }
}