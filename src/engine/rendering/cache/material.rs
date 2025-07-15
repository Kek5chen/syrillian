use crate::engine::assets::{HTexture, Material};
use crate::engine::rendering::cache::{AssetCache, CacheType};
use crate::engine::rendering::uniform::ShaderUniform;
use crate::ensure_aligned;
use nalgebra::Vector3;
use syrillian_macros::UniformIndex;
use wgpu::wgt::SamplerDescriptor;
use wgpu::{AddressMode, Device, Queue};
use crate::assets::HShader;

#[repr(u8)]
#[derive(Debug, Copy, Clone, UniformIndex)]
pub(crate) enum MaterialUniformIndex {
    Material = 0,
    DiffuseView = 1,
    DiffuseSampler = 2,
    NormalView = 3,
    NormalSampler = 4,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MaterialUniform {
    diffuse: Vector3<f32>,
    use_diffuse_texture: u32,
    use_normal_texture: u32,
    shininess: f32,
    opacity: f32,
    _padding: u32,
}

ensure_aligned!(MaterialUniform { diffuse }, align <= 16 * 2 => size);

#[allow(dead_code)]
#[derive(Debug)]
pub struct RuntimeMaterial {
    pub(crate) data: MaterialUniform,
    pub(crate) uniform: ShaderUniform<MaterialUniformIndex>,
    pub(crate) shader: HShader,
}

#[derive(Debug)]
pub enum MaterialError {
    MaterialNotFound,
    DeviceNotInitialized,
    QueueNotInitialized,
}

impl CacheType for Material {
    type Hot = RuntimeMaterial;

    fn upload(&self, device: &Device, _queue: &Queue, cache: &AssetCache) -> Self::Hot {
        let data = MaterialUniform {
            diffuse: self.color,
            _padding: 0xFFFFFFFF,
            use_diffuse_texture: self.diffuse_texture.is_some() as u32,
            use_normal_texture: self.normal_texture.is_some() as u32,
            shininess: self.shininess,
            opacity: self.opacity,
        };

        let mat_bgl = cache.bgl_material();
        let diffuse = cache.texture_opt(self.diffuse_texture, HTexture::FALLBACK_DIFFUSE);
        let normal = cache.texture_opt(self.normal_texture, HTexture::FALLBACK_NORMAL);

        // TODO: Add shininess
        let _shininess = cache.texture_opt(self.shininess_texture, HTexture::FALLBACK_SHININESS);

        // TODO: Add additional material mapping properties and such
        let sampler = device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            ..Default::default()
        });

        let uniform = ShaderUniform::<MaterialUniformIndex>::builder(&mat_bgl)
            .with_buffer_data(&data)
            .with_texture(&diffuse)
            .with_sampler(&sampler)
            .with_texture(&normal)
            .with_sampler(&sampler)
            .build(device);

        RuntimeMaterial {
            data,
            uniform,
            shader: self.shader,
        }
    }
}
