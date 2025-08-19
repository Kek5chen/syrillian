//! High-level renderer driving all drawing operations.
//!
//! The [`Renderer`] owns the [`State`], manages frame buffers and traverses
//! the [`World`](crate::engine::world::World) to draw all objects each frame.
//! It also provides debug drawing and post-processing utilities.

use super::error::*;
use crate::c_any_mut;
use crate::components::CameraComponent;
use crate::core::GameObjectId;
use crate::engine::assets::AssetStore;
use crate::engine::rendering::cache::AssetCache;
use crate::engine::rendering::context::DrawCtx;
use crate::engine::rendering::offscreen_surface::OffscreenSurface;
use crate::engine::rendering::post_process_pass::PostProcessData;
use crate::engine::rendering::FrameCtx;
use crate::rendering::lights::{LightType, LightUniform};
use crate::rendering::render_data::RenderUniformData;
use crate::rendering::{RenderPassType, State};
use crate::world::World;
use log::{error, trace};
use nalgebra::{Matrix4, Perspective3, Vector2};
use snafu::ResultExt;
use std::fmt::Debug;
use std::sync::{Arc, RwLock};
use syrillian_utils::debug_panic;
use wgpu::{
    Color, CommandEncoderDescriptor, LoadOp, Operations, RenderPass, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, StoreOp, SurfaceError, TextureView,
    TextureViewDescriptor,
};
use winit::dpi::PhysicalSize;
use winit::window::Window;

#[allow(dead_code)]
pub struct Renderer {
    pub state: Box<State>,
    pub window: Window,
    render_data: RenderUniformData,
    shadow_render_data: RenderUniformData,

    post_process_data: PostProcessData,
    offscreen_surface: OffscreenSurface,

    pub cache: Arc<AssetCache>,

    pub debug: DebugRenderer,

    frame_count: usize,
}

#[derive(Debug, Clone)]
pub struct DebugRenderer {
    pub mesh_edges: bool,
    pub vertex_normals: bool,
    pub rays: bool,
    pub colliders_edges: bool,
    pub text_geometry: bool,
    pub light: bool,
}

impl Default for DebugRenderer {
    fn default() -> Self {
        const DEBUG_BUILD: bool = cfg!(debug_assertions);

        DebugRenderer {
            mesh_edges: DEBUG_BUILD,
            colliders_edges: false,
            vertex_normals: false,
            rays: DEBUG_BUILD,
            text_geometry: DEBUG_BUILD,
            light: DEBUG_BUILD,
        }
    }
}

impl DebugRenderer {
    pub fn next_mode(&mut self) -> u32 {
        if self.mesh_edges && !self.vertex_normals {
            self.vertex_normals = true;
            1
        } else if self.mesh_edges {
            self.mesh_edges = false;
            self.vertex_normals = false;
            self.colliders_edges = true;
            2
        } else if self.colliders_edges {
            *self = DebugRenderer {
                mesh_edges: false,
                colliders_edges: false,
                vertex_normals: false,
                rays: false,
                text_geometry: false,
                light: false,
            };
            3
        } else {
            *self = DebugRenderer::default();
            0
        }
    }

    pub fn off(&mut self) {
        *self = DebugRenderer {
            mesh_edges: false,
            vertex_normals: false,
            rays: false,
            colliders_edges: false,
            text_geometry: false,
            light: false,
        }
    }
}

impl Renderer {
    pub fn new(window: Window, store: Arc<AssetStore>) -> Result<Self> {
        let state = Box::new(State::new(&window).context(StateErr)?);
        let offscreen_surface = OffscreenSurface::new(&state.device, &state.config);
        let cache = Arc::new(AssetCache::new(store, &state));

        // Let's heat it up :)
        let render_bgl = cache.bgl_render();
        let pp_bgl = cache.bgl_post_process();

        let render_data = RenderUniformData::empty(&state.device, &render_bgl);
        let shadow_render_data = RenderUniformData::empty(&state.device, &render_bgl);

        let post_process_data =
            PostProcessData::new(&state.device, &pp_bgl, &offscreen_surface.view());

        Ok(Renderer {
            state,
            window,
            render_data,
            shadow_render_data,
            post_process_data,
            offscreen_surface,
            cache,

            debug: DebugRenderer::default(),

            frame_count: 0,
        })
    }

    pub fn update_world(&mut self, world: &mut World) {
        if let Err(e) = self._update_world(world) {
            error!("Error when updating world drawables: {e}");
        }
    }

    pub fn _update_world(&mut self, world: &mut World) -> Result<()> {
        unsafe {
            let world_ptr = world as *mut World;
            self.traverse_and_update(&mut *world_ptr, &world.children, Matrix4::identity());
        }
        self.update_render_data(world)
    }

    pub fn render_frame(&mut self, world: &mut World) -> bool {
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
        self.shadow_pass(ctx, world);
        self.main_pass(ctx, world);
    }

    fn shadow_pass(&mut self, ctx: &mut FrameCtx, world: &mut World) {
        world.lights.update(self);

        let shadow_layers = world
            .lights
            .shadow_array(&world.assets)
            .unwrap()
            .array_layers;
        let light_count = world.lights.update_shadow_map_ids(shadow_layers);

        for layer in 0..light_count {
            let Some(light) = world.lights.light_for_layer(layer) else {
                debug_panic!("Invalid light layer");
                continue;
            };

            if light.type_id == LightType::Spot as u32 {
                self.update_shadow_camera_for_spot(light);
                self.prepare_shadow_map(world, ctx, layer);
            } else {
                // TODO: Other Light Type Shadow Maps
            }
        }
    }

