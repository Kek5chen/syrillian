//! High-level renderer driving all drawing operations.
//!
//! The [`Renderer`] owns the [`State`], manages frame buffers and traverses
//! the all Scene Proxies it gets from the world to draw the latest snapshots of all world objects each frame.
//! It also provides debug drawing and post-processing utilities.

use super::error::*;
use crate::components::TypedComponentId;
use crate::core::BoundingSphere;
use crate::engine::assets::{AssetStore, HTexture};
use crate::engine::rendering::FrameCtx;
use crate::engine::rendering::cache::{AssetCache, GpuTexture};
use crate::engine::rendering::offscreen_surface::OffscreenSurface;
use crate::engine::rendering::post_process_pass::PostProcessData;
use crate::game_thread::RenderTargetId;
#[cfg(debug_assertions)]
use crate::rendering::DebugRenderer;
use crate::rendering::light_manager::LightManager;
use crate::rendering::lights::LightType;
use crate::rendering::message::RenderMsg;
use crate::rendering::picking::{PickRequest, PickResult, color_bytes_to_hash};
use crate::rendering::proxies::{PROXY_PRIORITY_2D, SceneProxyBinding};
use crate::rendering::render_data::RenderUniformData;
use crate::rendering::texture_export::{TextureExportError, save_texture_to_png};
use crate::rendering::{GPUDrawCtx, RenderPassType, State};
use crossbeam_channel::{Receiver, Sender};
use itertools::Itertools;
use log::{error, trace, warn};
use nalgebra::{Matrix4, Vector2, Vector3, Vector4};
use snafu::ResultExt;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::mem;
use std::sync::{Arc, RwLock};
use syrillian_utils::debug_panic;
use web_time::{Duration, Instant};
use wgpu::*;
use winit::dpi::PhysicalSize;
use winit::window::{Window, WindowId};

pub const PICKING_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;
const PICKING_ROW_PITCH: u32 = 256;

struct PickingSurface {
    texture: Texture,
    view: TextureView,
}

impl PickingSurface {
    fn new(device: &Device, config: &SurfaceConfiguration) -> Self {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("Picking Texture"),
            size: Extent3d {
                width: config.width.max(1),
                height: config.height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: PICKING_TEXTURE_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&TextureViewDescriptor::default());

        Self { texture, view }
    }

    fn recreate(&mut self, device: &Device, config: &SurfaceConfiguration) {
        *self = Self::new(device, config);
    }

    fn view(&self) -> &TextureView {
        &self.view
    }

    fn texture(&self) -> &Texture {
        &self.texture
    }
}

pub struct RenderViewport {
    window: Window,
    surface: Surface<'static>,
    config: SurfaceConfiguration,
    depth_texture: Texture,
    offscreen_surface: OffscreenSurface,
    picking_surface: PickingSurface,
    post_process_data: PostProcessData,
    render_data: RenderUniformData,
    start_time: Instant,
    delta_time: Duration,
    last_frame_time: Instant,
    frame_count: usize,
}

impl RenderViewport {
    fn new(
        window: Window,
        surface: Surface<'static>,
        mut config: SurfaceConfiguration,
        state: &State,
        cache: &AssetCache,
    ) -> Self {
        Self::clamp_config(&mut config);
        surface.configure(&state.device, &config);

        let render_bgl = cache.bgl_render();
        let pp_bgl = cache.bgl_post_process();

        let offscreen_surface = OffscreenSurface::new(&state.device, &config);
        let depth_texture = Self::create_depth_texture(&state.device, &config);
        let depth_view = depth_texture.create_view(&TextureViewDescriptor::default());
        let picking_surface = PickingSurface::new(&state.device, &config);

        let post_process_data = PostProcessData::new(
            &state.device,
            &pp_bgl,
            offscreen_surface.view(),
            &depth_view,
        );

        let render_data = RenderUniformData::empty(&state.device, &render_bgl);

        RenderViewport {
            window,
            surface,
            config,
            depth_texture,
            offscreen_surface,
            picking_surface,
            post_process_data,
            render_data,
            start_time: Instant::now(),
            delta_time: Duration::default(),
            last_frame_time: Instant::now(),
            frame_count: 0,
        }
    }

