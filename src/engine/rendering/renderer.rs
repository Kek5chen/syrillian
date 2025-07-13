use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;
use crate::asset_management::{RuntimeShader, ShaderId, DIM3_SHADER_ID, FALLBACK_SHADER_ID, LIGHT_UBGL_ID, POST_PROCESS_BGL_ID, POST_PROCESS_SHADER_ID, RENDER_UBGL_ID};
use crate::components::{CameraComponent, CameraUniform, PointLightComponent, PointLightUniform};
use crate::core::GameObjectId;
use crate::rendering::{State, StateError};
use crate::world::World;
use log::error;
use nalgebra::{Matrix4, Perspective3, Vector2};
use snafu::{ResultExt, Snafu};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindingResource,
    Buffer, BufferAddress, BufferDescriptor, BufferUsages, Color, CommandEncoder,
    CommandEncoderDescriptor, Device, Extent3d, FilterMode, LoadOp, Operations, RenderPass,
    RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    SamplerDescriptor, StoreOp, SurfaceError, SurfaceTexture, Texture, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
};
use winit::window::Window;
use crate::ensure_aligned;

struct PostProcessPass {
    bind_group: BindGroup,
}

type Result<T, E = RenderError> = std::result::Result<T, E>;

#[derive(Debug, Snafu)]
#[snafu(context(suffix(Err)))]
pub enum RenderError {
    #[snafu(display("Render data should've been set"))]
    DataNotSet,

    #[snafu(display("Render pipeline is not set"))]
    NoRenderPipeline,

    #[snafu(display("No camera set for rendering"))]
    NoCameraSet,

    #[snafu(display("Rendering camera doesn't have a camera component"))]
    NoCameraComponentSet,

    #[snafu(display("Light UBGL was not created"))]
    NoLightUBGL,

    #[snafu(display("Error with current render surface: {source}"))]
    Surface { source: SurfaceError },

    #[snafu(display("Failed to create render state: {source}"))]
    State { source: StateError },
}

impl PostProcessPass {
    fn new(device: &Device, layout: &BindGroupLayout, view: &TextureView) -> Self {
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("PostProcess Sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..SamplerDescriptor::default()
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("PostProcess Bind Group"),
            layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });

        Self { bind_group }
    }
}

pub struct RenderContext {
    pub output: SurfaceTexture,
    pub color_view: TextureView,
    pub depth_view: TextureView,
    pub encoder: CommandEncoder,
}

#[repr(C)]
#[derive(Default, Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SystemUniform {
    screen_size: Vector2<u32>,
    time: f32,
    delta_time: f32,
}

ensure_aligned!(SystemUniform { screen_size }, align <= 8 * 2 => size);

pub struct RenderUniformData {
    camera_data: Box<CameraUniform>,
    camera_buffer: Buffer,
    system_data: Box<SystemUniform>,
    system_buffer: Buffer,
    bind_group: BindGroup,
}

#[derive(Debug, Default)]
pub struct DebugRenderer {
    pub draw_edges: bool,
}

#[allow(dead_code)]
pub struct Renderer {
    pub state: Box<State>,
    pub window: Window,
    pub current_pipeline: Option<ShaderId>,
    render_uniform_data: Option<RenderUniformData>,

    // Offscreen texture for rendering the scene before post-processing
    offscreen_texture: Texture,
    offscreen_view: TextureView,

    post_process_pass: Option<PostProcessPass>,

    pub debug: DebugRenderer,

    printed_errors: u32,
}

impl Renderer {
    pub fn create_uniform_init(
        bind_group_layout: &BindGroupLayout,
        state: &State,
        data: &'_ [u8],
    ) -> (Buffer, BindGroup) {
        let uniform_buffer = state.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: data,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let uniform_bind_group = state.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        (uniform_buffer, uniform_bind_group)
    }

    pub fn create_uniform_buffer(
        bind_group_layout: &BindGroupLayout,
        state: &State,
        data_size: usize,
    ) -> (Buffer, BindGroup) {
        let uniform_buffer = state.device.create_buffer(&BufferDescriptor {
            label: Some("Uniform Buffer"),
            size: data_size as BufferAddress,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: true,
        });

        let uniform_bind_group = state.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        (uniform_buffer, uniform_bind_group)
    }

