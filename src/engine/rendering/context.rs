use crate::engine::assets::HShader;
use crate::engine::rendering::cache::AssetCache;
use std::sync::{Arc, RwLock};
use wgpu::{RenderPass, SurfaceTexture, TextureView};

pub struct FrameCtx {
    pub output: SurfaceTexture,
    pub color_view: TextureView,
    pub depth_view: TextureView,
    pub cache: Arc<AssetCache>, // TODO: Rethink this every-frame cloned Arc
}

pub struct DrawCtx<'a> {
    pub frame: &'a FrameCtx,

    pub pass: RwLock<RenderPass<'a>>,
    pub shader_override: Option<HShader>,
}
