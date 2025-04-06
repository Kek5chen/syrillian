use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;

use log::{debug, error};
use nalgebra::{Matrix4, Perspective3};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::*;
use winit::window::Window;
use crate::asset_management::bindgroup_layout_manager::{CAMERA_UBGL_ID, POST_PROCESS_BGL_ID};
use crate::asset_management::shadermanager::{ShaderId, DEBUG_EDGES_SHADER_ID, DIM3_SHADER_ID, FALLBACK_SHADER_ID, POST_PROCESS_SHADER_ID};
use crate::components::camera::CameraData;
use crate::components::CameraComp;
use crate::object::GameObjectId;
use crate::state::State;
use crate::world::World;

struct PostProcessPass {
    bind_group: BindGroup,
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
            ..Default::default()
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
                }
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

pub struct CameraRenderData {
    camera_uniform_data: Box<CameraData>,
    camera_uniform_buffer: Buffer,
    camera_uniform_bind_group: BindGroup,
}

#[derive(Debug, Default)]
pub struct DebugRenderer {
    pub draw_edges: bool,
}

#[allow(dead_code)]
pub struct Renderer {
    pub(crate) state: Box<State>,
    window: Window,
    current_pipeline: Option<ShaderId>,
    camera_render_data: Option<CameraRenderData>,

    // Offscreen texture for rendering the scene before post-processing
    offscreen_texture: Texture,
    offscreen_view: TextureView,

    post_process_pass: Option<PostProcessPass>,

    debug: DebugRenderer,
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

    pub(crate) async fn new(window: Window) -> Self {
        let state = Box::new(State::new(&window).await);

        let (offscreen_texture, offscreen_view) = Self::create_offscreen_texture(&state.device, state.config.width, state.config.height, state.config.format);

        Renderer {
            state,
            window,
            current_pipeline: None,
            camera_render_data: None,
            offscreen_texture,
            offscreen_view,
            post_process_pass: None,
            debug: DebugRenderer::default(),
        }
    }

    fn create_offscreen_texture(device: &Device, width: u32, height: u32, format: TextureFormat) -> (Texture, TextureView) {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("Offscreen Texture"),
            size: Extent3d { width, height, depth_or_array_layers: 1 },
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

        let camera_data = Box::new(CameraData::empty());
        let camera_bgl = World::instance().assets.bind_group_layouts.get_bind_group_layout(CAMERA_UBGL_ID).unwrap();
        let (camera_uniform_buffer, camera_uniform_bind_group) = Self::create_uniform_init(
            camera_bgl,
            &self.state,
            bytemuck::cast_slice(&[*camera_data]),
        );
        self.camera_render_data = Some(CameraRenderData {
            camera_uniform_data: camera_data,
            camera_uniform_buffer,
            camera_uniform_bind_group,
        });

        let world = World::instance();
        let post_bgl = world.assets.bind_group_layouts.get_bind_group_layout(POST_PROCESS_BGL_ID).unwrap();
        self.post_process_pass = Some(PostProcessPass::new(
            &self.state.device,
            post_bgl,
            &self.offscreen_view
        ));

        self.debug.draw_edges = true;
    }

    pub fn render_world(&mut self, world: &mut World) -> bool {
        let mut ctx = match self.begin_render() {
            Ok(ctx) => ctx,
            Err(SurfaceError::Lost) => {
                self.state.resize(self.state.size);
                return false;
            }
            Err(SurfaceError::OutOfMemory) => {
                error!("The application ran out of GPU memory!");
                return false;
            }
            Err(e) => {
                error!("Surface error: {:?}", e);
                return false;
            }
        };

        self.render(&mut ctx, world, None);

        if self.debug.draw_edges {
            self.render(&mut ctx, world, Some(DEBUG_EDGES_SHADER_ID));
        }

        self.end_render(world, ctx);

        true
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.state.resize(new_size);

        // Re-create the offscreen texture and post process bind group after resize
        let (new_offscreen, new_offscreen_view) = Self::create_offscreen_texture(
            &self.state.device,
            self.state.config.width,
            self.state.config.height,
            self.state.config.format
        );
        self.offscreen_texture = new_offscreen;
        self.offscreen_view = new_offscreen_view;

        if let Some(pp) = &mut self.post_process_pass {
            let world = World::instance();
            let post_bgl = world.assets.bind_group_layouts.get_bind_group_layout(POST_PROCESS_BGL_ID).unwrap();
            *pp = PostProcessPass::new(&self.state.device, post_bgl, &self.offscreen_view);
        }
    }

