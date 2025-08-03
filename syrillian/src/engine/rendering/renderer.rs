//! High level renderer driving all drawing operations.
//!
//! The [`Renderer`] owns the [`State`], manages frame buffers and traverses
//! the [`World`](crate::engine::world::World) to draw all objects each frame.
//! It also provides debug drawing and post-processing utilities.

use super::error::*;
use crate::components::{CameraComponent, CameraUniform, PointLightComponent, PointLightUniform};
use crate::core::GameObjectId;
use crate::engine::assets::AssetStore;
use crate::engine::rendering::cache::AssetCache;
use crate::engine::rendering::context::DrawCtx;
use crate::engine::rendering::offscreen_surface::OffscreenSurface;
use crate::engine::rendering::post_process_pass::PostProcessData;
use crate::engine::rendering::uniform::ShaderUniform;
use crate::engine::rendering::FrameCtx;
use crate::ensure_aligned;
use crate::rendering::State;
use crate::world::World;
use log::{error, trace};
use nalgebra::{Matrix4, Perspective3, Vector2};
use snafu::ResultExt;
use std::fmt::Debug;
use std::sync::{Arc, RwLock};
use syrillian_macros::UniformIndex;
use wgpu::{
    Color, CommandEncoderDescriptor, LoadOp, Operations, RenderPass, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, StoreOp, SurfaceError,
    TextureViewDescriptor,
};
use winit::dpi::PhysicalSize;
use winit::window::Window;

#[allow(dead_code)]
pub struct Renderer {
    pub state: Box<State>,
    pub window: Window,
    render_uniform_data: RenderUniformData,

    post_process_data: PostProcessData,
    offscreen_surface: OffscreenSurface,

    pub cache: Arc<AssetCache>,

    pub debug: DebugRenderer,

    frame_count: usize,
    printed_errors: u32,
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
    system_data: Box<SystemUniform>,
    uniform: ShaderUniform<RenderUniformIndex>,
}

#[derive(Debug, Clone)]
pub struct DebugRenderer {
    pub draw_edges: bool,
    pub draw_vertex_normals: bool,
}

impl Default for DebugRenderer {
    fn default() -> Self {
        const DEBUG_BUILD: bool = cfg!(debug_assertions);

        DebugRenderer {
            draw_edges: DEBUG_BUILD,
            draw_vertex_normals: DEBUG_BUILD,
        }
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, UniformIndex)]
pub enum RenderUniformIndex {
    Camera = 0,
    System = 1,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, UniformIndex)]
pub enum PointLightUniformIndex {
    Count = 0,
    Lights = 1,
}

impl Renderer {
    pub(crate) async fn new(window: Window, store: Arc<AssetStore>) -> Result<Self> {
        let state = Box::new(State::new(&window).await.context(StateErr)?);
        let offscreen_surface = OffscreenSurface::new(&state.device, &state.config);
        let cache = Arc::new(AssetCache::new(store, &state));

        // Let's heat it up :)
        let render_bgl = cache.bgl_render();
        let pp_bgl = cache.bgl_post_process();

        let camera_data = Box::<CameraUniform>::default();
        let system_data = Box::<SystemUniform>::default();

        let render_uniform = ShaderUniform::<RenderUniformIndex>::builder(&render_bgl)
            .with_buffer_data(camera_data.as_ref())
            .with_buffer_data(system_data.as_ref())
            .build(&state.device);

        let render_uniform_data = RenderUniformData {
            camera_data,
            system_data,
            uniform: render_uniform,
        };

        let post_process_data =
            PostProcessData::new(&state.device, &pp_bgl, &offscreen_surface.view());

        drop(render_bgl);
        drop(pp_bgl);

        Ok(Renderer {
            state,
            window,
            render_uniform_data,
            post_process_data,
            offscreen_surface,
            cache,

            debug: DebugRenderer::default(),

            frame_count: 0,
            printed_errors: 0,
        })
    }

    pub fn init(&mut self) {}

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

        self.render(&mut ctx, world);

        self.end_render(world, ctx);

        true
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.state.resize(new_size);

        let pp_bgl = self.cache.bgl_post_process();

