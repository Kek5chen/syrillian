use crate::components::TypedComponentId;
use crate::rendering::lights::LightProxy;
use crate::rendering::message::RenderMsg;
use crate::rendering::proxies::SceneProxy;
use std::sync::RwLock;
use wgpu::{BindGroup, RenderPass, SurfaceTexture, TextureView};

pub struct FrameCtx {
    pub output: SurfaceTexture,
    pub color_view: TextureView,
    pub depth_view: TextureView,
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum RenderPassType {
    Color,
    Color2D,
    Shadow,
}

pub struct GPUDrawCtx<'a> {
    pub pass: RwLock<RenderPass<'a>>,
    pub pass_type: RenderPassType,
    pub frame: &'a FrameCtx,
    pub render_bind_group: &'a BindGroup,
    pub light_bind_group: &'a BindGroup,
    pub shadow_bind_group: &'a BindGroup,
}

pub struct CPUDrawCtx<'a> {
    current_cid: TypedComponentId,
    batch: &'a mut Vec<RenderMsg>,
}

impl<'a> CPUDrawCtx<'a> {
    pub fn new(cid: TypedComponentId, batch: &'a mut Vec<RenderMsg>) -> Self {
        Self {
            current_cid: cid,
            batch,
        }
    }

    pub fn send_proxy_update(&mut self, cmd: impl FnOnce(&mut dyn SceneProxy) + Send + 'static) {
        let msg = RenderMsg::ProxyUpdate(self.current_cid, Box::new(cmd));
        self.batch.push(msg);
    }

    pub fn send_light_proxy_update(&mut self, cmd: impl FnOnce(&mut LightProxy) + Send + 'static) {
        let msg = RenderMsg::LightProxyUpdate(self.current_cid, Box::new(cmd));
        self.batch.push(msg);
    }

    pub fn disable_proxy(&mut self) {
        let msg = RenderMsg::ProxyState(self.current_cid, false);
        self.batch.push(msg);
    }

    pub fn enable_proxy(&mut self) {
        let msg = RenderMsg::ProxyState(self.current_cid, true);
        self.batch.push(msg);
    }
}
