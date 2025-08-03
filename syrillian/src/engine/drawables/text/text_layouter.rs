use crate::assets::{HMaterial, HShader, HTexture, Material, Texture};
use crate::core::{GameObjectId, ModelUniform};
use crate::drawables::text::glyph::{generate_glyph_geometry_stream, GlyphRenderData};
use crate::drawables::text::render_font_atlas;
use crate::drawables::{BoneData, MeshUniformIndex};
use crate::rendering::uniform::ShaderUniform;
use crate::rendering::{AssetCache, Renderer};
use crate::utils::hsv_to_rgb;
use crate::{ensure_aligned, World};
use font_kit::canvas::Canvas;
use font_kit::family_name::FamilyName;
use font_kit::font::Font;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;
use log::{error, warn};
use nalgebra::{Matrix4, Vector2, Vector3};
use std::marker::PhantomData;
use std::sync::RwLock;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{Buffer, BufferUsages, RenderPass, ShaderStages};

const DEFAULT_GLYPH_SIZE: i32 = 100;

#[derive(Debug)]
pub struct TextRenderData {
    uniform: ShaderUniform<MeshUniformIndex>,
    glyph_vbo: Buffer,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TextPushConstants {
    position: Vector2<f32>,
    padding: [u32; 2],
    color: Vector3<f32>,
    text_size: f32,
}

ensure_aligned!(TextPushConstants { position, color }, align <= 16 * 2 => size);

#[derive(Debug)]
pub struct ThreeD;
#[derive(Debug)]
pub struct TwoD;

pub trait TextDim {
    fn shader() -> HShader;
}

#[derive(Debug)]
pub struct TextLayouter<DIM> {
    text: String,
    last_text_len: usize,
    glyph_data: Vec<GlyphRenderData>,
    text_dirty: bool,

    font: Font,
    pregenerated_canvas: Option<Canvas>,
    atlas_glyph_size: i32,
    font_atlas: HTexture,
    font_atlas_mat: HMaterial,
    font_dirty: bool,

    pc: TextPushConstants,
    rainbow_mode: bool,
    translation: ModelUniform,
    render_data: Option<TextRenderData>,

    _dim: PhantomData<DIM>,
}

impl<DIM: TextDim> TextLayouter<DIM> {
    pub fn new(text: String, font_family: String, text_size: f32, glyph_size: Option<i32>) -> Self {
        let glyph_size = glyph_size.unwrap_or(DEFAULT_GLYPH_SIZE);
        let font = find_font(font_family);
        let glyph_data = generate_glyph_geometry_stream(&text, &font);
        let canvas = render_font_atlas(&font, glyph_size);

        Self {
            text,
            last_text_len: 0,
            glyph_data,
            text_dirty: false,

            font,
            pregenerated_canvas: Some(canvas),
            atlas_glyph_size: glyph_size,
            font_atlas: HTexture::FALLBACK_DIFFUSE,
            font_atlas_mat: HMaterial::FALLBACK,
            font_dirty: false,

            pc: TextPushConstants {
                text_size,
                padding: [0; 2],
                position: Vector2::zeros(),
                color: Vector3::new(1., 1., 1.),
            },
            rainbow_mode: false,
            translation: ModelUniform::empty(),
            render_data: None,

            _dim: PhantomData,
        }
    }

    pub fn setup(&mut self, renderer: &Renderer, world: &mut World) {
        self.remake_atlas(world);

        let device = &renderer.state.device;

        let glyph_vbo = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Text 2D Glyph Data"),
            contents: bytemuck::cast_slice(&self.glyph_data[..]),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });

        let model_bgl = renderer.cache.bgl_model();
        let uniform = ShaderUniform::<MeshUniformIndex>::builder(&model_bgl)
            .with_buffer_data(&self.translation)
            .with_buffer_data(&BoneData::DUMMY_BONE)
            .build(device);

