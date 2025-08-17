use crate::assets::{HFont, HandleName, Store, StoreType, H};
use font_kit::canvas::{Canvas, Format, RasterizationOptions};
use font_kit::family_name::FamilyName;
use font_kit::hinting::HintingOptions;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;
use itertools::Itertools;
use log::{trace, warn};
use nalgebra::Vector2;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::{Vector2F, Vector2I};
use std::convert::Into;
use std::sync::Arc;

#[derive(Debug)]
pub struct Font {
    pub(crate) _inner: font_kit::font::Font, // TODO: Check if still needed
    pub(crate) family_name: String,
    pub(crate) font_bytes: Arc<Vec<u8>>,
    pub(crate) _atlas_em_px: i32, // TODO: Check if still needed
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

pub const DEFAULT_GLYPH_SIZE: i32 = 100;

impl Font {
    /// The default atlas glyph size is 100 pixels
    pub fn new(family_name: String, atlas_em_px: Option<i32>) -> Self {
        let atlas_em_px = atlas_em_px.unwrap_or(DEFAULT_GLYPH_SIZE);
        let (font, bytes) = find_font_and_bytes(family_name.clone());
        Self {
            _inner: font,
            family_name,
            font_bytes: bytes,
            _atlas_em_px: atlas_em_px,
        }
    }
}

impl Store<Font> {
    #[inline]
    pub fn load(&self, font_family: &str) -> HFont {
        self.load_sized(font_family, DEFAULT_GLYPH_SIZE)
    }

    pub fn load_sized(&self, font_family: &str, atlas_glyph_size: i32) -> HFont {
        if let Some(font) = self.find(&font_family) {
            return font;
        }

        self.add(Font::new(font_family.to_string(), Some(atlas_glyph_size)))
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

pub fn render_font_atlas(font: &font_kit::font::Font, em_px: i32) -> Canvas {
    let metrics = font.metrics();

    let units = metrics.units_per_em as f32;
    let scale = em_px as f32 / units;
    let ascent_px = (metrics.ascent * scale).ceil();
    let descent_px = (-metrics.descent * scale).ceil();
    let line_px = ascent_px + descent_px;

    let pad: i32 = ((em_px as f32) * 0.12).ceil() as i32;

    let cell_w: i32 = em_px + 2 * pad;
    let cell_h: i32 = (line_px as i32) + 2 * pad;

    let width = cell_w * FONT_ATLAS_GRID_N as i32;
    let height = cell_h * FONT_ATLAS_GRID_N as i32;

    let mut canvas = Canvas::new(Vector2I::new(width, height), Format::Rgba32);

    for (row, chars) in FONT_ATLAS_CHARS.iter().enumerate() {
        for (col, ch) in chars.iter().enumerate() {
            let Some(glyph_id) = font.glyph_for_char(*ch) else {
                warn!("Font {} is missing '{ch}'", font.family_name());
                continue
            };

            let cell_x = (col as i32 * cell_w + pad) as f32;
            let cell_y = (row as i32 * cell_h + pad) as f32;

            let baseline_y = cell_y + ascent_px;

            let transform = Transform2F::from_translation(Vector2F::new(cell_x, baseline_y));

            if let Err(e) = font.rasterize_glyph(
                &mut canvas,
                glyph_id,
                em_px as f32,
                transform,
                HintingOptions::Full(em_px as f32),
                RasterizationOptions::GrayscaleAa,
            ) {
                warn!("Font {} couldn't rasterize character {ch}: {e}", font.family_name());
            }
        }
    }

    trace!(
        "Generated font atlas of size X={} Y={} (Stride {})",
        canvas.size.x(),
        canvas.size.y(),
        canvas.stride
    );

    assert_eq!(
        canvas.pixels.len(),
        canvas.stride * height as usize
    );

    canvas
}

fn find_font_and_bytes(family_name: String) -> (font_kit::font::Font, Arc<Vec<u8>>) {
    let target_family = FamilyName::Title(family_name);
    let families = &[target_family.clone(), FamilyName::SansSerif];
    let handle = SystemSource::new().select_best_match(families, &Properties::new()).unwrap();

    // get raw bytes via handle
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