use crate::AppState;
use crate::assets::AssetStore;
use crate::game_thread::GameAppEvent;
use crate::rendering::Renderer;
use crate::windowing::game_thread::GameThread;
use log::{error, info, trace};
use std::error::Error;
use std::sync::mpsc;
use web_time::Instant;
use winit::application::ApplicationHandler;
use winit::error::EventLoopError;
use winit::event::{DeviceEvent, DeviceId, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{CursorGrabMode, WindowAttributes, WindowId};

#[cfg(target_arch = "wasm32")]
use winit::platform::web::{EventLoopExtWebSys, WindowExtWebSys};

pub struct App<S: AppState> {
    window_attributes: WindowAttributes,
    game_thread: Option<GameThread<S>>,
    state: Option<S>,
    renderer: Option<Renderer>,
}

pub struct AppSettings<S: AppState> {
    pub window: WindowAttributes,
    pub state: S,
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

    fn init_state(self) -> Result<(EventLoop<()>, App<S>), Box<dyn Error>> {
        let event_loop = match EventLoop::new() {
            Err(EventLoopError::NotSupported(_)) => {
                return Err("No graphics backend found that could be used.".into());
            }
            e => e?,
        };
        event_loop.set_control_flow(ControlFlow::Poll);

        let app = App {
            window_attributes: self.window,
            state: Some(self.state),
            game_thread: None,
            renderer: None,
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

        let (render_state_tx, render_state_rx) = mpsc::channel();
        let state = self.state.take().unwrap();

        #[allow(unused_mut)]
        let mut game_thread = GameThread::new(state, asset_store.clone(), render_state_tx);

        let window = event_loop
            .create_window(self.window_attributes.clone())
            .unwrap();

        #[cfg(target_arch = "wasm32")]
        if let Some(canvas) = window.canvas() {
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

        let renderer = match Renderer::new(render_state_rx, window, asset_store) {
            Ok(r) => r,
            Err(err) => {
                error!("Couldn't create renderer: {err}");
                event_loop.exit();
                return;
            }
        };

        trace!("Created Renderer");

        if !game_thread.init() {
            error!("Couldn't initialize Game Thread");
            event_loop.exit();
            return;
        }

        trace!("Created Game Thread");

        self.game_thread = Some(game_thread);
        self.renderer = Some(renderer);
    }

    fn handle_events(game_thread: &GameThread<S>, renderer: &mut Renderer) -> bool {
        for event in game_thread.receive_events() {
            match event {
                GameAppEvent::UpdateWindowTitle(title) => renderer.window_mut().set_title(&title),
                GameAppEvent::SetCursorMode(locked, visible) => {
                    if locked {
                        trace!("RT: Locked cursor");
                        renderer
                            .window_mut()
                            .set_cursor_grab(CursorGrabMode::Locked)
                            .or_else(|_| {
                                renderer
                                    .window_mut()
                                    .set_cursor_grab(CursorGrabMode::Confined)
                            })
                            .expect("Couldn't grab cursor");
                    } else {
                        trace!("RT: Unlocked cursor");
                        renderer
                            .window_mut()
                            .set_cursor_grab(CursorGrabMode::None)
                            .expect("Couldn't ungrab cursor");
                    }
                    renderer.window_mut().set_cursor_visible(visible);
                    if visible {
                        trace!("RT: Shown cursor");
                    } else {
                        trace!("RT: Hid cursor");
                    }
                }
                GameAppEvent::Shutdown => return false,
            }
        }
        true
    }
}

impl<S: AppState> ApplicationHandler for App<S> {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        match cause {
            StartCause::Poll => (),
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
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let event_start = Instant::now();
        if event_loop.exiting() {
            return;
        }

        let game_thread = self.game_thread.as_ref().unwrap();
        let renderer = self.renderer.as_mut().unwrap();

        if !Self::handle_events(game_thread, renderer) {
            event_loop.exit();
            return;
        }

        match event {
            WindowEvent::RedrawRequested => {
                renderer.tick_delta_time();
                renderer.handle_events();
                renderer.update();
                renderer.render_frame();
                if game_thread.next_frame().is_err() {
                    event_loop.exit();
                }
                renderer.window.request_redraw();
            }
            WindowEvent::CloseRequested => {
                // TODO: Quit Game Thread
                event_loop.exit()
            }
            WindowEvent::Resized(size) => {
                renderer.resize(size);
                if game_thread.resize(size).is_err() {
                    event_loop.exit();
                }
            }
            _ => {
                if game_thread.input(event).is_err() {
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
        if self
            .game_thread
            .as_ref()
            .unwrap()
            .device_event(device_id, event)
            .is_err()
        {
            event_loop.exit();
        }
    }
}