        self.offscreen_surface
            .recreate(&self.state.device, &self.state.config);
        self.post_process_data =
            PostProcessData::new(&self.state.device, &pp_bgl, self.offscreen_surface.view());
    }

    fn begin_render(&mut self) -> Result<FrameCtx> {
        self.frame_count += 1;

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

        Ok(FrameCtx {
            output,
            color_view,
            depth_view,
            cache: self.cache.clone(),

            #[cfg(debug_assertions)]
            debug: self.debug.clone(),
        })
    }

    fn render(&mut self, ctx: &mut FrameCtx, world: &mut World) {
        if let Err(e) = self.render_inner(ctx, world) {
            if self.printed_errors < 5 {
                self.printed_errors += 1;
                error!("{e}")
            }
            return;
        }
        self.printed_errors = 0;
    }

    fn render_inner(&mut self, ctx: &mut FrameCtx, world: &mut World) -> Result<()> {
        self.update_render_data(world)?;

        let light_uniform = self.setup_lights(world)?;

        let mut encoder = self
            .state
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Main Encoder"),
            });

        let mut pass = self.prepare_render_pass(&mut encoder, ctx);

        let render_uniform = &self.render_uniform_data.uniform;

        pass.set_bind_group(0, render_uniform.bind_group(), &[]);
        pass.set_bind_group(3, light_uniform.bind_group(), &[]);

        let draw_ctx = DrawCtx {
            frame: ctx,
            pass: RwLock::new(pass),
        };

        let world_ptr = world as *mut World;
        unsafe {
            self.traverse_and_render(
                &mut *world_ptr,
                &world.children,
                Matrix4::identity(),
                &draw_ctx,
            );
        }

        drop(draw_ctx);

        self.state.queue.submit(Some(encoder.finish()));

        Ok(())
    }

    fn traverse_and_render(
        &self,
        world: &mut World,
        children: &[GameObjectId],
        combined_matrix: Matrix4<f32>,
        ctx: &DrawCtx,
    ) {
        for child in children {
            if !child.children.is_empty() {
                self.traverse_and_render(
                    world,
                    &child.children,
                    combined_matrix * child.transform.full_matrix().to_homogeneous(),
                    ctx,
                );
            }
            let Some(drawable) = &mut child.clone().drawable else {
                continue;
            };

            drawable.update(world, *child, self, &combined_matrix);
            drawable.draw(world, ctx);
        }
    }

    fn end_render(&mut self, world: &mut World, mut ctx: FrameCtx) {
        self.render_final_pass(world, &mut ctx);

        ctx.output.present();
        self.window.request_redraw();

        if self.frame_count % 1000 == 0 {
            trace!("Refreshing cache...");
            let refreshed_count = self.cache.refresh_all();
            if cfg!(debug_assertions) && refreshed_count != 0 {
                trace!("Refreshed cache elements {}", refreshed_count);
            }
        }
    }

    fn render_final_pass(&mut self, _world: &mut World, ctx: &mut FrameCtx) {
        let mut encoder = self
            .state
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Final Pass Copy Encoder"),
            });
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Post Process Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &ctx.color_view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::BLACK),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..RenderPassDescriptor::default()
        });

        let post_shader = self.cache.shader_post_process();
        pass.set_pipeline(&post_shader.pipeline);
        pass.set_bind_group(0, self.post_process_data.uniform.bind_group(), &[]);
        pass.draw(0..6, 0..1);

        drop(pass);

        self.state.queue.submit(Some(encoder.finish()));
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

        let render_data = &mut self.render_uniform_data;

        render_data
            .camera_data
            .update(projection_matrix, camera_transform);

        self.state.queue.write_buffer(
            &render_data.uniform.buffer(RenderUniformIndex::Camera),
            0,
            bytemuck::bytes_of(render_data.camera_data.as_ref()),
        );

        Ok(())
    }

    fn update_system_data(&mut self, world: &World) -> Result<()> {
        let window_size = self.window.inner_size();
        let window_size = Vector2::new(window_size.width, window_size.height);

        let render_data = &mut self.render_uniform_data;
        render_data.system_data.screen_size = window_size;
        render_data.system_data.time = world.time().as_secs_f32();
        render_data.system_data.delta_time = world.delta_time().as_secs_f32();

        self.state.queue.write_buffer(
            &render_data.uniform.buffer(RenderUniformIndex::System),
            0,
            bytemuck::bytes_of(render_data.system_data.as_ref()),
        );

        Ok(())
    }

    fn setup_lights(&self, world: &World) -> Result<ShaderUniform<PointLightUniformIndex>> {
        // TODO: cache this if light data doesn't change?
        let point_lights = world.get_all_components_of_type::<PointLightComponent>();
        let point_light_count = point_lights.len() as u32;

        let light_bgl = self.cache.bgl_light();

        const DUMMY_POINT_LIGHT: PointLightUniform = PointLightUniform::zero();

        let builder = ShaderUniform::<PointLightUniformIndex>::builder(&light_bgl)
            .with_buffer_data(&point_light_count);

        let uniform;

        if point_light_count == 0 {
            uniform = builder
                .with_buffer_storage(&[DUMMY_POINT_LIGHT])
                .build(&self.state.device);
        } else {
            let light_data: Vec<PointLightUniform> = point_lights
                .iter()
                .map(|m| m.borrow_mut())
                .map(|mut light| {
                    light.update_inner_pos();
                    *light.inner()
                })
                .collect();

            uniform = builder
                .with_buffer_storage(light_data.as_slice())
                .build(&self.state.device);
        };

        Ok(uniform)
    }

    fn prepare_render_pass<'a>(
        &self,
        encoder: &'a mut wgpu::CommandEncoder,
        ctx: &mut FrameCtx,
    ) -> RenderPass<'a> {
        encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Offscreen Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: self.offscreen_surface.view(),
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::BLACK),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &ctx.depth_view,
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(1.0),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..RenderPassDescriptor::default()
        })
    }
}
