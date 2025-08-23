use crate::assets::{HFont, HShader};
use crate::components::glyph::{generate_glyph_geometry_stream, GlyphRenderData, TextAlignment};
use crate::components::BoneData;
use crate::core::ModelUniform;
use crate::rendering::proxies::mesh_proxy::MeshUniformIndex;
use crate::rendering::proxies::SceneProxy;
use crate::rendering::uniform::ShaderUniform;
use crate::rendering::{AssetCache, CPUDrawCtx, GPUDrawCtx, RenderPassType, Renderer};
use crate::utils::hsv_to_rgb;
use crate::{ensure_aligned, must_pipeline, proxy_data, proxy_data_mut};
use nalgebra::{Matrix4, Vector2, Vector3};
use std::any::Any;
use std::marker::PhantomData;
use std::sync::RwLock;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{Buffer, BufferUsages, RenderPass, ShaderStages};
use winit::window::Window;


#[cfg(debug_assertions)]
use crate::rendering::DebugRenderer;

#[derive(Debug)]
pub struct TextRenderData {
    uniform: ShaderUniform<MeshUniformIndex>,
    glyph_vbo: Buffer,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TextPushConstants {
    position: Vector2<f32>,
    em_scale: f32,
    msdf_range_px: f32,
    color: Vector3<f32>,
    padding: u32,
}

ensure_aligned!(TextPushConstants { position, color }, align <= 16 * 2 => size);

#[derive(Debug, Copy, Clone)]
pub struct ThreeD;
#[derive(Debug, Copy, Clone)]
pub struct TwoD;

pub trait TextDim: Copy + Clone + Send + 'static {
    fn shader() -> HShader;
    #[cfg(debug_assertions)]
    fn debug_shader() -> HShader;
}

#[derive(Debug, Clone)]
pub struct TextProxy<DIM> {
    text: String,
    alignment: TextAlignment,
    last_text_len: usize,
    glyph_data: Vec<GlyphRenderData>,
    text_dirty: bool,

    font: HFont,

    pc: TextPushConstants,
    rainbow_mode: bool,
    constants_dirty: bool,
    translation: ModelUniform,

    _dim: PhantomData<DIM>,
}

impl<DIM: TextDim> TextProxy<DIM> {
    pub fn new(text: String, font: HFont, em_scale: f32) -> Self {
        Self {
            text,
            alignment: TextAlignment::Left,
            last_text_len: 0,
            glyph_data: Vec::new(),
            text_dirty: false,

            font,

            pc: TextPushConstants {
                em_scale,
                position: Vector2::zeros(),
                color: Vector3::new(1., 1., 1.),
                msdf_range_px: 4.0,
                padding: 0,
            },
            rainbow_mode: false,
            constants_dirty: false,
            translation: ModelUniform::empty(),

            _dim: PhantomData,
        }
    }

    pub fn update_game_thread(
        &mut self,
        mut ctx: CPUDrawCtx,
    ) {
        if self.constants_dirty {
            let constants = self.pc;
            let rainbow_mode = self.rainbow_mode;
            ctx.send_proxy_update(move |proxy| {
                let proxy: &mut Self = proxy_data_mut!(proxy);

                proxy.pc = constants;
                proxy.rainbow_mode = rainbow_mode;
            });
            self.constants_dirty = false;
        }

        if self.text_dirty {
            let text = self.text.clone();
            let font = self.font;
            ctx.send_proxy_update(move |proxy| {
                let proxy: &mut Self = proxy_data_mut!(proxy);

                proxy.text = text;
                proxy.font = font;
                proxy.text_dirty = true;
            });

            self.text_dirty = false;
        }
    }

    pub fn update_render_thread(
        &mut self,
        renderer: &Renderer,
        data: &mut TextRenderData,
        local_to_world: &Matrix4<f32>,
    ) {
        if self.text_dirty {
            self.regenerate_geometry(renderer);
        }

        self.translation.update(local_to_world);

        if self.text_dirty {
            if self.text.len() > self.last_text_len {
                data.glyph_vbo =
                    renderer
                        .state
                        .device
                        .create_buffer_init(&BufferInitDescriptor {
                            label: Some("Text 2D Glyph Data"),
                            contents: bytemuck::cast_slice(&self.glyph_data[..]),
                            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                        });
            } else {
                renderer.state.queue.write_buffer(
                    &data.glyph_vbo,
                    0,
                    bytemuck::cast_slice(&self.glyph_data[..]),
                );
            }

            self.last_text_len = self.text.len();
            self.text_dirty = false;
        }

        if self.rainbow_mode {
            let time = renderer.start_time().elapsed().as_secs_f32() * 100.;
            self.pc.color = hsv_to_rgb(time % 360., 1.0, 1.0);
        }

        let mesh_buffer = data.uniform.buffer(MeshUniformIndex::MeshData);

        renderer
            .state
            .queue
            .write_buffer(mesh_buffer, 0, bytemuck::bytes_of(&self.translation));
    }

