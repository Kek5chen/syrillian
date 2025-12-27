use crate::engine::assets::generic_store::{HandleName, Store, StoreDefaults, StoreType};
use crate::engine::assets::{H, HTexture, StoreTypeFallback};
use crate::rendering::RenderMsg;
use crate::{World, store_add_checked};
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use wgpu::{
    AddressMode, Extent3d, FilterMode, MipmapFilterMode, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub data: Option<Vec<u8>>,
    pub view_formats: [TextureFormat; 1],
    pub array_layers: u32,
    pub repeat_mode: AddressMode,
    pub filter_mode: FilterMode,
    pub mip_filter_mode: MipmapFilterMode,
    pub has_transparency: bool,
}

impl H<Texture> {
    const FALLBACK_DIFFUSE_ID: u32 = 0;
    const FALLBACK_NORMAL_ID: u32 = 1;
    const FALLBACK_SHININESS_ID: u32 = 2;
    const MAX_BUILTIN_ID: u32 = 2;

    pub const FALLBACK_DIFFUSE: H<Texture> = H::new(Self::FALLBACK_DIFFUSE_ID);
    pub const FALLBACK_NORMAL: H<Texture> = H::new(Self::FALLBACK_NORMAL_ID);
    pub const FALLBACK_ROUGHNESS: H<Texture> = H::new(Self::FALLBACK_SHININESS_ID);

    pub fn export_screenshot(self, path: impl Into<PathBuf>, world: &World) -> bool {
        world
            .channels
            .render_tx
            .send(RenderMsg::CaptureTexture(self, path.into()))
            .is_ok()
    }
}

impl Texture {
    pub fn gen_fallback_diffuse(width: u32, height: u32) -> Vec<u8> {
        let mut diffuse = vec![];
        for x in 0..width as i32 {
            for y in 0..height as i32 {
                if x % 2 == y % 2 {
                    diffuse.extend_from_slice(&[0, 0, 0, 255]);
                } else {
                    diffuse.extend_from_slice(&[255, 0, 255, 255]);
                }
            }
        }
        diffuse
    }

    pub fn new_2d_shadow_map_array(capacity: u32, width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            format: TextureFormat::Depth32Float,
            data: None,
            view_formats: [TextureFormat::Depth32Float],
            array_layers: capacity,
            repeat_mode: AddressMode::Repeat,
            filter_mode: FilterMode::Linear,
            mip_filter_mode: MipmapFilterMode::Linear,
            has_transparency: false,
        }
    }

    pub(crate) fn desc(&self) -> TextureDescriptor<'_> {
        let layers = self.array_layers.max(1);
        let usage = TextureUsages::TEXTURE_BINDING
            | TextureUsages::RENDER_ATTACHMENT
            | TextureUsages::COPY_SRC
            | TextureUsages::COPY_DST;

        TextureDescriptor {
            label: None,
            size: Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: layers,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: self.format,
            usage,
            view_formats: &self.view_formats,
        }
    }

    pub fn view_desc(&self) -> wgpu::TextureViewDescriptor<'_> {
        use wgpu::{TextureAspect, TextureViewDimension};
        let dimension = if self.array_layers > 1 {
            TextureViewDimension::D2Array
        } else {
            TextureViewDimension::D2
        };

        wgpu::TextureViewDescriptor {
            label: Some("Texture View"),
            format: Some(self.format),
            dimension: Some(dimension),
            aspect: TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
            usage: None,
        }
    }

    pub fn sampler_desc(&self) -> wgpu::SamplerDescriptor<'_> {
        wgpu::SamplerDescriptor {
            address_mode_u: self.repeat_mode,
            address_mode_v: self.repeat_mode,
            address_mode_w: self.repeat_mode,
            mag_filter: self.filter_mode,
            min_filter: self.filter_mode,
            mipmap_filter: self.mip_filter_mode,
            ..Default::default()
        }
    }

    pub fn load_image(path: &str) -> Result<Texture, Box<dyn Error>> {
        let bytes = fs::read(path)?;
        Self::load_image_from_memory(&bytes)
    }

    pub fn load_image_from_memory(bytes: &[u8]) -> Result<Texture, Box<dyn Error>> {
        let image = image::load_from_memory(bytes)?;
        let rgba = image.into_rgba8();

        let mut data = Vec::with_capacity((rgba.width() * rgba.height() * 4) as usize);
        for pixel in rgba.pixels() {
            data.push(pixel[2]); // B
            data.push(pixel[1]); // G
            data.push(pixel[0]); // R
            data.push(pixel[3]); // A
        }

        Ok(Self::load_pixels(
            data,
            rgba.width(),
            rgba.height(),
            TextureFormat::Bgra8UnormSrgb,
        ))
    }

    pub fn load_pixels(pixels: Vec<u8>, width: u32, height: u32, format: TextureFormat) -> Texture {
        let has_transparency = Self::calculate_transparency(format, &pixels);
        Texture {
            width,
            height,
            format,
            data: Some(pixels),
            view_formats: [format],
            array_layers: 1,
            repeat_mode: AddressMode::Repeat,
            filter_mode: FilterMode::Linear,
            mip_filter_mode: MipmapFilterMode::Linear,
            has_transparency,
        }
    }

    pub fn load_pixels_with_transparency(
        pixels: Vec<u8>,
        width: u32,
        height: u32,
        format: TextureFormat,
        has_transparency: bool,
    ) -> Texture {
        Texture {
            width,
            height,
            format,
            data: Some(pixels),
            view_formats: [format],
            array_layers: 1,
            repeat_mode: AddressMode::Repeat,
            filter_mode: FilterMode::Linear,
            mip_filter_mode: MipmapFilterMode::Linear,
            has_transparency,
        }
    }

    fn calculate_transparency(format: TextureFormat, data: &[u8]) -> bool {
        let chunk_size = match format {
            TextureFormat::Rg8Unorm => 2,
            TextureFormat::Rgba8Unorm
            | TextureFormat::Rgba8UnormSrgb
            | TextureFormat::Rgba8Snorm
            | TextureFormat::Rgba8Uint
            | TextureFormat::Rgba8Sint
            | TextureFormat::Bgra8Unorm
            | TextureFormat::Bgra8UnormSrgb => 4,
            _ => return false,
        };

        for alpha in data.iter().skip(chunk_size - 1).step_by(chunk_size) {
            if *alpha < u8::MAX {
                return true;
            }
        }

        false
    }

    pub fn refresh_transparency(&mut self) {
        if let Some(data) = &self.data {
            self.has_transparency = Self::calculate_transparency(self.view_formats[0], data);
        }
    }
}