    fn begin_render(&mut self) -> Result<RenderContext, SurfaceError> {
        let mut output = self.state.surface.get_current_texture()?;
        if output.suboptimal {
            drop(output);
            self.state.recreate_surface();
            let st = match self.state.surface.get_current_texture() {
                Ok(st) => st,
                Err(e) => {
                    return Err(e);
                }
            };
            output = st;
        }

        let color_view = output
            .texture
            .create_view(&TextureViewDescriptor::default());
        let depth_view = self
            .state
            .depth_texture
            .create_view(&TextureViewDescriptor::default());
        let encoder = self.state.device.create_command_encoder(&CommandEncoderDescriptor {
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

    fn render(&mut self, ctx: &mut RenderContext, world: &mut World, shader_override: Option<ShaderId>) {
        if world.active_camera.is_none() {
            debug!("No camera active");
            return;
        }

        if self.current_pipeline.is_none() {
            debug!("No pipeline active");
            return;
        }

        let current_pipeline = self.current_pipeline.unwrap();

        let render_data = self
            .camera_render_data
            .as_mut()
            .expect("Camera render data should be initialized");

        let world_ptr: *mut World = world;
        let camera_rc = unsafe { (*world_ptr).active_camera.as_ref().unwrap() };

        let camera = camera_rc;
        let camera_comp: Option<Rc<RefCell<Box<CameraComp>>>> =
            camera.get_component::<CameraComp>();
        if camera_comp.is_none() {
            debug!("Camera didn't have a camera component");
            return;
        }

        let camera_comp = camera_comp.unwrap();
        let projection_matrix: &Perspective3<f32> = &camera_comp.borrow_mut().projection;
        let camera_transform = &camera.transform;
        render_data
            .camera_uniform_data
            .update(projection_matrix, camera_transform);
        self.state.queue.write_buffer(
            &render_data.camera_uniform_buffer,
            0,
            bytemuck::cast_slice(&[*render_data.camera_uniform_data]),
        );

        let shader_id = shader_override.unwrap_or(current_pipeline);

        let shader = world
                .assets
                .shaders
                .get_shader(shader_id)
                .expect("3D Pipeline should've been initialized previously");

        let (load_op_color, load_op_depth) = match shader.draw_over {
            true => (LoadOp::Load, LoadOp::Load),
            false => (LoadOp::Clear(Color::BLACK), LoadOp::Clear(1.0)),
        };

        let mut rpass = ctx.encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Offscreen Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &self.offscreen_view,
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
            ..Default::default()
        });

        rpass.set_pipeline(&shader.pipeline);
        rpass.set_bind_group(0, &render_data.camera_uniform_bind_group, &[]);

        unsafe {
            self.traverse_and_render(
                &mut *world_ptr,
                &mut rpass,
                &world.children,
                Matrix4::identity(),
            );
        }
    }

    unsafe fn traverse_and_render(
        &self,
        world: &mut World,
        rpass: &mut RenderPass,
        children: &[GameObjectId],
        combined_matrix: Matrix4<f32>,
    ) {
        let world_ptr: *mut World = world;
        for child in children {
            if !child.children.is_empty() {
                unsafe {
                    self.traverse_and_render(
                        &mut *world_ptr,
                        rpass,
                        &child.children,
                        combined_matrix * child.transform.full_matrix().to_homogeneous(),
                    );
                }
            }
            if let Some(drawable) = &mut child.clone().drawable {
                {
                    drawable.update(&mut *world_ptr, *child, &self.state.queue, &combined_matrix);
                }
                {
                    let rpass_ptr: *mut RenderPass = rpass;
                    drawable.draw(&mut *world_ptr, &mut *rpass_ptr);
                }
            }
        }
    }

    fn render_final_pass(&mut self, world: &mut World, ctx: &mut RenderContext) {
        let mut rpass = ctx.encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("PostProcess Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &ctx.color_view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::BLACK),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });

        let post_shader = world
            .assets
            .shaders
            .get_shader(POST_PROCESS_SHADER_ID)
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
}
