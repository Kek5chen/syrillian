use crate::components::TypedComponentId;
use crate::rendering::lights::LightProxy;
use crate::rendering::proxies::SceneProxy;
use crate::rendering::render_data::CameraUniform;
use nalgebra::Affine3;
use std::fmt::{Debug, Formatter};
use wgpu::Color;

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
    UpdateActiveCamera(CameraUpdateCommand),
    ProxyState(TypedComponentId, bool), // enabled
    SetSkyboxBackgroundColor(Color),
    CommandBatch(Vec<RenderMsg>),
}

impl Debug for RenderMsg {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            RenderMsg::RegisterProxy(_, _, _) => "Register Proxy",
            RenderMsg::RegisterLightProxy(_, _) => "Register Light Proxy",
            RenderMsg::RemoveProxy(_) => "Remove Proxy",
            RenderMsg::UpdateTransform(_, _) => "Update Transform",
            RenderMsg::ProxyUpdate(_, _) => "Proxy Update",
            RenderMsg::LightProxyUpdate(_, _) => "Light Proxy Update",
            RenderMsg::UpdateActiveCamera(_) => "Update Active Camera",
            RenderMsg::ProxyState(_, enable) => &format!("Proxy Enabled: {enable}"),
            RenderMsg::SetSkyboxBackgroundColor(_) => "Set Skybox Background Color",
            RenderMsg::CommandBatch(inner) => &format!("Command Batch {inner:?}"),
        };

        write!(f, "{name}")
    }
}
