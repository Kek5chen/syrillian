use crate::engine::assets::generic_store::{HandleName, Store, StoreDefaults, StoreType};
use crate::engine::assets::{H, StoreTypeFallback};
use crate::store_add_checked;
use wgpu::TextureFormat;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cubemap {
    pub faces: [Vec<u8>; 6],
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
}

pub type HCubemap = H<Cubemap>;

impl H<Cubemap> {
    const FALLBACK_CUBEMAP_ID: u32 = 0;
    const MAX_BUILTIN_ID: u32 = 0;

    pub const FALLBACK_CUBEMAP: H<Cubemap> = H::new(Self::FALLBACK_CUBEMAP_ID);
}

impl Cubemap {
    const FACE_COLORS: [[u8; 4]; 6] = [
        [255, 0, 0, 255],   // Right - Red
        [0, 255, 0, 255],   // Left - Green
        [0, 0, 255, 255],   // Top - Blue
        [255, 255, 0, 255], // Bottom - Yellow
        [255, 0, 255, 255], // Front - Magenta
        [0, 255, 255, 255], // Back - Cyan
    ];
    pub fn gen_fallback_cubemap(size: u32) -> [Vec<u8>; 6] {
        std::array::from_fn(|face_idx| {
            Self::FACE_COLORS[face_idx].repeat(size as usize * size as usize)
        })
    }
}

impl StoreDefaults for Cubemap {
    fn populate(store: &mut Store<Self>) {
        const FALLBACK_SIZE: u32 = 32;
        store_add_checked!(
            store,
            HCubemap::FALLBACK_CUBEMAP_ID,
            Cubemap {
                faces: Self::gen_fallback_cubemap(FALLBACK_SIZE),
                width: FALLBACK_SIZE,
                height: FALLBACK_SIZE,
                format: TextureFormat::Bgra8UnormSrgb
            }
        );
    }
}

impl StoreType for Cubemap {
    #[inline]
    fn name() -> &'static str {
        "Cubemap"
    }

    fn ident_fmt(handle: H<Self>) -> HandleName<Self> {
        match handle.id() {
            HCubemap::FALLBACK_CUBEMAP_ID => HandleName::Static(
                "Cubemap
  Fallback",
            ),
            _ => HandleName::Id(handle),
        }
    }

    fn is_builtin(handle: H<Self>) -> bool {
        handle.id() <= H::<Self>::MAX_BUILTIN_ID
    }
}

impl StoreTypeFallback for Cubemap {
    fn fallback() -> H<Self> {
        HCubemap::FALLBACK_CUBEMAP
    }
}

impl Store<Cubemap> {}
