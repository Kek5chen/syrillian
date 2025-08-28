use crate::components::TypedComponentId;
use crate::rendering::{GPUDrawCtx, Renderer};
use nalgebra::{Affine3, Matrix4};
use std::any::Any;
use winit::window::Window;

pub mod image;
pub mod mesh_proxy;
pub mod text_proxy;

#[cfg(debug_assertions)]
pub mod debug_proxy;

pub use image::*;
pub use mesh_proxy::*;
pub use text_proxy::*;

use crate::assets::AssetStore;
#[cfg(debug_assertions)]
pub use debug_proxy::*;

#[macro_export]
macro_rules! proxy_data_mut {
    ($data:expr) => {
        if let Some(data) = ($data as &mut dyn std::any::Any).downcast_mut() {
            data
        } else {
            ::syrillian_utils::debug_panic!(
                "Could not downcast proxy data. The GPU data type did not match up."
            );
            return;
        }
    };
}

#[macro_export]
macro_rules! proxy_data {
    ($data:expr) => {
        if let Some(data) = ($data as &dyn Any).downcast_ref() {
            data
        } else {
            ::syrillian_utils::debug_panic!(
                "Could not downcast proxy data. The GPU data type did not match up."
            );
            return;
        }
    };
}

pub const PROXY_PRIORITY_SOLID: u32 = 99;
pub const PROXY_PRIORITY_TRANSPARENT: u32 = 999;
pub const PROXY_PRIORITY_2D: u32 = 9999;

pub trait SceneProxy: Send + Any {
    fn setup_render(&mut self, renderer: &Renderer, local_to_world: &Matrix4<f32>) -> Box<dyn Any>;
    fn update_render(
        &mut self,
        renderer: &Renderer,
        data: &mut dyn Any,
        window: &Window,
        local_to_world: &Matrix4<f32>,
    );
    fn render(
        &self,
        renderer: &Renderer,
        data: &dyn Any,
        ctx: &GPUDrawCtx,
        local_to_world: &Matrix4<f32>,
    );
    fn priority(&self, store: &AssetStore) -> u32;
}

pub struct SceneProxyBinding {
    pub component_id: TypedComponentId,
    pub local_to_world: Affine3<f32>,
    pub proxy_data: Box<dyn Any>,
    pub proxy: Box<dyn SceneProxy>,
    pub enabled: bool,
}

impl SceneProxyBinding {
    pub fn new(
        component_id: TypedComponentId,
        local_to_world: Affine3<f32>,
        proxy_data: Box<dyn Any>,
        proxy: Box<dyn SceneProxy>,
    ) -> Self {
        Self {
            component_id,
            local_to_world,
            proxy_data,
            proxy,
            enabled: true,
        }
    }

    pub fn update_transform(&mut self, local_to_world: Affine3<f32>) {
        self.local_to_world = local_to_world;
    }

    pub fn update(&mut self, renderer: &Renderer, window: &Window) {
        self.proxy.update_render(
            renderer,
            self.proxy_data.as_mut(),
            window,
            self.local_to_world.matrix(),
        );
    }

    pub fn render(&self, renderer: &Renderer, ctx: &GPUDrawCtx) {
        self.proxy.render(
            renderer,
            self.proxy_data.as_ref(),
            ctx,
            self.local_to_world.matrix(),
        )
    }
}
