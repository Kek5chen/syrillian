use crate::components::TypedComponentId;
use crate::rendering::lights::LightProxy;
use crate::rendering::proxies::SceneProxy;
use crate::rendering::render_data::CameraUniform;
use nalgebra::Affine3;

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
    CommandBatch(Vec<RenderMsg>),
}
