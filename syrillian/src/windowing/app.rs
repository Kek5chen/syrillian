use crate::AppState;
use crate::assets::AssetStore;
use crate::game_thread::{GameAppEvent, RenderTargetId};
use crate::rendering::Renderer;
use crate::windowing::game_thread::GameThread;
use crate::world::WorldChannels;
use crossbeam_channel::{Receiver, unbounded};
use log::{error, info, trace};
use std::collections::HashMap;
use std::error::Error;
use web_time::Instant;
use winit::application::ApplicationHandler;
use winit::error::EventLoopError;
use winit::event::{DeviceEvent, DeviceId, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{CursorGrabMode, WindowAttributes, WindowId};

#[cfg(target_arch = "wasm32")]
use winit::platform::web::{EventLoopExtWebSys, WindowExtWebSys};

pub struct App<S: AppState> {
    windows: Vec<WindowAttributes>,
    runtimes: Vec<WindowRuntime>,
    window_map: HashMap<WindowId, usize>,
    game_thread: Option<GameThread<S>>,
    state: Option<S>,
}

pub struct AppSettings<S: AppState> {
    pub windows: Vec<WindowAttributes>,
    pub state: S,
}

struct WindowRuntime {
    renderer: Renderer,
    game_event_rx: Receiver<GameAppEvent>,
    drives_update: bool,
}

pub trait AppRuntime: AppState {
    fn configure(self, title: &str, width: u32, height: u32) -> AppSettings<Self>;

    fn default_config(self) -> AppSettings<Self>;
}

impl<S: AppState> AppSettings<S> {
    pub fn run(self) -> Result<(), Box<dyn Error>> {
        let (event_loop, app) = self.init_state()?;
        app.run(event_loop)
    }

    pub fn with_additional_window(mut self, window: WindowAttributes) -> Self {
        self.windows.push(window);
        self
    }

    fn init_state(self) -> Result<(EventLoop<()>, App<S>), Box<dyn Error>> {
        if self.windows.is_empty() {
            return Err("No windows configured".into());
        }

        let event_loop = match EventLoop::new() {
            Err(EventLoopError::NotSupported(_)) => {
                return Err("No graphics backend found that could be used.".into());
            }
            e => e?,
        };
        event_loop.set_control_flow(ControlFlow::Poll);

        let app = App {
            windows: self.windows,
            runtimes: Vec::new(),
            window_map: HashMap::new(),
            game_thread: None,
            state: Some(self.state),
        };

        Ok((event_loop, app))
    }
}

impl<S: AppState> App<S> {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn run(mut self, event_loop: EventLoop<()>) -> Result<(), Box<dyn Error>> {
        event_loop.run_app(&mut self)?;
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    pub fn run(self, event_loop: EventLoop<()>) -> Result<(), Box<dyn Error>> {
        event_loop.spawn_app(self);
        Ok(())
    }

    fn init(&mut self, event_loop: &ActiveEventLoop) {
        info!("Initializing render state");

        let asset_store = AssetStore::new();

        let mut render_targets = Vec::new();
        let mut runtimes = Vec::new();

        for (idx, window_attrs) in self.windows.iter().enumerate() {
            let (render_state_tx, render_state_rx) = unbounded();
            let (game_event_tx, game_event_rx) = unbounded();

            render_targets.push((render_state_tx, game_event_tx));

            let window_handle = event_loop.create_window(window_attrs.clone()).unwrap();

            #[cfg(target_arch = "wasm32")]
            if let Some(canvas) = window_handle.canvas() {
                web_sys::window()
                    .unwrap()
                    .document()
                    .unwrap()
                    .get_elements_by_tag_name("body")
                    .get_with_index(0)
                    .unwrap()
                    .append_child(&canvas)
                    .unwrap();
            }

            trace!("Created render surface");

            let renderer = match Renderer::new(render_state_rx, window_handle, asset_store.clone())
            {
                Ok(r) => r,
                Err(err) => {
                    error!("Couldn't create renderer: {err}");
                    event_loop.exit();
                    return;
                }
            };

            trace!("Created Renderer");

            let window_id = renderer.window().id();
            self.window_map.insert(window_id, runtimes.len());
            runtimes.push(WindowRuntime {
                renderer,
                game_event_rx,
                drives_update: idx == 0,
            });
        }

        let state = self.state.take().expect("app state already initialized");
        let channels = WorldChannels::from_targets(render_targets);
        let game_thread = GameThread::new(state, asset_store.clone(), channels);

        if !game_thread.init() {
            error!("Couldn't initialize Game Thread");
            event_loop.exit();
            return;
        }

        self.runtimes = runtimes;
        self.game_thread = Some(game_thread);

        for runtime in &self.runtimes {
            runtime.renderer.window().request_redraw();
        }
    }

    fn handle_events(runtime: &mut WindowRuntime, target: RenderTargetId) -> bool {
        for event in runtime.game_event_rx.try_iter() {
            match event {
                GameAppEvent::UpdateWindowTitle(event_target, title) if event_target == target => {
                    runtime.renderer.window_mut().set_title(&title)
                }
                GameAppEvent::SetCursorMode(event_target, locked, visible)
                    if event_target == target =>
                {
                    if locked {
                        trace!("RT: Locked cursor");
                        runtime
                            .renderer
                            .window_mut()
                            .set_cursor_grab(CursorGrabMode::Locked)
                            .or_else(|_| {
                                runtime
                                    .renderer
                                    .window_mut()
                                    .set_cursor_grab(CursorGrabMode::Confined)
                            })
                            .expect("Couldn't grab cursor");
                    } else {
                        trace!("RT: Unlocked cursor");
                        runtime
                            .renderer
                            .window_mut()
                            .set_cursor_grab(CursorGrabMode::None)
                            .expect("Couldn't ungrab cursor");
                    }
                    runtime.renderer.window_mut().set_cursor_visible(visible);
                    if visible {
                        trace!("RT: Shown cursor");
                    } else {
                        trace!("RT: Hid cursor");
                    }
                }
                GameAppEvent::Shutdown => return false,
                _ => {}
            }
        }
        true
    }

    fn handle_all_game_events(&mut self) -> bool {
        for (idx, runtime) in self.runtimes.iter_mut().enumerate() {
            if !Self::handle_events(runtime, idx) {
                return false;
            }
        }
        true
    }
}

impl<S: AppState> ApplicationHandler for App<S> {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        match cause {
            StartCause::Poll => {
                if !self.handle_all_game_events() {
                    event_loop.exit();
                }
            }
            StartCause::Init => self.init(event_loop),
            StartCause::ResumeTimeReached { .. } => (),
            StartCause::WaitCancelled { .. } => (),
        }
    }

    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        // TODO: Reinit cache?
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let event_start = Instant::now();
        if event_loop.exiting() {
            return;
        }

        if !self.handle_all_game_events() {
            event_loop.exit();
            return;
        }

        let target_id = *self
            .window_map
            .get(&window_id)
            .expect("runtime missing for window");
        let Some(game_thread) = self.game_thread.as_ref() else {
            return;
        };
        let drives_update = self
            .runtimes
            .get(target_id)
            .map(|r| r.drives_update)
            .unwrap_or(false);
        let Some(runtime) = self.runtimes.get_mut(target_id) else {
            return;
        };

        match event {
            WindowEvent::RedrawRequested => {
                runtime.renderer.tick_delta_time();
                runtime.renderer.handle_events();
                runtime.renderer.update();
                if !runtime.renderer.render_frame() {
                    event_loop.exit();
                    return;
                }
                if drives_update && game_thread.next_frame(target_id).is_err() {
                    event_loop.exit();
                    return;
                }
                runtime.renderer.window().request_redraw();
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                runtime.renderer.resize(size);
                if game_thread.resize(target_id, size).is_err() {
                    event_loop.exit();
                }
            }
            _ => {
                if game_thread.input(target_id, event).is_err() {
                    event_loop.exit();
                }
            }
        }

        debug_assert!(event_start.elapsed().as_secs_f32() < 2.0);
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: DeviceId,
        event: DeviceEvent,
    ) {
        if !self.handle_all_game_events() {
            event_loop.exit();
            return;
        }

        let Some(game_thread) = self.game_thread.as_ref() else {
            return;
        };

        if game_thread.device_event(device_id, event.clone()).is_err() {
            event_loop.exit();
        }
    }
}
