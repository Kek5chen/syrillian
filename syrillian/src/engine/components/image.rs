use crate::components::Component;
use crate::core::GameObjectId;
use crate::engine::assets::HMaterial;
use crate::engine::rendering::CPUDrawCtx;
use crate::rendering::proxies::image::ImageSceneProxy;
use crate::rendering::proxies::SceneProxy;
use crate::{proxy_data_mut, World};
use nalgebra::Matrix4;

#[derive(Debug, Clone, Copy)]
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
}

#[derive(Debug)]
pub struct Image {
    material: HMaterial,
    scaling: ImageScalingMode,
    translation: Matrix4<f32>,
    dirty: bool,
    pub parent: GameObjectId,
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
}

impl Component for Image {
    fn new(parent: GameObjectId) -> Self
    where
        Self: Sized,
    {
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
        }
    }

    fn create_render_proxy(&mut self, _world: &World) -> Option<Box<dyn SceneProxy>> {
        Some(Box::new(ImageSceneProxy {
            material: self.material,
            scaling: self.scaling,
            translation: self.translation,
            dirty: false,
        }))
    }

    fn update_proxy(&mut self, _world: &World, ctx: CPUDrawCtx) {
        if !self.dirty {
            return;
        }

        let scaling = self.scaling;
        let material = self.material;
        ctx.send_proxy_update(move |proxy| {
            let proxy: &mut ImageSceneProxy = proxy_data_mut!(proxy);

            proxy.scaling = scaling;
            proxy.material = material;
            proxy.dirty = true;
        });
        self.dirty = false;
    }

    fn parent(&self) -> GameObjectId {
        self.parent
    }
}
