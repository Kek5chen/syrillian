use wgpu::{Device, Extent3d, SurfaceConfiguration, TextureDescriptor, TextureDimension, TextureUsages, TextureView, TextureViewDescriptor};

pub struct OffscreenSurface(TextureView);

impl OffscreenSurface {
    pub fn new(device: &Device, config: &SurfaceConfiguration) -> Self{
        let tex = device
            .create_texture(&TextureDescriptor {
                label: Some("Offscreen Texture"),
                size: Extent3d {
                    width: config.width,
                    height: config.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: config.format,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&TextureViewDescriptor::default());
        
        OffscreenSurface(tex)
    }
    
    pub fn recreate(&mut self, device: &Device, config: &SurfaceConfiguration) {
        *self = Self::new(device, config);
    }
    
    pub fn view(&self) -> &TextureView {
        &self.0
    }
}
