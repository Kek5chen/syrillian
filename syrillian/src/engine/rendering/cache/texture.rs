use crate::engine::assets::Texture;
use crate::engine::rendering::cache::{AssetCache, CacheType};
use wgpu::util::{DeviceExt, TextureDataOrder};
use wgpu::{
    Device, Queue, TextureAspect, TextureFormat, TextureViewDescriptor, TextureViewDimension,
};

impl CacheType for Texture {
    type Hot = wgpu::TextureView;

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

        texture.create_view(&TextureViewDescriptor {
            label: Some("Texture View"),
            format: Some(TextureFormat::Bgra8UnormSrgb),
            dimension: Some(TextureViewDimension::D2),
            aspect: TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
            usage: None,
        })
    }
}
