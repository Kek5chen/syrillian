use crate::assets::{render_font_atlas, Font, HMaterial, HTexture, Material, Texture};
use crate::rendering::{AssetCache, CacheType};
use crate::World;
use wgpu::{Device, Queue};

pub struct FontAtlas {
    texture: HTexture,
    material: HMaterial,
}

impl CacheType for Font {
    type Hot = FontAtlas;

    fn upload(&self, _device: &Device, _queue: &Queue, _cache: &AssetCache) -> Self::Hot {
        let canvas = self
            .pregenerated_atlas
            .lock()
            .unwrap()
            .take()
            .unwrap_or_else(|| render_font_atlas(&self.inner, self.atlas_glyph_size));

        let texture = Texture::load_pixels(
            canvas.pixels,
            canvas.size.x() as u32,
            canvas.size.y() as u32,
            wgpu::TextureFormat::Bgra8UnormSrgb,
        );

        // FIXME: Somehow get a world into upload
        let world = World::instance();
        let texture = world.assets.textures.add(texture);

        let material = Material::builder()
            .name("Font Atlas".to_string())
            .diffuse_texture(texture)
            .build();

        let material = world.assets.materials.add(material);

        FontAtlas {
            texture,
            material,
        }
    }
}

impl FontAtlas {
    pub const fn texture(&self) -> HTexture {
        self.texture
    }

    pub const fn atlas(&self) -> HMaterial {
        self.material
    }
}
