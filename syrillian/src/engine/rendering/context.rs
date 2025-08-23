use crate::components::TypedComponentId;
use crate::rendering::lights::LightProxy;
use crate::rendering::message::RenderMsg;
use crate::rendering::proxies::SceneProxy;
use std::sync::{mpsc, RwLock};
use wgpu::{RenderPass, SurfaceTexture, TextureView};

pub struct FrameCtx {
    pub output: SurfaceTexture,
    pub color_view: TextureView,
    pub depth_view: TextureView,
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum RenderPassType {
    Color,
    Shadow,
}

pub struct GPUDrawCtx<'a> {
    pub pass: RwLock<RenderPass<'a>>,
    pub pass_type: RenderPassType,
    pub frame: &'a FrameCtx,
}

pub struct CPUDrawCtx {
    current_cid: TypedComponentId,
    render_tx: mpsc::Sender<RenderMsg>,
}

impl CPUDrawCtx {
    pub fn new(cid: TypedComponentId, render_tx: mpsc::Sender<RenderMsg>) -> Self {
        Self {
            current_cid: cid,
            render_tx,
        }
    }
    pub fn send_proxy_update(&self, cmd: impl FnOnce(&mut dyn SceneProxy) + Send + 'static) {
        let msg = RenderMsg::ProxyUpdate(self.current_cid, Box::new(cmd));
        self.render_tx.send(msg).unwrap();
    }

    pub fn send_light_proxy_update(&self, cmd: impl FnOnce(&mut LightProxy) + Send + 'static) {
        let msg = RenderMsg::LightProxyUpdate(self.current_cid, Box::new(cmd));
        self.render_tx.send(msg).unwrap();
    }

    pub fn disable_proxy(&self) {
        let msg = RenderMsg::ProxyState(self.current_cid, false);
        self.render_tx.send(msg).unwrap();
    }

    pub fn enable_proxy(&self) {
        let msg = RenderMsg::ProxyState(self.current_cid, true);
        self.render_tx.send(msg).unwrap();
    }
}
