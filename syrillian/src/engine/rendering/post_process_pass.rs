use crate::engine::rendering::uniform::ShaderUniform;
use syrillian_macros::UniformIndex;
use wgpu::{AddressMode, BindGroupLayout, Device, FilterMode, SamplerDescriptor, TextureView};

#[repr(u8)]
#[derive(Debug, Copy, Clone, UniformIndex)]
pub enum PostProcessUniformIndex {
    View = 0,
    Sampler = 1,
}

pub struct PostProcessData {
    pub(crate) uniform: ShaderUniform<PostProcessUniformIndex>,
}

impl PostProcessData {
    pub(crate) fn new(
        device: &Device,
        layout: &BindGroupLayout,
        offscreen_view: &TextureView,
    ) -> Self {
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("PostProcess Sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..SamplerDescriptor::default()
        });

        let uniform = ShaderUniform::<PostProcessUniformIndex>::builder(layout)
            .with_texture(&offscreen_view)
            .with_sampler(&sampler)
            .build(device);

        Self { uniform }
    }
}
