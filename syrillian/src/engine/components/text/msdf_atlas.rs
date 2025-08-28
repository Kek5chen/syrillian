use crate::World;
use crate::assets::{HMaterial, HTexture, Material, Texture};
use crate::rendering::AssetCache;
use etagere::{AtlasAllocator, size2};
use fdsm::bezier::scanline::FillRule;
use fdsm::generate::generate_msdf;
use fdsm::render::correct_sign_msdf;
use fdsm::shape::Shape;
use fdsm::transform::Transform;
use fdsm_ttf_parser as fdsm_tt;
use image::RgbImage;
use nalgebra::Affine2;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use ttf_parser::Face;
use wgpu::{
    Extent3d, Origin3d, Queue, TexelCopyBufferLayout, TexelCopyTextureInfo, TextureAspect,
    TextureFormat,
};

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

    pub fn metrics(&self) -> FontLineMetrics {
        self.metrics
    }

    pub fn entry(&self, ch: char) -> Option<GlyphAtlasEntry> {
        self.entries.read().unwrap().get(&ch).copied()
    }

    pub fn texture(&self) -> HTexture {
        self.texture
    }
    pub fn material(&self) -> HMaterial {
        self.material
    }

    /// ensure a set of glyphs is present in the atlas
    pub fn ensure_glyphs(
        &mut self,
        cache: &AssetCache,
        chars: impl IntoIterator<Item = char>,
        queue: &Queue,
    ) {
        let mut missing: Vec<char> = Vec::new();
        {
            let map = self.entries.read().unwrap();
            for ch in chars {
                if !map.contains_key(&ch) {
                    missing.push(ch);
                }
            }
        }
        if missing.is_empty() {
            return;
        }

        for ch in missing {
            if let Some(entry) = self.rasterize_and_pack(cache, ch, queue) {
                self.entries.write().unwrap().insert(ch, entry);
            }
        }
    }

    fn rasterize_and_pack(
        &mut self,
        cache: &AssetCache,
        ch: char,
        queue: &Queue,
    ) -> Option<GlyphAtlasEntry> {
        let face = Face::parse(&self.face_bytes, 0).expect("parse face");
        let gid = face.glyph_index(ch)?;
        // bbox in font units; if None (space), make a tiny box so we still allocate padding
        let bbox = face.glyph_bounding_box(gid).unwrap_or(ttf_parser::Rect {
            x_min: 0,
            y_min: 0,
            x_max: 1,
            y_max: 1,
        });
        let upm = self.metrics.units_per_em as f64;

        // plane bounds in em (no msdf margin)
        let left_em = bbox.x_min as f32 / upm as f32;
        let right_em = bbox.x_max as f32 / upm as f32;
        let bottom_em = bbox.y_min as f32 / upm as f32;
        let top_em = bbox.y_max as f32 / upm as f32;

        // compute bitmap size in texels
        let width_px = (((bbox.x_max - bbox.x_min) as f64) / self.shrinkage + 2.0 * self.range)
            .ceil()
            .max(1.0) as u32;
        let height_px = (((bbox.y_max - bbox.y_min) as f64) / self.shrinkage + 2.0 * self.range)
            .ceil()
            .max(1.0) as u32;

        let s = 1.0 / self.shrinkage;
        let tx = self.range - (bbox.x_min as f64) * s;
        let ty = self.range + (bbox.y_max as f64) * s;

        // Transform shape from font units to texture pixels, including margin.
        let transform = Affine2::from_matrix_unchecked(nalgebra::Matrix3::new(
            s, 0.0, tx, 0.0, -s, ty, 0.0, 0.0, 1.0,
        ));

        let mut shape: Shape<_> = fdsm_tt::load_shape_from_face(&face, gid);
        shape.transform(&transform);

        let colored = Shape::edge_coloring_simple(shape, 0.03, 0xD15EA5u64);
        let prepared = colored.prepare();

        let mut msdf: RgbImage = RgbImage::new(width_px, height_px);
        generate_msdf(&prepared, self.range, &mut msdf);
        correct_sign_msdf(&mut msdf, &prepared, FillRule::Nonzero);

        let pad = 2i32;
        let alloc = self
            .alloc
            .allocate(size2(width_px as i32 + 2 * pad, height_px as i32 + 2 * pad))?;
        let rect = alloc.rectangle;

        // Blit MSDF RGB into RGBA8 atlas buffer at (x+pad, y+pad)
        let dest_x = (rect.min.x + pad) as u32;
        let dest_y = (rect.min.y + pad) as u32;
        for row in 0..height_px {
            let dst_off = ((dest_y + row) as usize * self.stride) + (dest_x as usize) * 4;
            let src = &msdf.as_raw()[(row as usize) * (width_px as usize) * 3..]
                [..(width_px as usize) * 3];
            let dst = &mut self.pixels[dst_off..dst_off + (width_px as usize) * 4];
            for x in 0..(width_px as usize) {
                dst[4 * x] = src[3 * x]; // R
                dst[4 * x + 1] = src[3 * x + 1]; // G
                dst[4 * x + 2] = src[3 * x + 2]; // B
                dst[4 * x + 3] = 255u8; // A
            }
        }

        debug_assert!(dest_x + width_px <= self.width);
        debug_assert!(dest_y + height_px <= self.height);

        // upload sub-rect to gpu
        let gpu_texture = cache.textures.try_get(self.texture, cache).unwrap();
        let copy = TexelCopyTextureInfo {
            texture: &gpu_texture.texture,
            mip_level: 0,
            origin: Origin3d {
                x: rect.min.x.max(0) as u32,
                y: rect.min.y.max(0) as u32,
                z: 0,
            },
            aspect: TextureAspect::All,
        };

        let data_offset = ((dest_y as usize) * self.stride) + (dest_x as usize) * 4;
        queue.write_texture(
            copy,
            &self.pixels[data_offset
                ..data_offset + (height_px as usize - 1) * self.stride + (width_px as usize) * 4],
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(self.stride as u32),
                rows_per_image: Some(height_px),
            },
            Extent3d {
                width: width_px,
                height: height_px,
                depth_or_array_layers: 1,
            },
        );

        let uv_min = [
            (dest_x as f32) / (self.width as f32),
            (dest_y as f32) / (self.height as f32),
        ];
        let uv_max = [
            ((dest_x + width_px) as f32) / (self.width as f32),
            ((dest_y + height_px) as f32) / (self.height as f32),
        ];

        let adv_units = face.glyph_hor_advance(gid).unwrap_or(0) as f32;
        let advance_em = adv_units / self.metrics.units_per_em;

        Some(GlyphAtlasEntry {
            uv_min,
            uv_max,
            plane_min: [left_em, bottom_em],
            plane_max: [right_em, top_em],
            advance_em,
            msdf_range_px: self.range as f32,
        })
    }
}
