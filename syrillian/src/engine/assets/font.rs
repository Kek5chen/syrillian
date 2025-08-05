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
use std::sync::Mutex;

#[derive(Debug)]
pub struct Font {
    pub(crate) inner: font_kit::font::Font,
    family_name: String,
    pub(crate) pregenerated_atlas: Mutex<Option<Canvas>>,
    pub(crate) atlas_glyph_size: i32,
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
    ['/', '?', '`', '~', ' ', '\t', '\n', '\r', '\0', ' '],
];

pub const DEFAULT_GLYPH_SIZE: i32 = 100;

impl Font {
    /// The default atlas glyph size is 100 pixels
    pub fn new(family_name: String, atlas_glyph_size: Option<i32>) -> Self {
        let atlas_glyph_size = atlas_glyph_size.unwrap_or(DEFAULT_GLYPH_SIZE);
        let font = find_font(family_name.clone());
        let canvas = render_font_atlas(&font, atlas_glyph_size);

        Self {
            inner: font,
            family_name,
            pregenerated_atlas: Mutex::new(Some(canvas)),
            atlas_glyph_size,
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

pub fn render_font_atlas(font: &font_kit::font::Font, glyph_size: i32) -> Canvas {
    let point_size = glyph_size;
    let point_size_f = point_size as f32;
    let mut canvas = Canvas::new(Vector2I::splat(point_size * 10), Format::Rgba32);
    for (y, row) in FONT_ATLAS_CHARS.iter().enumerate() {
        for (x, ch) in row.iter().enumerate() {
            let x = x as f32;
            let y = y as f32 + 1.;
            let glyph_id = font.glyph_for_char(*ch).unwrap();
            let origin = font.origin(glyph_id).unwrap() * point_size_f;

            font.rasterize_glyph(
                &mut canvas,
                glyph_id,
                point_size_f,
                Transform2F::from_translation(Vector2F::new(
                    origin.x() + point_size_f * x + point_size_f / 8.,
                    origin.y() + point_size_f * y - point_size_f / 8.,
                )),
                HintingOptions::Full(point_size_f),
                RasterizationOptions::GrayscaleAa,
            )
                .unwrap();
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
        (10 * point_size * 10 * point_size * 4) as usize
    );

    canvas
}

fn find_font(family_name: String) -> font_kit::font::Font {
    let target_family = FamilyName::Title(family_name);
    let families = &[target_family, FamilyName::SansSerif];

    let font = SystemSource::new()
        .select_best_match(families, &Properties::new())
        .unwrap()
        .load()
        .unwrap();

    let target_name = match &families[0] {
        FamilyName::Title(name) => name,
        _ => unreachable!(),
    };

    let chosen_font = font.family_name();
    if &chosen_font != target_name {
        warn!("Didn't find Font {target_name:?}, fell back to {chosen_font:?}");
    }

    font
}
