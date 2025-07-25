use crate::engine::assets::generic_store::{HandleName, Store, StoreDefaults, StoreType};
use crate::engine::assets::{HTexture, StoreTypeFallback, H};
use crate::store_add_checked;
use std::error::Error;
use std::fs;
use wgpu::{Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub data: Option<Vec<u8>>,
}

impl H<Texture> {
    const FALLBACK_DIFFUSE_ID: u32 = 0;
    const FALLBACK_NORMAL_ID: u32 = 1;
    const FALLBACK_SHININESS_ID: u32 = 2;

    pub const FALLBACK_DIFFUSE: H<Texture> = H::new(Self::FALLBACK_DIFFUSE_ID);
    pub const FALLBACK_NORMAL: H<Texture> = H::new(Self::FALLBACK_NORMAL_ID);
    pub const FALLBACK_SHININESS: H<Texture> = H::new(Self::FALLBACK_SHININESS_ID);
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

    pub(crate) fn desc(&self) -> TextureDescriptor {
        TextureDescriptor {
            label: None,
            size: Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: self.format,
            usage: TextureUsages::TEXTURE_BINDING,
            view_formats: &[TextureFormat::Bgra8UnormSrgb],
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

        let tex = Texture {
            width: rgba.width(),
            height: rgba.height(),
            format: TextureFormat::Bgra8UnormSrgb,
            data: Some(data),
        };

        Ok(tex)
    }
}

impl StoreDefaults for Texture {
    fn populate(store: &mut Store<Self>) {
        const FALLBACK_SIZE: u32 = 35;

        store_add_checked!(
            store,
            HTexture::FALLBACK_DIFFUSE_ID,
            Texture {
                width: FALLBACK_SIZE,
                height: FALLBACK_SIZE,
                format: TextureFormat::Bgra8UnormSrgb,
                data: Some(Self::gen_fallback_diffuse(FALLBACK_SIZE, FALLBACK_SIZE)),
            }
        );

        store_add_checked!(
            store,
            HTexture::FALLBACK_NORMAL_ID,
            Texture {
                width: 1,
                height: 1,
                format: TextureFormat::Bgra8UnormSrgb,
                data: Some(vec![0, 0, 0, 0]),
            }
        );

        store_add_checked!(
            store,
            HTexture::FALLBACK_SHININESS_ID,
            Texture {
                width: 1,
                height: 1,
                format: TextureFormat::Bgra8UnormSrgb,
                data: Some(vec![0, 0, 0, 0]),
            }
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
}

impl StoreTypeFallback for Texture {
    fn fallback() -> H<Self> {
        HTexture::FALLBACK_DIFFUSE
    }
}

impl Store<Texture> {}
