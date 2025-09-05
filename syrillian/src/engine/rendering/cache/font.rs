use crate::World;
use crate::assets::{FONT_ATLAS_CHARS, Font, HMaterial, HTexture};
use crate::components::msdf_atlas::{FontLineMetrics, GlyphAtlasEntry, MsdfAtlas};
use crate::rendering::{AssetCache, CacheType};
use std::sync::{Arc, RwLock};
use wgpu::{Device, Queue};

pub struct FontAtlas {
    atlas: Arc<RwLock<MsdfAtlas>>,
}

impl CacheType for Font {
    type Hot = FontAtlas;

    fn upload(&self, _device: &Device, queue: &Queue, cache: &AssetCache) -> Self::Hot {
        let world = World::instance();

        let mut msdf = MsdfAtlas::new(self.font_bytes.clone(), self.atlas_em_px, 16.0, 4.0, world);

        msdf.ensure_glyphs(cache, FONT_ATLAS_CHARS.iter().flatten().copied(), queue);

        FontAtlas {
            atlas: Arc::new(RwLock::new(msdf)),
        }
    }
}

impl FontAtlas {
    pub fn atlas(&self) -> HMaterial {
        self.atlas
            .read()
            .unwrap_or_else(|_| {
                log::error!("Failed to read atlas to get HMaterial");
                std::process::exit(1);
            })
            .material()
    }
    pub fn texture(&self) -> HTexture {
        self.atlas
            .read()
            .unwrap_or_else(|_| {
                log::error!("Failed to read atlas to get HTexture");
                std::process::exit(1);
            })
            .texture()
    }
    pub fn metrics(&self) -> FontLineMetrics {
        self.atlas
            .read()
            .unwrap_or_else(|_| {
                log::error!("Failed to read atlas to get metrics");
                std::process::exit(1);
            })
            .metrics()
    }
    pub fn ensure_glyphs(
        &self,
        cache: &AssetCache,
        chars: impl IntoIterator<Item = char>,
        queue: &Queue,
    ) {
        let mut lock = match self.atlas.write() {
            Ok(lock) => lock,
            Err(e) => {
                log::error!(
                    "Failed to get a lock to write to atlas in ensure_glyphs in font.rs : {} ",
                    e
                );
                std::process::exit(1);
            }
        };
        lock.ensure_glyphs(cache, chars, queue);
    }
    pub fn entry(&self, ch: char) -> Option<GlyphAtlasEntry> {
        let lock = match self.atlas.write() {
            Ok(lock) => lock,
            Err(e) => {
                log::error!(
                    "Failed to get a lock to write to atlas in entry in font.rs : {} ",
                    e
                );
                std::process::exit(1);
            }
        };
        lock.entry(ch)
    }
}
