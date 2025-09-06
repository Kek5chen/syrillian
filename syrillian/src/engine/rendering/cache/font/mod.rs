use crate::World;
use crate::assets::{Font, HMaterial, HTexture};
use crate::rendering::glyph::GlyphBitmap;
use crate::rendering::msdf_atlas::{FontLineMetrics, GlyphAtlasEntry, MsdfAtlas};
use crate::rendering::{AssetCache, CacheType};
use dashmap::DashSet;
use fdsm::bezier::scanline::FillRule;
use fdsm::generate::generate_msdf;
use fdsm::render::correct_sign_msdf;
use fdsm::shape::Shape;
use fdsm::transform::Transform;
use fdsm_ttf_parser::load_shape_from_face;
use image::RgbImage;
use nalgebra::Affine2;
use std::sync::{Arc, RwLock, mpsc};
use ttf_parser::Face;
use wgpu::{Device, Queue};

pub mod glyph;
pub mod msdf_atlas;

pub struct FontAtlas {
    atlas: Arc<RwLock<MsdfAtlas>>,
    requested: DashSet<char>,

    #[cfg(not(target_arch = "wasm32"))]
    gen_tx: mpsc::Sender<char>,
    #[cfg(not(target_arch = "wasm32"))]
    ready_rx: mpsc::Receiver<GlyphBitmap>,

    #[cfg(target_arch = "wasm32")]
    pending: std::collections::VecDeque<char>,

    #[cfg(target_arch = "wasm32")]
    wasm_face_bytes: Arc<Vec<u8>>,
    #[cfg(target_arch = "wasm32")]
    wasm_units_per_em: f32,
    #[cfg(target_arch = "wasm32")]
    wasm_shrinkage: f64,
    #[cfg(target_arch = "wasm32")]
    wasm_range: f64,
}

impl CacheType for Font {
    type Hot = FontAtlas;

    fn upload(self, _device: &Device, _queue: &Queue, _cache: &AssetCache) -> Self::Hot {
        let world = World::instance();

        let msdf = MsdfAtlas::new(self.font_bytes.clone(), self.atlas_em_px, 16.0, 4.0, world);
        let atlas = Arc::new(RwLock::new(msdf));

        #[cfg(not(target_arch = "wasm32"))]
        let (gen_tx, ready_rx) = {
            let (tx_req, rx_req) = mpsc::channel();
            let (tx_ready, rx_ready) = mpsc::channel();
            let (face_bytes, units_per_em, shrinkage, range) = atlas.read().unwrap().font_params();

            std::thread::spawn(move || {
                while let Ok(ch) = rx_req.recv() {
                    if let Some(bmp) =
                        rasterize_msdf_glyph(&face_bytes, ch, shrinkage, range, units_per_em)
                    {
                        let _ = tx_ready.send(bmp);
                    }
                }
            });
            (tx_req, rx_ready)
        };

        #[cfg(target_arch = "wasm32")]
        let (pending, wasm_face_bytes, wasm_units_per_em, wasm_shrinkage, wasm_range) = {
            let (fb, upm, s, r) = atlas.read().unwrap().font_params();
            (std::collections::VecDeque::new(), fb, upm, s, r)
        };

        FontAtlas {
            atlas,
            requested: DashSet::new(),

            #[cfg(not(target_arch = "wasm32"))]
            gen_tx,
            #[cfg(not(target_arch = "wasm32"))]
            ready_rx,

            #[cfg(target_arch = "wasm32")]
            pending,
            #[cfg(target_arch = "wasm32")]
            wasm_face_bytes,
            #[cfg(target_arch = "wasm32")]
            wasm_units_per_em,
            #[cfg(target_arch = "wasm32")]
            wasm_shrinkage,
            #[cfg(target_arch = "wasm32")]
            wasm_range,
        }
    }
}

impl FontAtlas {
    pub fn atlas(&self) -> HMaterial {
        self.atlas.read().unwrap().material()
    }
    pub fn texture(&self) -> HTexture {
        self.atlas.read().unwrap().texture()
    }
    pub fn metrics(&self) -> FontLineMetrics {
        self.atlas.read().unwrap().metrics()
    }

    pub fn entry(&self, ch: char) -> Option<GlyphAtlasEntry> {
        self.atlas.read().unwrap().entry(ch)
    }

