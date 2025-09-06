use crate::engine::assets::Texture as CpuTexture;
use crate::engine::rendering::cache::{AssetCache, CacheType};
use wgpu::util::{DeviceExt, TextureDataOrder};
use wgpu::{Device, Queue, Texture as WgpuTexture, TextureView};

#[derive(Debug)]
pub struct GpuTexture {
    pub texture: WgpuTexture,
    pub view: TextureView,
}

impl CacheType for CpuTexture {
    type Hot = GpuTexture;

    fn upload(self, device: &Device, queue: &Queue, _cache: &AssetCache) -> Self::Hot {
        let texture = match &self.data {
            None => device.create_texture(&self.desc()),
            Some(data) => device.create_texture_with_data(
                queue,
                &self.desc(),
                TextureDataOrder::LayerMajor,
                data,
            ),
        };

        let view = texture.create_view(&self.view_desc());

        GpuTexture { texture, view }
    }
}