    fn prepare_shadow_map(&mut self, world: &mut World, ctx: &mut FrameCtx, layer: u32) {
        let mut encoder = self
            .state
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Shadow Pass Encoder"),
            });

        let layer_view = world.lights.shadow_layer(&self.cache, layer);
        let mut pass = self.prepare_shadow_pass(&mut encoder, &layer_view);

        let render_uniform = &self.shadow_render_data.uniform;
        pass.set_bind_group(0, render_uniform.bind_group(), &[]);

        self.render_world(world, ctx, pass, RenderPassType::Shadow);

        self.state.queue.submit(Some(encoder.finish()));
    }

    fn main_pass(&mut self, ctx: &mut FrameCtx, world: &mut World) {
        let mut encoder = self
            .state
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Main Encoder"),
            });

        let mut pass = self.prepare_main_render_pass(&mut encoder, ctx);

        let render_uniform = &self.render_data.uniform;
        pass.set_bind_group(0, render_uniform.bind_group(), &[]);

        self.render_world(world, ctx, pass, RenderPassType::Color);

        self.state.queue.submit(Some(encoder.finish()));
    }

    fn render_world(
        &self,
        world: &mut World,
        frame_ctx: &FrameCtx,
        mut pass: RenderPass,
        pass_type: RenderPassType,
    ) {
        let light_uniform = world.lights.uniform();

        pass.set_bind_group(3, light_uniform.bind_group(), &[]);

        if pass_type == RenderPassType::Color {
            let shadow_uniform = world.lights.shadow_uniform();
            pass.set_bind_group(4, shadow_uniform.bind_group(), &[]);
        }

        let draw_ctx = DrawCtx {
            frame: frame_ctx,
            pass: RwLock::new(pass),
            pass_type,
        };

        self.traverse_and_render(world, &world.children, &draw_ctx);

        world.execute_component_func(|comp, world| comp.draw(world, &draw_ctx));

        #[cfg(debug_assertions)]
        if self.debug.light {
            world.lights.draw_debug_lights(&draw_ctx);
        }

        drop(draw_ctx);
    }

    fn traverse_and_update(
        &self,
        world: &mut World,
        children: &[GameObjectId],
        parent_world: Matrix4<f32>,
    ) {
        for child in children {
            let child_world = parent_world * child.transform.full_matrix().matrix();

            for comp in child.components.iter().copied() {
                if let Some(comp) = c_any_mut!(comp) {
                    comp.update_draw(world, self, &child_world);
                }
            }

            if let Some(drawable) = &mut child.clone().drawable {
                drawable.update(world, child.clone(), &self, &child_world);
            };

            if !child.children.is_empty() {
                self.traverse_and_update(
                    world,
                    &child.children,
                    child_world,
                );
            }

        }
    }

    fn traverse_and_render(&self, world: &World, children: &[GameObjectId], ctx: &DrawCtx) {
        for child in children {
            if !child.children.is_empty() {
                self.traverse_and_render(world, &child.children, ctx);
            }

            let Some(drawable) = &mut child.clone().drawable else {
                continue;
            };

            drawable.draw(world, ctx);
        }
    }

    fn end_render(&mut self, world: &mut World, mut ctx: FrameCtx) {
        self.render_final_pass(world, &mut ctx);

        ctx.output.present();

        if self.cache.last_refresh().elapsed().as_secs_f32() > 5.0 {
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
        pass.set_pipeline(&post_shader.solid_pipeline());
        pass.set_bind_group(0, self.render_data.uniform.bind_group(), &[]);
        pass.set_bind_group(1, self.post_process_data.uniform.bind_group(), &[]);
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

    fn update_render_data(&mut self, world: &mut World) -> Result<()> {
        self.update_view_camera_data(world)?;
        self.update_system_data(world);
        world.lights.update(self);

        Ok(())
    }

    fn update_view_camera_data(&mut self, world: &World) -> Result<()> {
        let camera_rc = world
            .active_camera
            .as_ref()
            .ok_or(RenderError::NoCameraSet)?;

        let camera = camera_rc;
        let camera_comp = camera
            .get_component::<CameraComponent>()
            .ok_or(RenderError::NoCameraComponentSet)?;

        let projection_matrix = camera_comp.projection.as_matrix();
        let camera_transform = &camera.transform;

        self.render_data
            .camera_data
            .update_with_transform(projection_matrix, camera_transform);
        self.render_data.upload_camera_data(&self.state.queue);

        Ok(())
    }

    fn update_shadow_camera_for_spot(&mut self, light: &LightUniform) {
        let fovy = (2.0 * light.outer_angle).clamp(0.0175, 3.12);
        let near = 0.05_f32;
        let far = light.range.max(near + 0.01);
        let proj = Perspective3::new(1.0, fovy, near, far);

        self.shadow_render_data.camera_data.update(
            &proj.as_matrix(),
            &light.position,
            &light.view_mat,
        );
        self.shadow_render_data
            .upload_camera_data(&self.state.queue);
    }

    fn update_system_data(&mut self, world: &World) {
        let window_size = self.window.inner_size();
        let window_size = Vector2::new(window_size.width, window_size.height);

        let system_data = &mut self.render_data.system_data;
        system_data.screen_size = window_size;
        system_data.time = world.time().as_secs_f32();
        system_data.delta_time = world.delta_time().as_secs_f32();

        self.render_data.upload_system_data(&self.state.queue);
    }

    fn prepare_shadow_pass<'a>(
        &self,
        encoder: &'a mut wgpu::CommandEncoder,
        shadow_map: &TextureView,
    ) -> RenderPass<'a> {
        encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Shadow Pass"),
            color_attachments: &[],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &shadow_map,
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(1.0),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..RenderPassDescriptor::default()
        })
    }

    fn prepare_main_render_pass<'a>(
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
                    store: StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            ..RenderPassDescriptor::default()
        })
    }
}
