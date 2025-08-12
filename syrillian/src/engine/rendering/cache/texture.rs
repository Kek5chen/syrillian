use crate::engine::assets::Texture as CpuTexture;
use crate::engine::rendering::cache::{AssetCache, CacheType};
use wgpu::util::{DeviceExt, TextureDataOrder};
use wgpu::{Device, Queue, Texture as WgpuTexture, TextureAspect, TextureView, TextureViewDescriptor, TextureViewDimension};

#[derive(Debug)]
pub struct GpuTexture {
    pub texture: WgpuTexture,
    pub view: TextureView,
}

impl CacheType for CpuTexture {
    type Hot = GpuTexture;

    fn upload(&self, device: &Device, queue: &Queue, _cache: &AssetCache) -> Self::Hot {
        let texture = match &self.data {
            None => device.create_texture(&self.desc()),
            Some(data) => device.create_texture_with_data(
                &queue,
                &self.desc(),
                TextureDataOrder::LayerMajor,
                data,
            ),
        };

        let view = texture.create_view(&TextureViewDescriptor {
            label: Some("Texture View"),
            format: Some(self.format),
            dimension: Some(TextureViewDimension::D2),
            aspect: TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
            usage: None,
        });

        GpuTexture { texture, view }
    }
}