    fn clamp_config(config: &mut SurfaceConfiguration) {
        config.width = config.width.max(1);
        config.height = config.height.max(1);
    }

    fn recreate_surface(&mut self, state: &State) {
        self.surface.configure(&state.device, &self.config);
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>, state: &State, cache: &AssetCache) {
        let Ok(mut new_config) = state
            .surface_config(&self.surface, new_size)
            .context(StateErr)
        else {
            return;
        };

        Self::clamp_config(&mut new_config);
        self.config = new_config;
        self.surface.configure(&state.device, &self.config);

        self.offscreen_surface.recreate(&state.device, &self.config);
        self.depth_texture = Self::create_depth_texture(&state.device, &self.config);
        self.picking_surface.recreate(&state.device, &self.config);
        let pp_bgl = cache.bgl_post_process();
        let depth_view = self
            .depth_texture
            .create_view(&TextureViewDescriptor::default());
        self.post_process_data = PostProcessData::new(
            &state.device,
            &pp_bgl,
            self.offscreen_surface.view(),
            &depth_view,
        );
    }

    fn create_depth_texture(device: &Device, config: &SurfaceConfiguration) -> Texture {
        device.create_texture(&TextureDescriptor {
            label: Some("Depth Texture"),
            size: Extent3d {
                width: config.width.max(1),
                height: config.height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
    }

    fn begin_render(&mut self, state: &State) -> Result<FrameCtx> {
        self.frame_count += 1;

        let mut output = self.surface.get_current_texture().context(SurfaceErr)?;
        if output.suboptimal {
            drop(output);
            self.recreate_surface(state);
            output = self.surface.get_current_texture().context(SurfaceErr)?;
        }

        let color_view = output
            .texture
            .create_view(&TextureViewDescriptor::default());
        let depth_view = self
            .depth_texture
            .create_view(&TextureViewDescriptor::default());

        Ok(FrameCtx {
            output,
            color_view,
            depth_view,
        })
    }

    fn update_render_data(&mut self, queue: &Queue) {
        self.update_system_data(queue);
    }

    fn update_view_camera_data(&mut self, queue: &Queue) {
        self.render_data.upload_camera_data(queue);
    }

    fn update_system_data(&mut self, queue: &Queue) {
        let window_size = self.window.inner_size();
        let window_size = Vector2::new(window_size.width.max(1), window_size.height.max(1));

        let system_data = &mut self.render_data.system_data;
        system_data.screen_size = window_size;
        system_data.time = self.start_time.elapsed().as_secs_f32();
        system_data.delta_time = self.delta_time.as_secs_f32();

        self.render_data.upload_system_data(queue);
    }

    /// Updates the delta time based on the elapsed time since the last frame
    fn tick_delta_time(&mut self) {
        self.delta_time = self.last_frame_time.elapsed();
        self.last_frame_time = Instant::now();
    }

    fn window(&self) -> &Window {
        &self.window
    }

    fn window_mut(&mut self) -> &mut Window {
        &mut self.window
    }

    fn size(&self) -> PhysicalSize<u32> {
        PhysicalSize {
            width: self.config.width,
            height: self.config.height,
        }
    }
}

#[allow(dead_code)]
pub struct Renderer {
    pub state: Box<State>,
    pub cache: AssetCache,
    shadow_render_data: RenderUniformData,
    viewports: HashMap<RenderTargetId, RenderViewport>,
    window_map: HashMap<WindowId, RenderTargetId>,
    game_rx: Receiver<RenderMsg>,
    proxies: HashMap<TypedComponentId, SceneProxyBinding>,
    sorted_proxies: Vec<(u32, TypedComponentId)>,
    start_time: Instant,
    pick_result_tx: Sender<PickResult>,
    pending_pick_requests: Vec<PickRequest>,
    pub(super) lights: LightManager,
}

impl Renderer {
    pub fn new(
        game_rx: Receiver<RenderMsg>,
        pick_result_tx: Sender<PickResult>,
        main_window: Window,
        store: Arc<AssetStore>,
    ) -> Result<Self> {
        let (state, surface, config) = State::new(&main_window).context(StateErr)?;
        let cache = AssetCache::new(store, &state);

        let render_bgl = cache.bgl_render();
        let shadow_render_data = RenderUniformData::empty(&state.device, &render_bgl);
        let lights = LightManager::new(&cache, &state.device);
        let start_time = Instant::now();

        main_window.request_redraw();

        let mut window_map = HashMap::new();
        window_map.insert(main_window.id(), RenderTargetId::PRIMARY);

        let mut viewports = HashMap::new();
        viewports.insert(
            RenderTargetId::PRIMARY,
            RenderViewport::new(main_window, surface, config, &state, &cache),
        );

        Ok(Renderer {
            state: Box::new(state),
            cache,
            shadow_render_data,
            viewports,
            window_map,
            game_rx,
            start_time,
            proxies: HashMap::new(),
            sorted_proxies: Vec::new(),
            pick_result_tx,
            pending_pick_requests: Vec::new(),
            lights,
        })
    }

    fn take_pick_request(&mut self, target: RenderTargetId) -> Option<PickRequest> {
        if let Some(idx) = self
            .pending_pick_requests
            .iter()
            .rposition(|r| r.target == target)
        {
            Some(self.pending_pick_requests.swap_remove(idx))
        } else {
            None
        }
    }

    pub fn find_render_target_id(&self, window_id: &WindowId) -> Option<RenderTargetId> {
        self.window_map.get(window_id).copied()
    }

    pub fn window(&self, viewport: RenderTargetId) -> Option<&Window> {
        self.viewports.get(&viewport).map(RenderViewport::window)
    }

    pub fn window_mut(&mut self, viewport: RenderTargetId) -> Option<&mut Window> {
        self.viewports
            .get_mut(&viewport)
            .map(RenderViewport::window_mut)
    }

    /// Export the offscreen render target for a viewport as a PNG image.
    pub fn export_offscreen_png(
        &self,
        target: RenderTargetId,
        path: impl AsRef<std::path::Path>,
    ) -> Result<(), TextureExportError> {
        let viewport = self
            .viewports
            .get(&target)
            .ok_or(TextureExportError::Unavailable {
                reason: "render target not found",
            })?;

        save_texture_to_png(
            &self.state.device,
            &self.state.queue,
            viewport.offscreen_surface.texture(),
            viewport.config.format,
            viewport.config.width,
            viewport.config.height,
            path,
        )
    }

    /// Export the picking buffer for a viewport as a PNG image.
    pub fn export_picking_png(
        &self,
        target: RenderTargetId,
        path: impl AsRef<std::path::Path>,
    ) -> Result<(), TextureExportError> {
        let viewport = self
            .viewports
            .get(&target)
            .ok_or(TextureExportError::Unavailable {
                reason: "render target not found",
            })?;

        save_texture_to_png(
            &self.state.device,
            &self.state.queue,
            viewport.picking_surface.texture(),
            PICKING_TEXTURE_FORMAT,
            viewport.config.width,
            viewport.config.height,
            path,
        )
    }

    /// Export any GPU texture to a PNG image.
    pub fn export_texture_png(
        &self,
        texture: &GpuTexture,
        path: impl AsRef<std::path::Path>,
    ) -> Result<(), TextureExportError> {
        let size = texture.size;
        save_texture_to_png(
            &self.state.device,
            &self.state.queue,
            &texture.texture,
            texture.format,
            size.width,
            size.height,
            path,
        )
    }

    /// Export a cached texture handle to a PNG image.
    pub fn export_cached_texture_png(
        &self,
        texture: HTexture,
        path: impl AsRef<std::path::Path>,
    ) -> Result<(), TextureExportError> {
        let gpu_tex = self.cache.texture(texture);
        self.export_texture_png(&gpu_tex, path)
    }

    pub fn start_time(&self) -> Instant {
        self.start_time
    }

    pub fn handle_events(&mut self) {
        loop {
            let Ok(msg) = self.game_rx.try_recv() else {
                break;
            };
            self.handle_message(msg);
        }
    }

    pub fn resize(&mut self, target_id: RenderTargetId, new_size: PhysicalSize<u32>) -> bool {
        let Some(viewport) = self.viewports.get_mut(&target_id) else {
            warn!("Invalid Viewport {target_id:?} referenced");
            return false;
        };

        viewport.resize(new_size, &self.state, &self.cache);

        true
    }

    pub fn redraw(&mut self, target_id: RenderTargetId) -> bool {
        let Some(mut viewport) = self.viewports.remove(&target_id) else {
            warn!("Invalid Viewport {target_id:?} referenced");
            return false;
        };

        viewport.tick_delta_time();
        let rendered = self.render_frame(target_id, &mut viewport);

        self.viewports.insert(target_id, viewport);

        rendered
    }

    pub fn update(&mut self) {
        let mut proxies = mem::take(&mut self.proxies);
        for proxy in proxies.values_mut() {
            proxy.update(self);
        }
        self.proxies = proxies;

        for vp in self.viewports.values_mut() {
            vp.update_render_data(&self.state.queue);
        }

        self.resort_proxies();

        self.lights
            .update(&self.cache, &self.state.queue, &self.state.device);
    }

    fn resort_proxies(&mut self) {
        let frustum = self
            .viewports
            .get(&RenderTargetId::PRIMARY)
            .map(|vp| Frustum::from_matrix(&vp.render_data.camera_data.proj_view_mat));

        self.sorted_proxies =
            sorted_enabled_proxy_ids(&self.proxies, self.cache.store(), frustum.as_ref());
    }

    pub fn render_frame(
        &mut self,
        target_id: RenderTargetId,
        viewport: &mut RenderViewport,
    ) -> bool {
        let mut ctx = match viewport.begin_render(&self.state) {
            Ok(ctx) => ctx,
            Err(RenderError::Surface {
                source: SurfaceError::Lost,
            }) => {
                let size = viewport.size();
                viewport.resize(size, &self.state, &self.cache);
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

        let frustum = Frustum::from_matrix(&viewport.render_data.camera_data.proj_view_mat);
        self.sorted_proxies =
            sorted_enabled_proxy_ids(&self.proxies, self.cache.store(), Some(&frustum));

        if let Some(request) = self.take_pick_request(target_id) {
            self.picking_pass(viewport, &mut ctx, request);
        }

        self.render(viewport, &mut ctx);
        self.end_render(viewport, ctx);

        true
    }

    fn picking_pass(
        &mut self,
        viewport: &RenderViewport,
        ctx: &mut FrameCtx,
        request: PickRequest,
    ) {
        let split_idx = self.first_ui_proxy_index();

        let mut encoder = self
            .state
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Picking Encoder"),
            });

        {
            let pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Picking Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: viewport.picking_surface.view(),
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        }),
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
            });

            self.render_scene(
                ctx,
                pass,
                RenderPassType::Picking,
                &self.sorted_proxies[..split_idx],
                &viewport.render_data,
            );
        }

