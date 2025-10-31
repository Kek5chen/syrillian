use crate::World;
use crate::components::{Component, NewComponent};
use crate::core::GameObjectId;
use crate::rendering::CPUDrawCtx;
use crate::rendering::lights::{Light, LightProxy, LightType};
use crate::utils::FloatMathExt;
use std::marker::PhantomData;

pub trait LightTypeTrait: Send + Sync {
    fn type_id() -> LightType;
}

pub struct Point;
pub struct Sun;
pub struct Spot;

pub struct LightComponent<L: LightTypeTrait + 'static> {
    parent: GameObjectId,

    target_inner_angle: f32,
    target_outer_angle: f32,
    pub inner_angle_t: f32,
    pub outer_angle_t: f32,
    pub tween_enabled: bool,
    dirty: bool,

    local_proxy: LightProxy,

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

impl<L: LightTypeTrait + 'static> NewComponent for LightComponent<L> {
    fn new(parent: GameObjectId) -> Self {
        const DEFAULT_INNER_ANGLE: f32 = 5.0f32.to_radians();
        const DEFAULT_OUTER_ANGLE: f32 = 30.0f32.to_radians();

        let mut local_proxy = LightProxy::dummy();

        let type_id = L::type_id();
        local_proxy.type_id = type_id as u32;
        if type_id == LightType::Spot {
            local_proxy.inner_angle = DEFAULT_INNER_ANGLE;
            local_proxy.outer_angle = DEFAULT_OUTER_ANGLE;
            local_proxy.range = 100.0;
            local_proxy.intensity = 1000.0;
        }

        local_proxy.position = parent.transform.position();
        local_proxy.direction = parent.transform.forward();
        local_proxy.up = parent.transform.up();
        local_proxy.view_mat = parent.transform.view_matrix_rigid().to_matrix();

        LightComponent {
            parent,

            target_inner_angle: DEFAULT_INNER_ANGLE,
            target_outer_angle: DEFAULT_OUTER_ANGLE,
            inner_angle_t: 1.0,
            outer_angle_t: 1.0,
            tween_enabled: false,

            dirty: true,
            local_proxy,

            light_type: PhantomData,
        }
    }
}

impl<L: LightTypeTrait + 'static> Component for LightComponent<L> {
    fn update(&mut self, world: &mut World) {
        if self.parent.transform.is_dirty() {
            self.local_proxy.position = self.parent.transform.position();
            self.local_proxy.direction = self.parent.transform.forward();
            self.local_proxy.up = self.parent.transform.up();
            self.local_proxy.view_mat = self.parent.transform.view_matrix_rigid().to_matrix();
            self.dirty = true;
        }

        if self.tween_enabled {
            let delta = world.delta_time().as_secs_f32();

            self.local_proxy.outer_angle = self
                .local_proxy
                .outer_angle
                .lerp(self.target_outer_angle, self.outer_angle_t * delta);
            self.local_proxy.inner_angle = self
                .local_proxy
                .inner_angle
                .lerp(self.target_inner_angle, self.inner_angle_t * delta);
            self.dirty = true;
        }
    }

    fn create_light_proxy(&mut self, _world: &World) -> Option<Box<LightProxy>> {
        Some(Box::new(self.local_proxy))
    }

    fn update_proxy(&mut self, _world: &World, mut ctx: CPUDrawCtx) {
        if !self.dirty {
            return;
        }

        let new_proxy = self.local_proxy;
        ctx.send_light_proxy_update(move |proxy| {
            *proxy = new_proxy;
        });

        self.dirty = false;
    }
}

impl<L: LightTypeTrait + 'static> Light for LightComponent<L> {
    #[inline]
    fn light_type(&self) -> LightType {
        L::type_id()
    }

    fn data(&self) -> &LightProxy {
        &self.local_proxy
    }

    fn data_mut(&mut self, mark_dirty: bool) -> &mut LightProxy {
        if mark_dirty {
            self.dirty = true;
        }
        &mut self.local_proxy
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    fn is_dirty(&self) -> bool {
        self.dirty
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