impl StoreDefaults for Texture {
    fn populate(store: &mut Store<Self>) {
        const FALLBACK_SIZE: u32 = 35;

        store_add_checked!(
            store,
            HTexture::FALLBACK_DIFFUSE_ID,
            Texture::load_pixels_with_transparency(
                Self::gen_fallback_diffuse(FALLBACK_SIZE, FALLBACK_SIZE),
                FALLBACK_SIZE,
                FALLBACK_SIZE,
                TextureFormat::Bgra8UnormSrgb,
                false,
            )
        );

        store_add_checked!(
            store,
            HTexture::FALLBACK_NORMAL_ID,
            Texture::load_pixels_with_transparency(
                vec![0; 4],
                1,
                1,
                TextureFormat::Bgra8UnormSrgb,
                false
            )
        );

        store_add_checked!(
            store,
            HTexture::FALLBACK_SHININESS_ID,
            Texture::load_pixels_with_transparency(
                vec![0; 4],
                1,
                1,
                TextureFormat::Bgra8UnormSrgb,
                false
            )
        );
    }
}

impl StoreType for Texture {
    #[inline]
    fn name() -> &'static str {
        "Texture"
    }

    fn ident_fmt(handle: H<Self>) -> HandleName<Self> {
        match handle.id() {
            HTexture::FALLBACK_DIFFUSE_ID => HandleName::Static("Diffuse Fallback"),
            HTexture::FALLBACK_NORMAL_ID => HandleName::Static("Normal Fallback"),
            HTexture::FALLBACK_SHININESS_ID => HandleName::Static("Diffuse Fallback"),
            _ => HandleName::Id(handle),
        }
    }

    fn is_builtin(handle: H<Self>) -> bool {
        handle.id() <= H::<Self>::MAX_BUILTIN_ID
    }
}

impl StoreTypeFallback for Texture {
    fn fallback() -> H<Self> {
        HTexture::FALLBACK_DIFFUSE
    }
}

impl Texture {
    pub fn new_2d_shadow_array(capacity: u32, width: u32, height: u32) -> Self {
        Texture {
            width,
            height,
            format: TextureFormat::Depth32Float,
            data: None,
            view_formats: [TextureFormat::Depth32Float],
            array_layers: capacity.max(1),
            repeat_mode: AddressMode::Repeat,
            filter_mode: FilterMode::Linear,
            mip_filter_mode: MipmapFilterMode::Linear,
            has_transparency: false,
        }
    }
}

impl Store<Texture> {}