        if split_idx < self.sorted_proxies.len() {
            let pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Picking UI Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: viewport.picking_surface.view(),
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..RenderPassDescriptor::default()
            });

            self.render_scene(
                ctx,
                pass,
                RenderPassType::PickingUi,
                &self.sorted_proxies[split_idx..],
                &viewport.render_data,
            );
        }

        let read_buffer = self.state.device.create_buffer(&BufferDescriptor {
            label: Some("Picking Readback Buffer"),
            size: PICKING_ROW_PITCH as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        encoder.copy_texture_to_buffer(
            TexelCopyTextureInfo {
                texture: viewport.picking_surface.texture(),
                mip_level: 0,
                origin: Origin3d {
                    x: request.position.0,
                    y: request.position.1,
                    z: 0,
                },
                aspect: TextureAspect::All,
            },
            TexelCopyBufferInfo {
                buffer: &read_buffer,
                layout: TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(PICKING_ROW_PITCH),
                    rows_per_image: Some(1),
                },
            },
            Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );

        self.state.queue.submit(Some(encoder.finish()));

        if let Some(result) = self.resolve_pick_buffer(read_buffer, request) {
            let _ = self.pick_result_tx.send(result);
        }
    }

    fn resolve_pick_buffer(&self, buffer: Buffer, request: PickRequest) -> Option<PickResult> {
        let slice = buffer.slice(0..4);
        let (tx, rx) = crossbeam_channel::bounded(1);
        slice.map_async(MapMode::Read, move |res| {
            let _ = tx.send(res);
        });
        let _ = self.state.device.poll(PollType::Wait);

        if rx.recv().ok().and_then(Result::ok).is_none() {
            buffer.unmap();
            return None;
        }

        let data = slice.get_mapped_range();
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(&data[..4]);
        drop(data);
        buffer.unmap();

        let hash = color_bytes_to_hash(bytes);

        Some(PickResult {
            id: request.id,
            target: request.target,
            hash,
        })
    }

    fn render(&mut self, viewport: &RenderViewport, ctx: &mut FrameCtx) {
        self.shadow_pass(ctx);
        self.main_pass(viewport, ctx);
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

        let assignments: Vec<_> = self
            .lights
            .shadow_assignments()
            .iter()
            .copied()
            .take(light_count as usize)
            .collect();

        // Shadow map ids and assignments may change when capacity is constrained, so upload the
        // updated proxy data again before the main pass consumes it.
        self.lights
            .update(&self.cache, &self.state.queue, &self.state.device);

        for assignment in assignments {
            let Some(light) = self.lights.light(assignment.light_index).copied() else {
                debug_panic!("Invalid light index");
                continue;
            };

            let Ok(light_type) = LightType::try_from(light.type_id) else {
                debug_panic!("Invalid Light Type Id was stored");
                continue;
            };

            match light_type {
                LightType::Spot => {
                    if assignment.face == 0 {
                        self.shadow_render_data
                            .update_shadow_camera_for_spot(&light, &self.state.queue);
                        self.prepare_shadow_map(ctx, assignment.layer);
                    }
                }
                LightType::Point => {
                    self.shadow_render_data.update_shadow_camera_for_point(
                        &light,
                        assignment.face,
                        &self.state.queue,
                    );
                    self.prepare_shadow_map(ctx, assignment.layer);
                }
                LightType::Sun => {}
            }
        }
    }

    fn prepare_shadow_map(&mut self, ctx: &mut FrameCtx, layer: u32) {
        let split_idx = self.first_ui_proxy_index();

        let mut encoder = self
            .state
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Shadow Pass Encoder"),
            });

        let layer_view = self.lights.shadow_layer(&self.cache, layer);
        let pass = self.prepare_shadow_pass(&mut encoder, &layer_view);

        self.render_scene(
            ctx,
            pass,
            RenderPassType::Shadow,
            &self.sorted_proxies[..split_idx],
            &self.shadow_render_data,
        );

        self.state.queue.submit(Some(encoder.finish()));
    }

    fn main_pass(&mut self, viewport: &RenderViewport, ctx: &mut FrameCtx) {
        let mut encoder = self
            .state
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Main Encoder"),
            });

        let split_idx = self.first_ui_proxy_index();

        {
            let pass = self.prepare_main_render_pass(&mut encoder, viewport, ctx);

            self.render_scene(
                ctx,
                pass,
                RenderPassType::Color,
                &self.sorted_proxies[..split_idx],
                &viewport.render_data,
            );
        }

        if split_idx < self.sorted_proxies.len() {
            let pass = self.prepare_ui_render_pass(&mut encoder, viewport, ctx);

            self.render_scene(
                ctx,
                pass,
                RenderPassType::Color2D,
                &self.sorted_proxies[split_idx..],
                &viewport.render_data,
            );
        }

        self.state.queue.submit(Some(encoder.finish()));
    }

    fn render_scene(
        &self,
        frame_ctx: &FrameCtx,
        pass: RenderPass,
        pass_type: RenderPassType,
        proxies: &[(u32, TypedComponentId)],
        render_uniform: &RenderUniformData,
    ) {
        let shadow_bind_group = match pass_type {
            RenderPassType::Color | RenderPassType::Color2D => self.lights.shadow_uniform(),
            RenderPassType::Shadow => self.lights.placeholder_shadow_uniform(),
            RenderPassType::Picking | RenderPassType::PickingUi => {
                self.lights.placeholder_shadow_uniform()
            }
        }
        .bind_group();

        let draw_ctx = GPUDrawCtx {
            frame: frame_ctx,
            pass: RwLock::new(pass),
            pass_type,
            render_bind_group: render_uniform.uniform.bind_group(),
            light_bind_group: self.lights.uniform().bind_group(),
            shadow_bind_group,
        };

        self.render_proxies(&draw_ctx, proxies);

        #[cfg(debug_assertions)]
        if DebugRenderer::light() && pass_type == RenderPassType::Color {
            self.lights.render_debug_lights(self, &draw_ctx);
        }
    }

    fn render_proxies(&self, ctx: &GPUDrawCtx, proxies: &[(u32, TypedComponentId)]) {
        for proxy in proxies.iter().map(|(_, ctid)| self.proxies.get(ctid)) {
            let Some(proxy) = proxy else {
                debug_panic!("Sorted proxy not in proxy list");
                continue;
            };
            proxy.render(self, ctx);
        }
    }

    fn first_ui_proxy_index(&self) -> usize {
        self.sorted_proxies
            .partition_point(|(priority, _)| *priority < PROXY_PRIORITY_2D)
    }

    fn end_render(&mut self, viewport: &mut RenderViewport, mut ctx: FrameCtx) {
        self.render_final_pass(viewport, &mut ctx);

        viewport.window.pre_present_notify();
        ctx.output.present();

        if self.cache.last_refresh().elapsed().as_secs_f32() > 5.0 {
            trace!("Refreshing cache...");
            let refreshed_count = self.cache.refresh_all();
            if cfg!(debug_assertions) && refreshed_count != 0 {
                trace!("Refreshed cache elements {}", refreshed_count);
            }
        }
    }

    fn render_final_pass(&mut self, viewport: &RenderViewport, ctx: &mut FrameCtx) {
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
        let groups = post_shader.bind_groups();
        pass.set_pipeline(post_shader.solid_pipeline());
        pass.set_bind_group(
            groups.render,
            viewport.render_data.uniform.bind_group(),
            &[],
        );
        if let Some(idx) = groups.post_process {
            pass.set_bind_group(idx, viewport.post_process_data.uniform.bind_group(), &[]);
        }
        pass.draw(0..6, 0..1);

        drop(pass);

        self.state.queue.submit(Some(encoder.finish()));
    }

    fn handle_message(&mut self, msg: RenderMsg) {
        match msg {
            RenderMsg::RegisterProxy(cid, object_hash, mut proxy, local_to_world) => {
                trace!("Registered Proxy for #{:?}", cid.0);
                let data = proxy.setup_render(self, local_to_world.matrix());
                let binding = SceneProxyBinding::new(cid, object_hash, local_to_world, data, proxy);
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
            RenderMsg::UpdateActiveCamera(render_target_id, camera_data) => {
                if let Some(vp) = self.viewports.get_mut(&render_target_id) {
                    camera_data(&mut vp.render_data.camera_data);
                    vp.update_view_camera_data(&self.state.queue);
                }
            }
            RenderMsg::ProxyState(cid, enabled) => {
                if let Some(binding) = self.proxies.get_mut(&cid) {
                    binding.enabled = enabled;
                }
            }
            RenderMsg::PickRequest(request) => {
                if self.viewports.contains_key(&request.target) {
                    self.pending_pick_requests.push(request);
                }
            }
            RenderMsg::CommandBatch(batch) => {
                for message in batch {
                    self.handle_message(message);
                }
            }
            RenderMsg::CaptureOffscreenTexture(target, path) => {
                if let Err(e) = self.export_offscreen_png(target, &path) {
                    warn!("Couldn't capture offscreen texture: {e}");
                }
            }
            RenderMsg::CapturePickingTexture(target, path) => {
                if let Err(e) = self.export_picking_png(target, &path) {
                    warn!("Couldn't capture picking texture: {e}");
                }
            }
            RenderMsg::CaptureTexture(texture, path) => {
                if let Err(e) = self.export_cached_texture_png(texture, &path) {
                    warn!("Couldn't capture picking texture: {e}");
                }
            }
        }
    }

    pub fn add_window(&mut self, target_id: RenderTargetId, window: Window) -> Result<()> {
        if self.viewports.contains_key(&target_id) {
            warn!(
                "Viewport #{:?} already exists; ignoring duplicate add",
                target_id
            );
            return Ok(());
        }

        let surface = self.state.create_surface(&window).context(StateErr)?;
        let config = self
            .state
            .surface_config(&surface, window.inner_size())
            .context(StateErr)?;

        self.window_map.insert(window.id(), target_id);

        let viewport = RenderViewport::new(window, surface, config, &self.state, &self.cache);
        self.viewports.insert(target_id, viewport);

        Ok(())
    }

    fn prepare_shadow_pass<'a>(
        &self,
        encoder: &'a mut CommandEncoder,
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
        encoder: &'a mut CommandEncoder,
        viewport: &RenderViewport,
        ctx: &mut FrameCtx,
    ) -> RenderPass<'a> {
        encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Offscreen Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: viewport.offscreen_surface.view(),
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

    fn prepare_ui_render_pass<'a>(
        &self,
        encoder: &'a mut CommandEncoder,
        viewport: &RenderViewport,
        _ctx: &mut FrameCtx,
    ) -> RenderPass<'a> {
        encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("UI Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: viewport.offscreen_surface.view(),
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..RenderPassDescriptor::default()
        })
    }
}