    pub fn render(&self, renderer: &Renderer, data: &TextRenderData, ctx: &GPUDrawCtx) {
        if DIM::shader() != HShader::TEXT_3D && ctx.pass_type == RenderPassType::Shadow {
            return;
        }

        let cache: &AssetCache = &renderer.cache;
        let pass: &RwLock<RenderPass> = &ctx.pass;

        let font = cache.font(self.font);

        let shader = cache.shader(DIM::shader());
        let material = cache.material(font.atlas());

        let mut pass = pass.write().unwrap();
        must_pipeline!(pipeline = shader, ctx.pass_type => return);

        pass.set_pipeline(pipeline);
        pass.set_vertex_buffer(0, data.glyph_vbo.slice(..));
        pass.set_push_constants(
            ShaderStages::VERTEX_FRAGMENT,
            0,
            bytemuck::bytes_of(&self.pc),
        );
        pass.set_bind_group(1, data.uniform.bind_group(), &[]);
        pass.set_bind_group(2, material.uniform.bind_group(), &[]);

        pass.draw(0..self.glyph_data.len() as u32 * 6, 0..1);

        #[cfg(debug_assertions)]
        if DebugRenderer::text_geometry() {
            self.draw_debug_edges(cache, &mut pass, ctx.pass_type);
        }
    }

    #[cfg(debug_assertions)]
    fn draw_debug_edges(&self, cache: &AssetCache, pass: &mut RenderPass, pass_type: RenderPassType) {
        let shader = cache.shader(DIM::debug_shader());
        must_pipeline!(pipeline = shader, pass_type => return);
        pass.set_pipeline(pipeline);

        pass.set_push_constants(
            ShaderStages::VERTEX_FRAGMENT,
            0,
            bytemuck::bytes_of(&self.pc),
        );

        pass.draw(0..self.glyph_data.len() as u32 * 6, 0..1);
    }

    pub fn regenerate_geometry(&mut self, renderer: &Renderer) {
        let hot_font = renderer.cache.font(self.font);

        self.glyph_data = generate_glyph_geometry_stream(
            &renderer.cache,
            &renderer.state.queue,
            &self.text,
            &hot_font,
            TextAlignment::Left,
            1.0,
        );
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.text_dirty = true;
    }

    pub fn set_font(&mut self, font: HFont) {
        self.font = font;
        self.text_dirty = true;
    }

    pub const fn set_position(&mut self, x: f32, y: f32) {
        self.set_position_vec(Vector2::new(x, y));
    }

    pub fn set_alignment(&mut self, alignment: TextAlignment) {
        self.alignment = alignment;
        self.text_dirty = true;
    }

    pub const fn set_position_vec(&mut self, pos: Vector2<f32>) {
        self.pc.position = pos;
        self.constants_dirty = true;
    }

    pub const fn set_color(&mut self, r: f32, g: f32, b: f32) {
        self.set_color_vec(Vector3::new(r, g, b));
    }

    pub const fn set_color_vec(&mut self, color: Vector3<f32>) {
        self.pc.color = color;
        self.constants_dirty = true;
    }

    pub const fn set_size(&mut self, text_size_em: f32) {
        self.pc.em_scale = text_size_em;
        self.constants_dirty = true;
    }

    pub const fn set_rainbow_mode(&mut self, enabled: bool) {
        self.rainbow_mode = enabled;
        self.constants_dirty = true;
    }
}

impl<DIM: TextDim> SceneProxy for TextProxy<DIM> {
    fn setup_render(&mut self, renderer: &Renderer, _local_to_world: &Matrix4<f32>) -> Box<dyn Any> {
        let hot_font = renderer.cache.font(self.font);

        self.glyph_data = generate_glyph_geometry_stream(
            &renderer.cache,
            &renderer.state.queue,
            &self.text,
            &hot_font,
            TextAlignment::Left,
            1.0,
        );

        let device = &renderer.state.device;

        let glyph_vbo = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Text 2D Glyph Data"),
            contents: bytemuck::cast_slice(&self.glyph_data[..]),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });

        let model_bgl = renderer.cache.bgl_model();
        let uniform = ShaderUniform::<MeshUniformIndex>::builder(&model_bgl)
            .with_buffer_data(&self.translation)
            .with_buffer_data(&BoneData::DUMMY)
            .build(device);

        Box::new(TextRenderData { uniform, glyph_vbo })
    }

    fn update_render(&mut self, renderer: &Renderer, data: &mut dyn Any, _window: &Window, local_to_world: &Matrix4<f32>) {
        let data: &mut TextRenderData = proxy_data_mut!(data);

        self.update_render_thread(renderer, data, local_to_world);
    }

    fn render<'a>(&self, renderer: &Renderer, data: &dyn Any, ctx: &GPUDrawCtx, _local_to_world: &Matrix4<f32>) {
        let data: &TextRenderData = proxy_data!(data);
        self.render(renderer, data, ctx);
    }
}

impl TextDim for ThreeD {
    fn shader() -> HShader {
        HShader::TEXT_3D
    }

    #[cfg(debug_assertions)]
    fn debug_shader() -> HShader {
        HShader::DEBUG_TEXT3D_GEOMETRY
    }
}

impl TextDim for TwoD {
    fn shader() -> HShader {
        HShader::TEXT_2D
    }

    #[cfg(debug_assertions)]
    fn debug_shader() -> HShader {
        HShader::DEBUG_TEXT2D_GEOMETRY
    }
}
