use crate::components::{Component, NewComponent};
use crate::core::GameObjectId;
use crate::engine::assets::HMaterial;
use crate::engine::rendering::CPUDrawCtx;
use crate::game_thread::RenderTargetId;
use crate::rendering::proxies::SceneProxy;
use crate::rendering::proxies::image::ImageSceneProxy;
use crate::{World, proxy_data_mut};
use nalgebra::Matrix4;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImageScalingMode {
    Absolute {
        left: u32,
        right: u32,
        top: u32,
        bottom: u32,
    },
    Relative {
        width: u32,
        height: u32,
        left: u32,
        right: u32,
        top: u32,
        bottom: u32,
    },
    RelativeStretch {
        left: f32,
        right: f32,
        top: f32,
        bottom: f32,
    },
    Ndc {
        center: [f32; 2],
        size: [f32; 2],
    },
}

#[derive(Debug)]
pub struct Image {
    material: HMaterial,
    scaling: ImageScalingMode,
    translation: Matrix4<f32>,
    dirty: bool,
    draw_order: u32,
    order_dirty: bool,
    pub parent: GameObjectId,
    render_target: RenderTargetId,
}

impl Image {
    pub fn scaling_mode(&self) -> ImageScalingMode {
        self.scaling
    }

    pub fn set_scaling_mode(&mut self, scaling: ImageScalingMode) {
        self.scaling = scaling;
        self.dirty = true;
    }

    pub fn material(&self) -> HMaterial {
        self.material
    }

    pub fn set_material(&mut self, material: HMaterial) {
        self.material = material;
        self.dirty = true;
    }

    pub fn set_render_target(&mut self, target: RenderTargetId) {
        self.render_target = target;
        self.dirty = true;
    }

    pub fn set_ndc_layout(&mut self, center: [f32; 2], size: [f32; 2]) {
        self.scaling = ImageScalingMode::Ndc { center, size };
        self.dirty = true;
    }

    pub fn set_translation(&mut self, translation: Matrix4<f32>) {
        self.translation = translation;
        self.dirty = true;
    }

    pub fn set_draw_order(&mut self, order: u32) {
        if self.draw_order == order {
            return;
        }
        self.draw_order = order;
        self.order_dirty = true;
    }
}

impl NewComponent for Image {
    fn new(parent: GameObjectId) -> Self {
        Image {
            parent,

            material: HMaterial::FALLBACK,
            scaling: ImageScalingMode::Absolute {
                left: 0,
                right: 100,
                top: 0,
                bottom: 100,
            },
            translation: Matrix4::identity(),
            dirty: false,
            draw_order: 0,
            order_dirty: false,
            render_target: RenderTargetId::PRIMARY,
        }
    }
}

impl Component for Image {
    fn create_render_proxy(&mut self, _world: &World) -> Option<Box<dyn SceneProxy>> {
        Some(Box::new(ImageSceneProxy {
            material: self.material,
            scaling: self.scaling,
            translation: self.translation,
            dirty: false,
            draw_order: self.draw_order,
            render_target: self.render_target,
        }))
    }

    fn update_proxy(&mut self, _world: &World, mut ctx: CPUDrawCtx) {
        if self.order_dirty {
            let order = self.draw_order;
            ctx.send_proxy_update(move |proxy| {
                let proxy: &mut ImageSceneProxy = proxy_data_mut!(proxy);

                proxy.draw_order = order;
                proxy.dirty = true;
            });
            self.order_dirty = false;
        }

        if self.dirty {
            let scaling = self.scaling;
            let material = self.material;
            let translation = self.translation;
            let target = self.render_target;
            ctx.send_proxy_update(move |proxy| {
                let proxy: &mut ImageSceneProxy = proxy_data_mut!(proxy);

                proxy.scaling = scaling;
                proxy.material = material;
                proxy.translation = translation;
                proxy.render_target = target;
                proxy.dirty = true;
            });
            self.dirty = false;
        }
    }
}
