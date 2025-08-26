use crate::assets::{HFont, HandleName, Store, StoreDefaults, StoreType, StoreTypeFallback, H};
use crate::store_add_checked;
use font_kit::family_name::FamilyName;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;
use itertools::Itertools;
use nalgebra::Vector2;
use std::convert::Into;
use std::sync::Arc;

#[derive(Debug)]
pub struct Font {
    pub(crate) family_name: String,
    pub(crate) font_bytes: Arc<Vec<u8>>,
    pub(crate) atlas_em_px: u32,
}

impl StoreType for Font {
    fn name() -> &'static str {
        "Font"
    }

    fn ident_fmt(handle: H<Self>) -> HandleName<Self> {
        HandleName::Id(handle)
    }

    fn is_builtin(_: H<Self>) -> bool {
        false
    }
}

impl H<Font> {
    const DEFAULT_ID: u32 = 0;
    pub const DEFAULT: HFont = HFont::new(Self::DEFAULT_ID);
}

impl StoreDefaults for Font {
    fn populate(store: &mut Store<Self>) {
        store_add_checked!(store, HFont::DEFAULT_ID, Font::new("Arial".to_string(), None));
    }
}

impl StoreTypeFallback for Font {
    fn fallback() -> H<Self> {
        HFont::DEFAULT
    }
}

pub const FONT_ATLAS_GRID_N: u32 = 10;
pub const FONT_ATLAS_CHARS: [[char; FONT_ATLAS_GRID_N as usize]; FONT_ATLAS_GRID_N as usize] = [
    ['1', '2', '3', '4', '5', '6', '7', '8', '9', '0'],
    ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j'],
    ['k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't'],
    ['u', 'v', 'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D'],
    ['E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N'],
    ['O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X'],
    ['Y', 'Z', '!', '@', '#', '$', '%', '^', '&', '*'],
    ['(', ')', '-', '_', '+', '=', '[', ']', '{', '}'],
    ['|', '\\', ':', ';', '"', '\'', '<', '>', ',', '.'],
    ['/', '?', '`', '~', ' ', ' ', ' ', ' ', ' ', ' '],
];

pub const DEFAULT_ATLAS_SIZE: u32 = 1024;

impl Font {
    /// The default atlas glyph size is 100 pixels
    pub fn new(family_name: String, atlas_em_px: Option<u32>) -> Self {
        let atlas_em_px = atlas_em_px.unwrap_or(DEFAULT_ATLAS_SIZE);
        let (_, bytes) = find_font_and_bytes(family_name.clone());
        Self {
            family_name,
            font_bytes: bytes,
            atlas_em_px,
        }
    }
}

impl Store<Font> {
    #[inline]
    pub fn load(&self, font_family: &str, atlas_em_px: Option<u32>) -> HFont {
        self.load_sized(font_family, atlas_em_px)
    }

    pub fn load_sized(&self, font_family: &str, atlas_em_px: Option<u32>) -> HFont {
        if let Some(font) = self.find(&font_family) {
            return font;
        }

        self.add(Font::new(font_family.to_string(), atlas_em_px))
    }

    pub fn find(&self, family_name: &str) -> Option<HFont> {
        self.items()
            .find(|item| item.family_name == family_name)
            .map(|item| (*item.key()).into())
    }
}

pub fn id_from_atlas(character: char) -> Vector2<u32> {
    for (y, row) in FONT_ATLAS_CHARS.iter().enumerate() {
        if let Some((x, _)) = row.iter().find_position(|c| **c == character) {
            return Vector2::new(x as u32, y as u32);
        }
    }
    Vector2::new(0, 0)
}

fn find_font_and_bytes(family_name: String) -> (font_kit::font::Font, Arc<Vec<u8>>) {
    let target_family = FamilyName::Title(family_name);
    let families = &[target_family.clone(), FamilyName::SansSerif];
    let handle = SystemSource::new().select_best_match(families, &Properties::new()).unwrap();

    let font = handle.load().unwrap();
    let bytes = match font.handle() {
        Some(font_kit::handle::Handle::Memory { bytes, .. }) => bytes.clone(),
        Some(font_kit::handle::Handle::Path { path, .. }) => {
            Arc::new(std::fs::read(path).expect("read font file"))
        }
        None => panic!("font-kit did not expose a handle; cannot build MSDF"),
    };

    (font, bytes)
}