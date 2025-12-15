use crate::engine::assets::Texture as CpuTexture;
use crate::engine::rendering::cache::{AssetCache, CacheType};
use wgpu::util::{DeviceExt, TextureDataOrder};
use wgpu::{Device, Extent3d, Queue, Sampler, Texture as WgpuTexture, TextureFormat, TextureView};

#[derive(Debug)]
pub struct GpuTexture {
    pub texture: WgpuTexture,
    pub view: TextureView,
    pub sampler: Sampler,
    pub size: Extent3d,
    pub format: TextureFormat,
}

impl CacheType for CpuTexture {
    type Hot = GpuTexture;

    fn upload(self, device: &Device, queue: &Queue, _cache: &AssetCache) -> Self::Hot {
        let desc = self.desc();
        let size = desc.size;
        let format = desc.format;

        let texture = match &self.data {
            None => device.create_texture(&self.desc()),
            Some(data) => {
                device.create_texture_with_data(queue, &desc, TextureDataOrder::LayerMajor, data)
            }
        };

        let view = texture.create_view(&self.view_desc());
        let sampler = device.create_sampler(&self.sampler_desc());

        GpuTexture {
            texture,
            view,
            sampler,
            size,
            format,
        }
    }
}
