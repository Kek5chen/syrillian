use crate::World;
use crate::assets::{HMaterial, HTexture, Material, Texture};
use crate::rendering::AssetCache;
use crate::rendering::glyph::GlyphBitmap;
use etagere::{AtlasAllocator, size2};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use ttf_parser::Face;
use wgpu::{Extent3d, Origin3d, TexelCopyBufferLayout, TextureAspect, TextureFormat};

#[derive(Clone, Copy, Debug)]
pub struct GlyphAtlasEntry {
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],

    pub plane_min: [f32; 2], // (left_em, bottom_em)
    pub plane_max: [f32; 2], // (right_em, top_em)
    pub advance_em: f32,

    pub msdf_range_px: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct FontLineMetrics {
    pub ascent_em: f32,
    pub descent_em: f32,
    pub line_gap_em: f32,
    pub units_per_em: f32,
}

pub struct MsdfAtlas {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    stride: usize,

    alloc: AtlasAllocator,

    entries: RwLock<HashMap<char, GlyphAtlasEntry>>,
    metrics: FontLineMetrics,

    shrinkage: f64,
    range: f64,

    face_bytes: Arc<Vec<u8>>,

    pub texture: HTexture,
    pub material: HMaterial,
}

impl MsdfAtlas {
    pub fn new(
        face_bytes: Arc<Vec<u8>>,
        atlas_size: u32,
        shrinkage: f64,
        range: f64,
        world: &World,
    ) -> Self {
        let face = Face::parse(&face_bytes, 0).expect("parse face");
        let units_per_em = face.units_per_em() as f32;

        let ascent_em = face.ascender() as f32 / units_per_em;
        let descent_em = (-face.descender()) as f32 / units_per_em;
        let line_gap_em = face.line_gap() as f32 / units_per_em;

        // allocate linear rgba8 atlas (not srgb)
        let width = atlas_size;
        let height = atlas_size;
        let stride = (width as usize) * 4;
        let pixels = vec![0u8; stride * height as usize];

        let texture = Texture::load_pixels(
            pixels.clone(),
            width,
            height,
            TextureFormat::Rgba8Unorm, // linear
        );
        let texture = world.assets.textures.add(texture);

        let material = Material::builder()
            .name("MSDF Font Atlas".into())
            .diffuse_texture(texture)
            .build();
        let material = world.assets.materials.add(material);

        Self {
            width,
            height,
            pixels,
            stride,
            alloc: AtlasAllocator::new(size2(width as i32, height as i32)),
            entries: RwLock::new(HashMap::new()),
            metrics: FontLineMetrics {
                ascent_em,
                descent_em,
                line_gap_em,
                units_per_em,
            },
            shrinkage,
            range,
            face_bytes,
            texture,
            material,
        }
    }

    pub fn font_params(&self) -> (Arc<Vec<u8>>, f32, f64, f64) {
        (
            self.face_bytes.clone(),
            self.metrics.units_per_em,
            self.shrinkage,
            self.range,
        )
    }

    pub(crate) fn integrate_ready_glyph(
        &mut self,
        cache: &AssetCache,
        queue: &wgpu::Queue,
        glyph: GlyphBitmap,
    ) -> Option<GlyphAtlasEntry> {
        let pad = 2i32;
        let alloc = self.alloc.allocate(size2(
            glyph.width_px as i32 + 2 * pad,
            glyph.height_px as i32 + 2 * pad,
        ))?;
        let rect = alloc.rectangle;

        let dest_x = (rect.min.x + pad) as u32;
        let dest_y = (rect.min.y + pad) as u32;

        for row in 0..glyph.height_px {
            let dst_off = ((dest_y + row) as usize * self.stride) + (dest_x as usize) * 4;
            let src_off = (row as usize) * (glyph.width_px as usize) * 4;
            self.pixels[dst_off..dst_off + (glyph.width_px as usize) * 4].copy_from_slice(
                &glyph.pixels_rgba[src_off..src_off + (glyph.width_px as usize) * 4],
            );
        }

        let gpu_texture = cache.textures.try_get(self.texture, cache).unwrap();
        let copy = wgpu::TexelCopyTextureInfo {
            texture: &gpu_texture.texture,
            mip_level: 0,
            origin: Origin3d {
                x: rect.min.x.max(0) as u32,
                y: rect.min.y.max(0) as u32,
                z: 0,
            },
            aspect: TextureAspect::All,
        };
        queue.write_texture(
            copy,
            &self.pixels[((dest_y as usize) * self.stride) + (dest_x as usize) * 4
                ..((dest_y + glyph.height_px - 1) as usize) * self.stride
                    + (dest_x as usize) * 4
                    + (glyph.width_px as usize) * 4],
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(self.stride as u32),
                rows_per_image: Some(glyph.height_px),
            },
            Extent3d {
                width: glyph.width_px,
                height: glyph.height_px,
                depth_or_array_layers: 1,
            },
        );

        let uv_min = [
            dest_x as f32 / self.width as f32,
            dest_y as f32 / self.height as f32,
        ];
        let uv_max = [
            (dest_x + glyph.width_px) as f32 / self.width as f32,
            (dest_y + glyph.height_px) as f32 / self.height as f32,
        ];

        let entry = GlyphAtlasEntry {
            uv_min,
            uv_max,
            plane_min: glyph.plane_min,
            plane_max: glyph.plane_max,
            advance_em: glyph.advance_em,
            msdf_range_px: glyph.msdf_range_px,
        };
        self.entries.write().unwrap().insert(glyph.ch, entry);
        Some(entry)
    }

    pub fn metrics(&self) -> FontLineMetrics {
        self.metrics
    }

    pub fn entry(&self, ch: char) -> Option<GlyphAtlasEntry> {
        self.entries.read().unwrap().get(&ch).copied()
    }

    pub fn contains(&self, ch: char) -> bool {
        self.entries.read().unwrap().contains_key(&ch)
    }

    pub fn texture(&self) -> HTexture {
        self.texture
    }

    pub fn material(&self) -> HMaterial {
        self.material
    }
}