    pub(crate) async fn new(window: Window) -> Result<Self> {
        let state = Box::new(State::new(&window).await.context(StateErr)?);

        let (offscreen_texture, offscreen_view) = Self::create_offscreen_texture(
            &state.device,
            state.config.width,
            state.config.height,
            state.config.format,
        );

        Ok(Renderer {
            state,
            window,
            current_pipeline: None,
            render_uniform_data: None,
            offscreen_texture,
            offscreen_view,
            post_process_pass: None,
            debug: DebugRenderer::default(),
            printed_errors: 0,
        })
    }

    fn create_offscreen_texture(
        device: &Device,
        width: u32,
        height: u32,
        format: TextureFormat,
    ) -> (Texture, TextureView) {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("Offscreen Texture"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&TextureViewDescriptor::default());
        (texture, view)
    }

    pub fn init(&mut self) {
        // TODO: Make it possible to pick a shader
        self.current_pipeline = Some(DIM3_SHADER_ID);

        let camera_data = Box::<CameraUniform>::default();
        let system_data = Box::<SystemUniform>::default();

        let camera_buffer = self.state.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Camera Uniform Buffer"),
            contents: bytemuck::bytes_of(camera_data.as_ref()),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let system_buffer = self.state.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("System Uniform Buffer"),
            contents: bytemuck::bytes_of(system_data.as_ref()),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let render_data_bgl = World::instance()
            .assets
            .bind_group_layouts
            .get_bind_group_layout(RENDER_UBGL_ID)
            .unwrap();
        let bind_group = self.state.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Render Uniform Bind Group"),
            layout: render_data_bgl,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: system_buffer.as_entire_binding(),
                },
            ],
        });

        self.render_uniform_data = Some(RenderUniformData {
            camera_data,
            camera_buffer,
            system_data,
            system_buffer,
            bind_group,
        });

        let world = World::instance();
        let post_bgl = world
            .assets
            .bind_group_layouts
            .get_bind_group_layout(POST_PROCESS_BGL_ID)
            .unwrap();
        self.post_process_pass = Some(PostProcessPass::new(
            &self.state.device,
            post_bgl,
            &self.offscreen_view,
        ));

        #[cfg(debug_assertions)]
        self.init_debug();
    }

    #[cfg(debug_assertions)]
    fn init_debug(&mut self) {
        self.debug.draw_edges = true;
    }

    pub fn render_world(&mut self, world: &mut World) -> bool {
        let mut ctx = match self.begin_render() {
            Ok(ctx) => ctx,
            Err(RenderError::Surface {
                source: SurfaceError::Lost,
            }) => {
                self.state.resize(self.state.size);
                return true; // drop frame but don't cancel
            }
            Err(RenderError::Surface {
                source: SurfaceError::OutOfMemory,
            }) => {
                error!("The application ran out of GPU memory!");
                return false;
            }
            Err(e) => {
                error!("Surface error: {e}");
                return false;
            }
        };

        self.render(&mut ctx, world, None);

        #[cfg(debug_assertions)]
        self.render_debug(&mut ctx, world);

        self.end_render(world, ctx);

        true
    }

    #[cfg(debug_assertions)]
    pub fn render_debug(&mut self, ctx: &mut RenderContext, world: &mut World) {
        use crate::asset_management::DEBUG_EDGES_SHADER_ID;
        if self.debug.draw_edges {
            self.render(ctx, world, Some(DEBUG_EDGES_SHADER_ID));
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.state.resize(new_size);

        // Re-create the offscreen texture and post process bind group after resize
        let (new_offscreen, new_offscreen_view) = Self::create_offscreen_texture(
            &self.state.device,
            self.state.config.width,
            self.state.config.height,
            self.state.config.format,
        );
        self.offscreen_texture = new_offscreen;
        self.offscreen_view = new_offscreen_view;

        if let Some(pp) = &mut self.post_process_pass {
            let world = World::instance();
            let post_bgl = world
                .assets
                .bind_group_layouts
                .get_bind_group_layout(POST_PROCESS_BGL_ID)
                .unwrap();
            *pp = PostProcessPass::new(&self.state.device, post_bgl, &self.offscreen_view);
        }
    }

    fn begin_render(&mut self) -> Result<RenderContext> {
        let mut output = self
            .state
            .surface
            .get_current_texture()
            .context(SurfaceErr)?;
        if output.suboptimal {
            drop(output);
            self.state.recreate_surface();
            output = self
                .state
                .surface
                .get_current_texture()
                .context(SurfaceErr)?;
        }

        let color_view = output
            .texture
            .create_view(&TextureViewDescriptor::default());
        let depth_view = self
            .state
            .depth_texture
            .create_view(&TextureViewDescriptor::default());
        let encoder = self
            .state
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Main Encoder"),
            });

        if self.current_pipeline.is_none() {
            self.current_pipeline = Some(FALLBACK_SHADER_ID);
        }

        Ok(RenderContext {
            output,
            color_view,
            depth_view,
            encoder,
        })
    }

    fn render(
        &mut self,
        ctx: &mut RenderContext,
        world: &mut World,
        shader_override: Option<ShaderId>,
    ) {
        if let Err(e) = self.render_inner(ctx, world, shader_override) {
            if self.printed_errors < 5 {
                self.printed_errors += 1;
                error!("{e}")
            }
            return;
        }
        self.printed_errors = 0;
    }

    fn render_inner(
        &mut self,
        ctx: &mut RenderContext,
        world: &mut World,
        shader_override: Option<ShaderId>,
    ) -> Result<()> {
        self.update_render_data(world)?;

        let shader_id = self.default_shader_id(shader_override)?;
        let shader = world
            .assets
            .shaders
            .get_shader(Some(shader_id));
        let (load_op_color, load_op_depth) = determine_draw_over_color(shader);

        let light_bind_group = self.setup_lights(world)?;

        let mut rpass = self.prepare_render_pass(ctx, load_op_color, load_op_depth);

        let render_data = self
            .render_uniform_data
            .as_mut()
            .ok_or(RenderError::DataNotSet)?;

        rpass.set_bind_group(0, &render_data.bind_group, &[]);
        rpass.set_bind_group(3, &light_bind_group, &[]);

        let world_ptr = world as *mut World;
        unsafe {
            self.traverse_and_render(
                &mut *world_ptr,
                &mut rpass,
                &world.children,
                Matrix4::identity(),
                shader_override,
            );
        }

        Ok(())
    }

    fn traverse_and_render(
        &self,
        world: &mut World,
        rpass: &mut RenderPass,
        children: &[GameObjectId],
        combined_matrix: Matrix4<f32>,
        shader_override: Option<ShaderId>,
    ) {
        for child in children {
            if !child.children.is_empty() {
                self.traverse_and_render(
                    world,
                    rpass,
                    &child.children,
                    combined_matrix * child.transform.full_matrix().to_homogeneous(),
                    shader_override,
                );
            }
            let Some(drawable) = &mut child.clone().drawable else {
                continue;
            };

            drawable.update(world, *child, self, &combined_matrix);
            drawable.draw(world, rpass, self, shader_override);
        }
    }

    fn render_final_pass(&mut self, world: &mut World, ctx: &mut RenderContext) {
        let mut rpass = ctx.encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("PostProcess Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &ctx.color_view,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::BLACK),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..RenderPassDescriptor::default()
        });

        let post_shader = world
            .assets
            .shaders
            .get_shader_opt(POST_PROCESS_SHADER_ID)
            .expect("PostProcess shader should be initialized");
        rpass.set_pipeline(&post_shader.pipeline);
        rpass.set_bind_group(0, &self.post_process_pass.as_ref().unwrap().bind_group, &[]);
        rpass.draw(0..6, 0..1);
    }

    fn end_render(&mut self, world: &mut World, mut ctx: RenderContext) {
        self.render_final_pass(world, &mut ctx);

        self.state.queue.submit(Some(ctx.encoder.finish()));
        ctx.output.present();
        self.window.request_redraw();
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn window_mut(&mut self) -> &mut Window {
        &mut self.window
    }

    fn update_render_data(&mut self, world: &World) -> Result<()> {
        self.update_camera_data(world)?;
        self.update_system_data(world)?;

        Ok(())
    }

    fn update_camera_data(&mut self, world: &World) -> Result<()> {
        let render_data = self
            .render_uniform_data
            .as_mut()
            .ok_or(RenderError::DataNotSet)?;

        let camera_rc = world
            .active_camera
            .as_ref()
            .ok_or(RenderError::NoCameraSet)?;

        let camera = camera_rc;
        let camera_comp = camera
            .get_component::<CameraComponent>()
            .ok_or(RenderError::NoCameraComponentSet)?;

        let projection_matrix: &Perspective3<f32> = &camera_comp.borrow_mut().projection;
        let camera_transform = &camera.transform;

        render_data
            .camera_data
            .update(projection_matrix, camera_transform);

        self.state.queue.write_buffer(
            &render_data.camera_buffer,
            0,
            bytemuck::bytes_of(render_data.camera_data.as_ref()),
        );

        Ok(())
    }

    fn update_system_data(&mut self, world: &World) -> Result<()> {
        let render_data = self
            .render_uniform_data
            .as_mut()
            .ok_or(RenderError::DataNotSet)?;

        let window_size = self.window.inner_size();
        let window_size = Vector2::new(window_size.width, window_size.height);

        render_data.system_data.screen_size = window_size;
        render_data.system_data.time = world.time().as_secs_f32();
        render_data.system_data.delta_time = world.get_delta_time().as_secs_f32();

        self.state.queue.write_buffer(
            &render_data.system_buffer,
            0,
            bytemuck::bytes_of(render_data.system_data.as_ref()),
        );

        Ok(())
    }

    fn setup_lights(&self, world: &World) -> Result<BindGroup> {
        // TODO: cache this if light data doesn't change?
        let point_lights = World::instance().get_all_components_of_type::<PointLightComponent>();
        let point_light_count = point_lights.len();

        let light_bgl = world
            .assets
            .bind_group_layouts
            .get_bind_group_layout(LIGHT_UBGL_ID)
            .ok_or(RenderError::NoLightUBGL)?;
        let point_light_buffer = self.make_point_light_buffer(point_light_count, &point_lights);
        let light_count_buffer = self.state.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Light Count Buffer"),
            contents: bytemuck::bytes_of(&(point_light_count as u32)),
            usage: BufferUsages::UNIFORM,
        });
        let light_bind_group = self.state.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Light Bind Group"),
            layout: light_bgl,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: light_count_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: point_light_buffer.as_entire_binding(),
                },
            ],
        });

        Ok(light_bind_group)
    }

    fn make_point_light_buffer(
        &self,
        point_light_count: usize,
        point_lights: &[Rc<RefCell<Box<PointLightComponent>>>],
    ) -> Buffer {
        if point_light_count == 0 {
            let dummy_point_light: PointLightUniform = PointLightUniform::default();
            self.state.device.create_buffer_init(&BufferInitDescriptor {
                label: Some("Empty Point Light Buffer"),
                usage: BufferUsages::STORAGE,
                contents: bytemuck::cast_slice(&[dummy_point_light]),
            })
        } else {
            let light_data: Vec<PointLightUniform> = point_lights
                .iter()
                .map(|m| m.borrow_mut())
                .map(|mut light| {
                    light.update_inner_pos();
                    *light.inner()
                })
                .collect();
            let light_bytes = bytemuck::cast_slice(&light_data);
            self.state.device.create_buffer_init(&BufferInitDescriptor {
                label: Some("Point Light Buffer"),
                contents: light_bytes,
                usage: BufferUsages::STORAGE,
            })
        }
    }

    fn prepare_render_pass<'a>(
        &self,
        ctx: &'a mut RenderContext,
        load_op_color: LoadOp<Color>,
        load_op_depth: LoadOp<f32>,
    ) -> RenderPass<'a> {
        ctx.encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Offscreen Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &self.offscreen_view,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: load_op_color,
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &ctx.depth_view,
                depth_ops: Some(Operations {
                    load: load_op_depth,
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..RenderPassDescriptor::default()
        })
    }

    fn default_shader_id(&self, shader_override: Option<usize>) -> Result<usize> {
        shader_override
            .or_else(|| self.current_pipeline)
            .ok_or(RenderError::NoRenderPipeline)
    }
}

fn determine_draw_over_color(shader: &RuntimeShader) -> (LoadOp<Color>, LoadOp<f32>) {
    if shader.draw_over {
        (LoadOp::Load, LoadOp::Load)
    } else {
        (LoadOp::Clear(Color::BLACK), LoadOp::Clear(1.0))
    }
}
