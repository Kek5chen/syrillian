//! High-level renderer driving all drawing operations.
//!
//! The [`Renderer`] owns the [`State`], manages frame buffers and traverses
//! the all Scene Proxies it gets from the world to draw the latest snapshots of all world objects each frame.
//! It also provides debug drawing and post-processing utilities.

use super::error::*;
use crate::components::TypedComponentId;
use crate::engine::assets::AssetStore;
use crate::engine::rendering::FrameCtx;
use crate::engine::rendering::cache::AssetCache;
use crate::engine::rendering::offscreen_surface::OffscreenSurface;
use crate::engine::rendering::post_process_pass::PostProcessData;
use crate::rendering::light_manager::LightManager;
use crate::rendering::lights::LightType;
use crate::rendering::message::RenderMsg;
use crate::rendering::proxies::SceneProxyBinding;
use crate::rendering::render_data::RenderUniformData;
use crate::rendering::{GPUDrawCtx, RenderPassType, State};
use itertools::Itertools;
use log::{error, trace};
use nalgebra::Vector2;
use snafu::ResultExt;
use std::collections::HashMap;
use std::mem::swap;
use std::sync::{Arc, RwLock, mpsc};
use syrillian_utils::debug_panic;
use web_time::{Duration, Instant};
use wgpu::{
    Color, CommandEncoderDescriptor, LoadOp, Operations, RenderPass, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, StoreOp, SurfaceError, TextureView,
    TextureViewDescriptor,
};
use winit::dpi::PhysicalSize;
use winit::window::Window;

#[cfg(debug_assertions)]
use crate::rendering::DebugRenderer;

#[allow(dead_code)]
pub struct Renderer {
    pub state: Box<State>,
    pub window: Window,
    render_data: RenderUniformData,
    shadow_render_data: RenderUniformData,

    post_process_data: PostProcessData,
    offscreen_surface: OffscreenSurface,

    pub cache: AssetCache,

    game_rx: mpsc::Receiver<RenderMsg>,
    proxies: HashMap<TypedComponentId, SceneProxyBinding>,
    sorted_proxies: Vec<TypedComponentId>,
    pub(super) lights: LightManager,
    skybox_background_color: Color,

    start_time: Instant,
    delta_time: Duration,
    last_frame_time: Instant,

    frame_count: usize,
}

impl Renderer {
    pub fn new(
        game_rx: mpsc::Receiver<RenderMsg>,
        window: Window,
        store: Arc<AssetStore>,
    ) -> Result<Self> {
        let state = Box::new(State::new(&window).context(StateErr)?);
        let offscreen_surface = OffscreenSurface::new(&state.device, &state.config);
        let cache = AssetCache::new(store, &state);

        // Let's heat it up :)
        let render_bgl = cache.bgl_render();
        let pp_bgl = cache.bgl_post_process();

        let render_data = RenderUniformData::empty(&state.device, &render_bgl);
        let shadow_render_data = RenderUniformData::empty(&state.device, &render_bgl);

        let post_process_data =
            PostProcessData::new(&state.device, &pp_bgl, offscreen_surface.view());

        let lights = LightManager::new(&cache, &state.device);

        Ok(Renderer {
            state,
            window,
            render_data,
            shadow_render_data,
            post_process_data,
            offscreen_surface,
            cache,

            game_rx,
            proxies: HashMap::new(),
            sorted_proxies: Vec::new(),
            lights,
            skybox_background_color: Color::BLACK,

            start_time: Instant::now(),
            delta_time: Duration::default(),
            last_frame_time: Instant::now(),

            frame_count: 0,
        })
    }