        self.render_data = Some(TextRenderData { uniform, glyph_vbo })
    }

    pub fn update(
        &mut self,
        world: &mut World,
        parent: GameObjectId,
        renderer: &Renderer,
        outer_transform: &Matrix4<f32>,
    ) {
        if self.rainbow_mode {
            let time = world.start_time().elapsed().as_secs_f32() * 100.;
            self.pc.color = hsv_to_rgb(time % 360., 1.0, 1.0);
        }

        if self.font_dirty {
            self.remake_atlas(world);
            self.text_dirty = false;
        }

        let render_data = self
            .render_data
            .as_mut()
            .expect("Render Data should be set up");

        self.translation.update(parent, outer_transform);

        let mesh_buffer = render_data.uniform.buffer(MeshUniformIndex::MeshData);

        renderer
            .state
            .queue
            .write_buffer(mesh_buffer, 0, bytemuck::bytes_of(&self.translation));

        if self.text_dirty {
            if self.text.len() > self.last_text_len {
                render_data.glyph_vbo = renderer
                    .state
                    .device
                    .create_buffer_init(&BufferInitDescriptor {
                        label: Some("Text 2D Glyph Data"),
                        contents: bytemuck::cast_slice(&self.glyph_data[..]),
                        usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                    });
            } else {
                renderer.state.queue.write_buffer(
                    &render_data.glyph_vbo,
                    0,
                    bytemuck::cast_slice(&self.glyph_data[..]),
                );
            }

            self.last_text_len = self.text.len();
            self.text_dirty = false;
        }
    }

    fn remake_atlas(&mut self, world: &mut World) {
        let canvas = self
            .pregenerated_canvas
            .take()
            .unwrap_or_else(|| render_font_atlas(&self.font, self.atlas_glyph_size));

        let texture = Texture::load_pixels(
            canvas.pixels,
            canvas.size.x() as u32,
            canvas.size.y() as u32,
            wgpu::TextureFormat::Bgra8UnormSrgb,
        );

        world.assets.textures.remove(self.font_atlas);
        self.font_atlas = world.assets.textures.add(texture);

        let material = Material::builder()
            .name("Font Atlas".to_string())
            .diffuse_texture(self.font_atlas)
            .build();

        world.assets.materials.remove(self.font_atlas_mat);
        self.font_atlas_mat = world.assets.materials.add(material);
    }

    pub fn draw(&self, cache: &AssetCache, pass: &RwLock<RenderPass>) {
        let Some(render_data) = &self.render_data else {
            error!("Render data wasn't set up.");
            return;
        };

        let shader = cache.shader(DIM::shader());
        let material = cache.material(self.font_atlas_mat);

        let mut pass = pass.write().unwrap();

        pass.set_pipeline(&shader.pipeline);
        pass.set_vertex_buffer(0, render_data.glyph_vbo.slice(..));
        pass.set_push_constants(
            ShaderStages::VERTEX_FRAGMENT,
            0,
            bytemuck::bytes_of(&self.pc),
        );
        pass.set_bind_group(1, render_data.uniform.bind_group(), &[]);
        pass.set_bind_group(2, material.uniform.bind_group(), &[]);

        pass.draw(0..self.glyph_data.len() as u32 * 6, 0..1);
    }

    pub fn regenerate_geometry(&mut self) {
        let glyph_data = generate_glyph_geometry_stream(&self.text, &self.font);
        self.glyph_data = glyph_data;
        self.text_dirty = true;
    }

    pub fn regenerate_atlas(&mut self) {
        self.font_dirty = true;
        self.pregenerated_canvas = Some(render_font_atlas(&self.font, self.atlas_glyph_size));
    }

    pub fn set_atlas_glyph_size(&mut self, glyph_size: i32) {
        self.atlas_glyph_size = glyph_size;
        self.regenerate_atlas();
    }

    pub fn set_text(&mut self, text: String) {
        self.text = text;
        self.regenerate_geometry();
    }

    pub fn set_font(&mut self, family_name: String) {
        self.font = find_font(family_name);

        self.regenerate_atlas();
        self.regenerate_geometry();
    }

    pub const fn set_position(&mut self, x: f32, y: f32) {
        self.set_position_vec(Vector2::new(x, y));
    }

    pub const fn set_position_vec(&mut self, pos: Vector2<f32>) {
        self.pc.position = pos;
    }

    pub const fn set_color(&mut self, r: f32, g: f32, b: f32) {
        self.set_color_vec(Vector3::new(r, g, b));
    }

    pub const fn set_color_vec(&mut self, color: Vector3<f32>) {
        self.pc.color = color;
    }

    pub const fn set_size(&mut self, text_size: f32) {
        self.pc.text_size = text_size;
    }

    pub const fn rainbow_mode(&mut self, enabled: bool) {
        self.rainbow_mode = enabled;
    }
}

impl TextDim for ThreeD {
    fn shader() -> HShader {
        HShader::TEXT_3D
    }
}

impl TextDim for TwoD {
    fn shader() -> HShader {
        HShader::TEXT_2D
    }
}

pub fn find_font(family_name: String) -> Font {
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