    pub fn request_glyphs(&self, chars: impl IntoIterator<Item = char>) {
        for ch in chars {
            let atlas = self.atlas.read().unwrap();
            if !atlas.contains(ch) && self.requested.insert(ch) {
                #[cfg(not(target_arch = "wasm32"))]
                let _ = self.gen_tx.send(ch);
                #[cfg(target_arch = "wasm32")]
                self.pending.push_back(ch);
            }
        }
    }

    pub fn pump(&self, cache: &AssetCache, queue: &Queue, max_glyphs: usize) {
        if self.requested.is_empty() {
            return;
        }

        let mut processed = 0;

        #[cfg(not(target_arch = "wasm32"))]
        while processed < max_glyphs {
            match self.ready_rx.try_recv() {
                Ok(bmp) => {
                    if let Ok(mut atlas) = self.atlas.write() {
                        let _ = atlas.integrate_ready_glyph(cache, queue, bmp.clone());
                    }
                    self.requested.remove(&bmp.ch);
                    processed += 1;
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => break,
            }
        }

        #[cfg(target_arch = "wasm32")]
        while processed < max_glyphs {
            let Some(ch) = self.pending.pop_front() else {
                break;
            };
            if let Some(bmp) = rasterize_msdf_glyph(
                &self.wasm_face_bytes,
                ch,
                self.wasm_shrinkage,
                self.wasm_range,
                self.wasm_units_per_em,
            ) {
                if let Ok(mut atlas) = self.atlas.write() {
                    let _ = atlas.integrate_ready_glyph(cache, queue, bmp.clone());
                }
            }
            self.requested.remove(&ch);
            processed += 1;
        }
    }
}

fn rasterize_msdf_glyph(
    face_bytes: &Arc<Vec<u8>>,
    ch: char,
    shrinkage: f64,
    range: f64,
    metrics_units_per_em: f32,
) -> Option<GlyphBitmap> {
    let face = Face::parse(face_bytes, 0).ok()?;
    let gid = face.glyph_index(ch)?;
    let bbox = face.glyph_bounding_box(gid).unwrap_or(ttf_parser::Rect {
        x_min: 0,
        y_min: 0,
        x_max: 1,
        y_max: 1,
    });

    let upm = metrics_units_per_em as f64;
    let left_em = bbox.x_min as f32 / upm as f32;
    let right_em = bbox.x_max as f32 / upm as f32;
    let bottom_em = bbox.y_min as f32 / upm as f32;
    let top_em = bbox.y_max as f32 / upm as f32;

    let width_px = (((bbox.x_max - bbox.x_min) as f64) / shrinkage + 2.0 * range)
        .ceil()
        .max(1.0) as u32;
    let height_px = (((bbox.y_max - bbox.y_min) as f64) / shrinkage + 2.0 * range)
        .ceil()
        .max(1.0) as u32;

    let s = 1.0 / shrinkage;
    let tx = range - (bbox.x_min as f64) * s;
    let ty = range + (bbox.y_max as f64) * s;

    let transform = Affine2::from_matrix_unchecked(nalgebra::Matrix3::new(
        s, 0.0, tx, 0.0, -s, ty, 0.0, 0.0, 1.0,
    ));

    let mut shape: Shape<_> = load_shape_from_face(&face, gid);
    shape.transform(&transform);
    let colored = Shape::edge_coloring_simple(shape, 0.03, 0xD15EA5u64);
    let prepared = colored.prepare();

    let mut msdf: RgbImage = RgbImage::new(width_px, height_px);
    generate_msdf(&prepared, range, &mut msdf);
    correct_sign_msdf(&mut msdf, &prepared, FillRule::Nonzero);

    let mut pixels_rgba = vec![0u8; (width_px as usize) * (height_px as usize) * 4];
    let src = msdf.as_raw();
    for i in 0..(width_px as usize * height_px as usize) {
        pixels_rgba[4 * i] = src[3 * i];
        pixels_rgba[4 * i + 1] = src[3 * i + 1];
        pixels_rgba[4 * i + 2] = src[3 * i + 2];
        pixels_rgba[4 * i + 3] = 255;
    }

    let adv_units = face.glyph_hor_advance(gid).unwrap_or(0) as f32;
    let advance_em = adv_units / metrics_units_per_em;

    Some(GlyphBitmap {
        ch,
        width_px,
        height_px,
        plane_min: [left_em, bottom_em],
        plane_max: [right_em, top_em],
        advance_em,
        msdf_range_px: range as f32,
        pixels_rgba,
    })
}
