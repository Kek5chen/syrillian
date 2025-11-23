use crate::components::TypedComponentId;
use crate::game_thread::RenderTargetId;
use crate::rendering::lights::LightProxy;
use crate::rendering::proxies::SceneProxy;
use crate::rendering::render_data::CameraUniform;
use nalgebra::Affine3;
use std::fmt::{Debug, Formatter};

pub type ProxyUpdateCommand = Box<dyn FnOnce(&mut dyn SceneProxy) + Send>;
pub type LightProxyCommand = Box<dyn FnOnce(&mut LightProxy) + Send>;
pub type CameraUpdateCommand = Box<dyn FnOnce(&mut CameraUniform) + Send>;

pub enum RenderMsg {
    RegisterProxy(TypedComponentId, Box<dyn SceneProxy>, Affine3<f32>),
    RegisterLightProxy(TypedComponentId, Box<LightProxy>),
    RemoveProxy(TypedComponentId),
    UpdateTransform(TypedComponentId, Affine3<f32>),
    ProxyUpdate(TypedComponentId, ProxyUpdateCommand),
    LightProxyUpdate(TypedComponentId, LightProxyCommand),
    UpdateActiveCamera(RenderTargetId, CameraUpdateCommand),
    ProxyState(TypedComponentId, bool), // enabled
    CommandBatch(Vec<RenderMsg>),
}

impl Debug for RenderMsg {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            RenderMsg::RegisterProxy(..) => "Register Proxy",
            RenderMsg::RegisterLightProxy(..) => "Register Light Proxy",
            RenderMsg::RemoveProxy(_) => "Remove Proxy",
            RenderMsg::UpdateTransform(..) => "Update Transform",
            RenderMsg::ProxyUpdate(..) => "Proxy Update",
            RenderMsg::LightProxyUpdate(..) => "Light Proxy Update",
            RenderMsg::UpdateActiveCamera(..) => "Update Active Camera",
            RenderMsg::ProxyState(_, enable) => &format!("Proxy Enabled: {enable}"),
            RenderMsg::CommandBatch(inner) => &format!("Command Batch {inner:?}"),
        };

        write!(f, "{name}")
    }
}