    pub fn handle_message(&mut self, msg: RenderMsg) {
        match msg {
            RenderMsg::RegisterProxy(cid, mut proxy, local_to_world) => {
                trace!("Registered Proxy for #{:?}", cid.0);
                let data = proxy.setup_render(self, local_to_world.matrix());
                let binding = SceneProxyBinding::new(cid, local_to_world, data, proxy);
                self.proxies.insert(cid, binding);
            }
            RenderMsg::RegisterLightProxy(cid, proxy) => {
                trace!("Registered Light Proxy for #{:?}", cid.0);
                self.lights.add_proxy(cid, *proxy);
            }
            RenderMsg::RemoveProxy(cid) => {
                self.proxies.remove(&cid);
                self.lights.remove_proxy(cid);
            }
            RenderMsg::UpdateTransform(cid, ltw) => {
                if let Some(cid) = self.proxies.get_mut(&cid) {
                    cid.update_transform(ltw);
                }
            }
            RenderMsg::ProxyUpdate(cid, command) => {
                if let Some(binding) = self.proxies.get_mut(&cid) {
                    command(binding.proxy.as_mut());
                }
            }
            RenderMsg::LightProxyUpdate(cid, command) => {
                self.lights.execute_light_command(cid, command);
            }
            RenderMsg::UpdateActiveCamera(camera_data) => {
                camera_data(&mut self.render_data.camera_data);
                self.update_view_camera_data();
            }
            RenderMsg::ProxyState(cid, enabled) => {
                if let Some(binding) = self.proxies.get_mut(&cid) {
                    binding.enabled = enabled;
                }
            }
            RenderMsg::SetSkyboxBackgroundColor(color) => {
                self.set_skybox_background_color(color);
            }
            RenderMsg::CommandBatch(batch) => {
                for message in batch {
                    self.handle_message(message);
                }
            }
        }
    }

    pub fn handle_events(&mut self) {
        loop {
            let Ok(msg) = self.game_rx.try_recv() else {
                break;
            };
            self.handle_message(msg)
        }
    }

    pub fn update(&mut self) {
        let mut proxies = HashMap::new();
        swap(&mut self.proxies, &mut proxies);

        for proxy in proxies.values_mut() {
            proxy.update(self, self.window());
        }

        self.proxies = proxies;
        self.resort_proxies();
        self.update_render_data();
    }

