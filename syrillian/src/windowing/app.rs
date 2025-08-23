use crate::assets::AssetStore;
use crate::game_thread::GameAppEvent;
use crate::rendering::Renderer;
use crate::windowing::game_thread::GameThread;
use crate::AppState;
use log::{error, info, trace};
use std::error::Error;
use std::sync::mpsc;
use winit::application::ApplicationHandler;
use winit::error::EventLoopError;
use winit::event::{DeviceEvent, DeviceId, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{CursorGrabMode, WindowAttributes, WindowId};

pub struct App<S: AppState> {
    window_attributes: WindowAttributes,
    game_thread: Option<GameThread>,
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
    pub fn run(mut self, event_loop: EventLoop<()>) -> Result<(), Box<dyn Error>> {
        event_loop.run_app(&mut self)?;
        Ok(())
    }

    fn init(&mut self, event_loop: &ActiveEventLoop) {
        info!("Initializing render state");

        let asset_store = AssetStore::new();

        let (render_state_tx, render_state_rx) = mpsc::channel();
        let state = self.state.take().unwrap();
        let game_thread = match GameThread::new(state, asset_store.clone(), render_state_tx) {
            Ok(r) => r,
            Err(e) => {
                error!("Error when creating renderer: {e}");
                event_loop.exit();
                return;
            }
        };

        let window = event_loop
            .create_window(self.window_attributes.clone())
            .unwrap();

        let renderer = match Renderer::new(render_state_rx, window, asset_store) {
            Ok(r) => r,
            Err(err) => {
                error!("Couldn't create renderer: {err}");
                event_loop.exit();
                return;
            }
        };


        if let Err(e) = game_thread.init() {
            error!("Error when initializing Game Thread: {e}");
            event_loop.exit();
            return;
        }

        self.game_thread = Some(game_thread);
        self.renderer = Some(renderer);
    }

    fn handle_events(game_thread: &GameThread, renderer: &mut Renderer) {
        for event in game_thread.receive_events() {
            match event {
                GameAppEvent::UpdateWindowTitle(title) => renderer.window_mut().set_title(&title),
                GameAppEvent::SetCursorMode(locked, visible) => {
                    if locked {
                        trace!("RT: Locked cursor");
                        renderer.window_mut().set_cursor_grab(CursorGrabMode::Locked)
                            .or_else(|_| renderer.window_mut().set_cursor_grab(CursorGrabMode::Confined)).expect("Couldn't grab cursor");
                    } else {
                        trace!("RT: Unlocked cursor");
                        renderer.window_mut().set_cursor_grab(CursorGrabMode::None).expect("Couldn't ungrab cursor");
                    }
                    renderer.window_mut().set_cursor_visible(visible);
                    if visible {
                        trace!("RT: Shown cursor");
                    } else {
                        trace!("RT: Hid cursor");
                    }
                }
            }
        }
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
        if event_loop.exiting() {
            return;
        }

        let game_thread = self.game_thread.as_ref().unwrap();
        let renderer = self.renderer.as_mut().unwrap();

        match event {
            WindowEvent::RedrawRequested => {
                Self::handle_events(game_thread, renderer);
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
            },
            WindowEvent::Resized(size) => {
                renderer.resize(size);
                if game_thread.resize(size).is_err() {
                    event_loop.exit();
                }
            },
            _ => {
                if game_thread.input(event).is_err() {
                    event_loop.exit();
                }
            }
        }
    }

    fn device_event(&mut self, event_loop: &ActiveEventLoop, device_id: DeviceId, event: DeviceEvent) {
        if self.game_thread.as_ref().unwrap().device_event(device_id, event).is_err() {
            event_loop.exit();
        }
    }
}
