use crate::assets::{HMaterial, HShader, HTexture, Material, Texture};
use crate::core::ModelUniform;
use crate::drawables::Drawable;
use crate::rendering::{DrawCtx, Renderer};
use crate::World;
use font_kit::canvas::{Canvas, Format, RasterizationOptions};
use font_kit::family_name::FamilyName;
use font_kit::font::Font;
use font_kit::hinting::HintingOptions;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;
use itertools::Itertools;
use log::{error, trace};
use nalgebra::Vector2;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::{Vector2F, Vector2I};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{Buffer, BufferUsages, ShaderStages};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GlyphVertex {
    pos: Vector2<f32>,
    atlas_uv: Vector2<f32>,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GlyphRenderData {
    triangles: [[GlyphVertex; 3]; 2],
}

#[derive(Debug)]
pub struct TextRenderData {
    _translation_data: ModelUniform,
    glyph_vbo: Buffer,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TextPushConstants {
    glyph_size: u32,
    _padding: u32,
    position: Vector2<f32>,
}

#[derive(Debug)]
pub struct Text2D {
    text: String,
    font_atlas: HTexture,
    font_atlas_mat: HMaterial,
    font: Font,
    pc: TextPushConstants,
    glyph_data: Vec<GlyphRenderData>,
    render_data: Option<TextRenderData>,
}

const FONT_ATLAS_GRID_N: u32 = 10;
const FONT_ATLAS_CHARS: [[char; FONT_ATLAS_GRID_N as usize]; FONT_ATLAS_GRID_N as usize] = [
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

fn id_from_atlas(character: char) -> Vector2<u32> {
    for (y, row) in FONT_ATLAS_CHARS.iter().enumerate() {
        if let Some((x, _)) = row.iter().find_position(|c| **c == character) {
            return Vector2::new(x as u32, y as u32);
        }
    }
    Vector2::new(0, 0)
}

impl GlyphVertex {
    pub fn new(pos: Vector2<f32>, atlas_uv: Vector2<f32>) -> Self {
        Self { pos, atlas_uv }
    }
}

impl GlyphRenderData {
    pub fn new(offset: &Vector2<f32>, glyph: char) -> Self {
        let atlas_id = id_from_atlas(glyph);
        let atlas_len = FONT_ATLAS_GRID_N as f32;
        let atlas_base = Vector2::new(atlas_id.x as f32, atlas_id.y as f32 + 1.);

        let atlas_top_left = (atlas_base + Vector2::new(0.0, -1.0)) / atlas_len;
        let atlas_top_right = (atlas_base + Vector2::new(1.0, -1.0)) / atlas_len;
        let atlas_bottom_right = (atlas_base + Vector2::new(1.0, 0.0)) / atlas_len;
        let atlas_bottom_left = atlas_base / atlas_len;

        let pos_top_left = Vector2::new(offset.x, offset.y + 1.);
        let pos_top_right = Vector2::new(offset.x + 1., offset.y + 1.);
        let pos_bottom_left = Vector2::new(offset.x, offset.y);
        let pos_bottom_right = Vector2::new(offset.x + 1., offset.y);

        let top_left = GlyphVertex::new(pos_top_left, atlas_top_left);
        let top_right = GlyphVertex::new(pos_top_right, atlas_top_right);
        let bottom_left = GlyphVertex::new(pos_bottom_left, atlas_bottom_left);
        let bottom_right = GlyphVertex::new(pos_bottom_right, atlas_bottom_right);

        Self {
            triangles: [
                // [top_left, top_right.clone(), bottom_left.clone()],
                // [top_right, bottom_right, bottom_left.clone()],
                [top_left, bottom_left.clone(), top_right.clone()],
                [top_right, bottom_left.clone(), bottom_right],
            ],
        }
    }
}

impl Text2D {
    pub fn new(text: String, font_family: String, glyph_size: u32) -> Self {
        let font = SystemSource::new()
            .select_best_match(
                &[FamilyName::Title(font_family), FamilyName::SansSerif],
                &Properties::new(),
            )
            .unwrap()
            .load()
            .unwrap();

        let glyph_data = generate_glyph_geometry_stream(&text, &font);

        Self {
            text,
            font_atlas: HTexture::FALLBACK_DIFFUSE,
            font_atlas_mat: HMaterial::FALLBACK,
            font,
            pc: TextPushConstants {
                position: Vector2::zeros(),
                _padding: 0,
                glyph_size,
            },
            glyph_data,
            render_data: None,
        }
    }

    #[inline]
    pub fn set_text(&mut self, text: String) {
        self.text = text;

        let glyph_data = generate_glyph_geometry_stream(&self.text, &self.font);
        self.glyph_data = glyph_data;
    }

    fn generate_font_atlas(&self) -> Canvas {
        let point_size = self.pc.glyph_size as i32;
        let point_size_f = point_size as f32;
        let mut canvas = Canvas::new(Vector2I::splat(point_size * 10), Format::Rgba32);
        for (y, row) in FONT_ATLAS_CHARS.iter().enumerate() {
            for (x, ch) in row.iter().enumerate() {
                let x = x as f32;
                let y = y as f32 + 1.;
                let glyph_id = self.font.glyph_for_char(*ch).unwrap();
                let origin = self.font.origin(glyph_id).unwrap() * point_size_f;

                self.font
                    .rasterize_glyph(
                        &mut canvas,
                        glyph_id,
                        point_size_f,
                        Transform2F::from_translation(Vector2F::new(
                            origin.x() + point_size_f * x + point_size_f / 8.,
                            origin.y() + point_size_f * y - point_size_f / 8.,
                        )),
                        HintingOptions::None,
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

    #[inline]
    pub fn set_position(&mut self, x: f32, y: f32) {
        self.pc.position = Vector2::new(x, y);
    }
}

impl Drawable for Text2D {
    fn setup(&mut self, renderer: &Renderer, world: &mut World) {
        let canvas = self.generate_font_atlas();

        let texture = Texture::load_pixels(
            canvas.pixels,
            canvas.size.x() as u32,
            canvas.size.y() as u32,
            wgpu::TextureFormat::Bgra8UnormSrgb,
        );

        self.font_atlas = world.assets.textures.add(texture);

        let material = Material::builder()
            .name("Font Atlas".to_string())
            .diffuse_texture(self.font_atlas)
            .build();

        self.font_atlas_mat = world.assets.materials.add(material);

        let device = &renderer.state.device;
        let translation_data = ModelUniform::empty();

        let glyph_vbo = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Text 2D Glyph Data"),
            contents: bytemuck::cast_slice(&self.glyph_data[..]),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });

        self.render_data = Some(TextRenderData {
            _translation_data: translation_data,
            glyph_vbo,
        })
    }
    fn draw(&self, _world: &mut World, ctx: &DrawCtx) {
        let Some(render_data) = &self.render_data else {
            error!("Render data wasn't set up.");
            return;
        };

        let shader = ctx.frame.cache.shader(HShader::TEXT_2D);
        let material = ctx.frame.cache.material(self.font_atlas_mat);

        let mut pass = ctx.pass.write().unwrap();

        pass.set_pipeline(&shader.pipeline);

        pass.set_vertex_buffer(0, render_data.glyph_vbo.slice(..));

        pass.set_push_constants(
            ShaderStages::VERTEX,
            0,
            bytemuck::bytes_of(&self.pc),
        );

        pass.set_bind_group(2, material.uniform.bind_group(), &[]);

        pass.draw(0..self.glyph_data.len() as u32 * 6, 0..1);
    }
}

fn generate_glyph_geometry_stream(text: &str, font: &Font) -> Vec<GlyphRenderData> {
    let mut glyph_bounds: Vec<GlyphRenderData> = Vec::new();
    let mut offset = Vector2::zeros();

    for character in text.chars() {
        if character == '\n' {
            offset = Vector2::new(0., offset.y + 1.);
            continue;
        }

        let glyph_id = font.glyph_for_char(character).unwrap();
        let glyph_size = font.advance(glyph_id).unwrap() / 2048.;
        glyph_bounds.push(GlyphRenderData::new(&offset, character));
        offset.x += glyph_size.x();
    }

    println!("{glyph_bounds:?}");

    glyph_bounds
}