    pub fn render_frame(&mut self) -> bool {
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

        self.render(&mut ctx);
        self.end_render(ctx);

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
        })
    }

    fn render(&mut self, ctx: &mut FrameCtx) {
        self.shadow_pass(ctx);
        self.main_pass(ctx);
    }

    fn shadow_pass(&mut self, ctx: &mut FrameCtx) {
        self.lights
            .update(&self.cache, &self.state.queue, &self.state.device);

        let shadow_layers = self
            .lights
            .shadow_array(self.cache.textures.store())
            .unwrap()
            .array_layers;
        let light_count = self.lights.update_shadow_map_ids(shadow_layers);

        for layer in 0..light_count {
            let Some(light) = self.lights.light_for_layer(layer) else {
                debug_panic!("Invalid light layer");
                continue;
            };

            if light.type_id == LightType::Spot as u32 {
                self.shadow_render_data
                    .update_shadow_camera_for_spot(light, &self.state.queue);
                self.prepare_shadow_map(ctx, layer);
            } else {
                // TODO: Other Light Type Shadow Maps
            }
        }
    }

    fn prepare_shadow_map(&mut self, ctx: &mut FrameCtx, layer: u32) {
        let mut encoder = self
            .state
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Shadow Pass Encoder"),
            });

        let layer_view = self.lights.shadow_layer(&self.cache, layer);
        let mut pass = self.prepare_shadow_pass(&mut encoder, &layer_view);

        let render_uniform = &self.shadow_render_data.uniform;
        pass.set_bind_group(0, render_uniform.bind_group(), &[]);

        self.render_scene(ctx, pass, RenderPassType::Shadow);

        self.state.queue.submit(Some(encoder.finish()));
    }

    fn main_pass(&mut self, ctx: &mut FrameCtx) {
        let mut encoder = self
            .state
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Main Encoder"),
            });

        let mut pass = self.prepare_main_render_pass(&mut encoder, ctx);

        let render_uniform = &self.render_data.uniform;
        pass.set_bind_group(0, render_uniform.bind_group(), &[]);

        self.render_scene(ctx, pass, RenderPassType::Color);

        self.state.queue.submit(Some(encoder.finish()));
    }

    fn render_scene(&self, frame_ctx: &FrameCtx, mut pass: RenderPass, pass_type: RenderPassType) {
        let light_uniform = self.lights.uniform();

        pass.set_bind_group(3, light_uniform.bind_group(), &[]);

        if pass_type == RenderPassType::Color {
            let shadow_uniform = self.lights.shadow_uniform();
            pass.set_bind_group(4, shadow_uniform.bind_group(), &[]);
        }

        let draw_ctx = GPUDrawCtx {
            frame: frame_ctx,
            pass: RwLock::new(pass),
            pass_type,
        };

        self.render_proxies(&draw_ctx);

        #[cfg(debug_assertions)]
        if DebugRenderer::light() {
            self.lights.render_debug_lights(self, &draw_ctx);
        }
    }

    fn resort_proxies(&mut self) {
        self.sorted_proxies.clear();
        self.sorted_proxies.extend(
            self.proxies
                .iter()
                .filter(|(_, p)| p.enabled)
                .sorted_by_key(|(_, proxy)| proxy.proxy.priority(self.cache.store()))
                .map(|(tid, _)| *tid),
        );
    }

    fn render_proxies(&self, ctx: &GPUDrawCtx) {
        for proxy in self
            .sorted_proxies
            .iter()
            .map(|ctid| self.proxies.get(ctid))
        {
            let Some(proxy) = proxy else {
                debug_panic!("Sorted proxy not in proxy list");
                continue;
            };
            proxy.render(self, ctx);
        }
    }

    fn end_render(&mut self, mut ctx: FrameCtx) {
        self.render_final_pass(&mut ctx);

        self.window.pre_present_notify();

        ctx.output.present();

        self.tick_delta_time();

        if self.cache.last_refresh().elapsed().as_secs_f32() > 5.0 {
            trace!("Refreshing cache...");
            let refreshed_count = self.cache.refresh_all();
            if cfg!(debug_assertions) && refreshed_count != 0 {
                trace!("Refreshed cache elements {}", refreshed_count);
            }
        }
    }

    fn render_final_pass(&mut self, ctx: &mut FrameCtx) {
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
        pass.set_pipeline(post_shader.solid_pipeline());
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

    fn skybox_background_color(&self) -> Color {
        self.skybox_background_color
    }

    pub fn set_skybox_background_color(&mut self, color: Color) {
        self.skybox_background_color = color;
    }

    fn update_render_data(&mut self) {
        self.update_system_data();
        self.lights
            .update(&self.cache, &self.state.queue, &self.state.device);
    }

    fn update_view_camera_data(&mut self) {
        self.render_data.upload_camera_data(&self.state.queue);
    }

    fn update_system_data(&mut self) {
        let window_size = self.window.inner_size();
        let window_size = Vector2::new(window_size.width, window_size.height);

        let system_data = &mut self.render_data.system_data;
        system_data.screen_size = window_size;
        system_data.time = self.start_time.elapsed().as_secs_f32();
        system_data.delta_time = self.delta_time.as_secs_f32();

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
                view: shadow_map,
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
        let default_color = self.skybox_background_color();

        encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Offscreen Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: self.offscreen_surface.view(),
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(default_color),
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

    /// Updates the delta time based on the elapsed time since the last frame
    pub fn tick_delta_time(&mut self) {
        self.delta_time = self.last_frame_time.elapsed();
        self.last_frame_time = Instant::now();
    }

    pub fn last_frame_time(&self) -> Instant {
        self.last_frame_time
    }

    /// Returns the time elapsed since the last frame
    pub fn delta_time(&self) -> Duration {
        self.delta_time
    }

    /// Returns the instant in time when the world was created
    pub fn start_time(&self) -> Instant {
        self.start_time
    }

    /// Returns the total time elapsed since the world was created
    pub fn time(&self) -> Duration {
        self.start_time.elapsed()
    }
}