fn sorted_enabled_proxy_ids(
    proxies: &HashMap<TypedComponentId, SceneProxyBinding>,
    store: &AssetStore,
    frustum: Option<&Frustum>,
) -> Vec<(u32, TypedComponentId)> {
    proxies
        .iter()
        .filter(|(_, binding)| binding.enabled)
        .filter(|(_, binding)| {
            if let Some(f) = frustum
                && let Some(bounds) = binding.bounds()
            {
                return f.intersects_sphere(&bounds);
            }
            true
        })
        .map(|(tid, proxy)| (proxy.proxy.priority(store), *tid))
        .sorted_by_key(|(priority, _)| *priority)
        .collect()
}

#[derive(Debug, Clone, Copy)]
struct FrustumPlane {
    normal: Vector3<f32>,
    d: f32,
}

impl FrustumPlane {
    fn distance_to(&self, sphere: &BoundingSphere) -> f32 {
        self.normal.dot(&sphere.center) + self.d
    }
}

#[derive(Debug, Clone, Copy)]
struct Frustum {
    planes: [FrustumPlane; 6],
}

impl Frustum {
    fn from_matrix(m: &Matrix4<f32>) -> Self {
        let row0 = m.row(0).transpose();
        let row1 = m.row(1).transpose();
        let row2 = m.row(2).transpose();
        let row3 = m.row(3).transpose();

        let plane_from = |v: Vector4<f32>| {
            let normal = Vector3::new(v.x, v.y, v.z);
            let len = normal.norm();
            if len > 0.0 {
                FrustumPlane {
                    normal: normal / len,
                    d: v.w / len,
                }
            } else {
                FrustumPlane { normal, d: v.w }
            }
        };

        let planes = [
            plane_from(row3 + row0), // left
            plane_from(row3 - row0), // right
            plane_from(row3 + row1), // bottom
            plane_from(row3 - row1), // top
            plane_from(row3 + row2), // near
            plane_from(row3 - row2), // far
        ];

        Frustum { planes }
    }

