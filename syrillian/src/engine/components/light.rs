use crate::components::Component;
use crate::core::GameObjectId;
use crate::rendering::lights::{Light, LightHandle, LightType};
use crate::World;
use debug_panic::debug_panic;
use std::marker::PhantomData;

pub trait LightTypeTrait {
    fn type_id() -> LightType;
}

pub struct Point;
pub struct Sun;

pub struct LightComponent<L: LightTypeTrait + 'static> {
    parent: GameObjectId,
    handle: LightHandle,
    light_type: PhantomData<L>,
}

pub type PointLightComponent = LightComponent<Point>;
pub type SunLightComponent = LightComponent<Sun>;

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

impl<L: LightTypeTrait + 'static> Component for LightComponent<L> {
    fn new(parent: GameObjectId) -> Self
    where
        Self: Sized,
    {
        let world = World::instance();
        let handle = world.lights.register();

        if let Some(light) = world.lights.get_mut(handle) {
            light.type_id = L::type_id() as u32;
        } else {
            debug_panic!("Light wasn't created");
        }

        LightComponent {
            parent,
            handle,
            light_type: PhantomData,
        }
    }

    fn update(&mut self, world: &mut World) {
        let Some(data) = world.lights.get_mut(self.handle) else {
            debug_panic!("Light disappeared");
            return;
        };

        data.position = self.parent.transform.position();
        data.direction = self.parent.transform.forward();
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