use crate::assets::{Font, HMaterial, HTexture, FONT_ATLAS_CHARS};
use crate::components::msdf_atlas::{FontLineMetrics, GlyphAtlasEntry, MsdfAtlas};
use crate::rendering::{AssetCache, CacheType};
use crate::World;
use std::sync::{Arc, RwLock};
use wgpu::{Device, Queue};

pub struct FontAtlas {
    atlas: Arc<RwLock<MsdfAtlas>>,
}

impl CacheType for Font {
    type Hot = FontAtlas;

    fn upload(&self, _device: &Device, queue: &Queue, cache: &AssetCache) -> Self::Hot {
        let world = World::instance();

        let mut msdf = MsdfAtlas::new(self.font_bytes.clone(), 1024, 16.0, 4.0, &world);

        msdf.ensure_glyphs(cache, FONT_ATLAS_CHARS.iter().flatten().copied(), queue);

        FontAtlas {
            atlas: Arc::new(RwLock::new(msdf)),
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
    pub fn ensure_glyphs(
        &self,
        cache: &AssetCache,
        chars: impl IntoIterator<Item=char>,
        queue: &Queue,
    ) {
        self.atlas
            .write()
            .unwrap()
            .ensure_glyphs(cache, chars, queue)
    }
    pub fn entry(&self, ch: char) -> Option<GlyphAtlasEntry> {
        self.atlas.read().unwrap().entry(ch)
    }
}