    fn intersects_sphere(&self, sphere: &BoundingSphere) -> bool {
        self.planes
            .iter()
            .all(|p| p.distance_to(sphere) >= -sphere.radius)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::ComponentId;
    use crate::rendering::proxies::SceneProxy;
    use nalgebra::{Affine3, Matrix4};
    use slotmap::Key;
    use std::any::{Any, TypeId};
    use std::collections::HashMap;

    #[derive(Debug)]
    struct TestProxy {
        priority: u32,
    }

    impl SceneProxy for TestProxy {
        fn setup_render(&mut self, _: &Renderer, _: &Matrix4<f32>) -> Box<dyn Any> {
            Box::new(())
        }

        fn update_render(&mut self, _: &Renderer, _: &mut dyn Any, _: &Matrix4<f32>) {}

        fn render(&self, _: &Renderer, _: &dyn Any, _: &GPUDrawCtx, _: &Matrix4<f32>) {}

        fn priority(&self, _: &AssetStore) -> u32 {
            self.priority
        }
    }

    #[test]
    fn resort_proxies_orders_by_priority() {
        struct MarkerLow;
        struct MarkerMid;
        struct MarkerHigh;

        let store = AssetStore::new();
        let mut proxies = HashMap::new();

        let id_high = insert_proxy::<MarkerHigh>(&mut proxies, 900, true);
        let id_low = insert_proxy::<MarkerLow>(&mut proxies, 10, true);
        let id_mid = insert_proxy::<MarkerMid>(&mut proxies, 50, true);

        let sorted = sorted_enabled_proxy_ids(&proxies, &store, None);
        assert_eq!(sorted, vec![(10, id_low), (50, id_mid), (900, id_high)]);
    }

    #[test]
    fn resort_proxies_ignores_disabled_bindings() {
        struct MarkerEnabled;
        struct MarkerDisabled;

        let store = AssetStore::new();
        let mut proxies = HashMap::new();

        let id_enabled = insert_proxy::<MarkerEnabled>(&mut proxies, 5, true);
        let id_disabled = insert_proxy::<MarkerDisabled>(&mut proxies, 1, false);

        let sorted = sorted_enabled_proxy_ids(&proxies, &store, None);
        assert_eq!(sorted, vec![(5, id_enabled)]);
        assert!(!sorted.contains(&(1, id_disabled)));
    }

    fn insert_proxy<T: 'static>(
        proxies: &mut HashMap<TypedComponentId, SceneProxyBinding>,
        priority: u32,
        enabled: bool,
    ) -> TypedComponentId {
        let tid = TypedComponentId(TypeId::of::<T>(), ComponentId::null());
        let mut binding = SceneProxyBinding::new(
            tid,
            1,
            Affine3::identity(),
            Box::new(()),
            Box::new(TestProxy { priority }),
        );
        binding.enabled = enabled;
        proxies.insert(tid, binding);
        tid
    }
}
