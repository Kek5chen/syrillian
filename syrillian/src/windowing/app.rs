use crate::AppState;
use crate::assets::AssetStore;
use crate::game_thread::GameAppEvent;
use crate::rendering::Renderer;
use crate::windowing::game_thread::GameThread;
use crate::world::WorldChannels;
use crossbeam_channel::unbounded;
use log::{error, info, trace};
use std::error::Error;
use std::marker::PhantomData;
use winit::application::ApplicationHandler;
use winit::dpi::Size;
use winit::error::EventLoopError;
use winit::event::{DeviceEvent, DeviceId, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{CursorGrabMode, WindowAttributes, WindowId};

#[cfg(target_arch = "wasm32")]
use winit::platform::web::{EventLoopExtWebSys, WindowExtWebSys};

pub struct App<S: AppState> {
    main_window_attributes: WindowAttributes,
    renderer: Option<Renderer>,
    game_thread: Option<GameThread<S>>,
}

pub struct AppSettings<S: AppState> {
    pub main_window: WindowAttributes,
    pub(crate) _state_type: PhantomData<S>,
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
            main_window_attributes: self.main_window,
            renderer: None,
            game_thread: None,
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

        let (render_state_tx, render_state_rx) = unbounded();
        let (game_event_tx, game_event_rx) = unbounded();
        let (pick_result_tx, pick_result_rx) = unbounded();

        let main_window = event_loop
            .create_window(self.main_window_attributes.clone())
            .unwrap();

        #[cfg(target_arch = "wasm32")]
        if let Some(canvas) = main_window.canvas() {
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

        let renderer = match Renderer::new(
            render_state_rx,
            pick_result_tx,
            main_window,
            asset_store.clone(),
        ) {
            Ok(r) => r,
            Err(err) => {
                error!("Couldn't create renderer: {err}");
                event_loop.exit();
                return;
            }
        };

        trace!("Created Renderer");

        let channels = WorldChannels::new(render_state_tx, game_event_tx, pick_result_rx);
        let game_thread = GameThread::new(asset_store.clone(), channels, game_event_rx);

        if !game_thread.init() {
            error!("Couldn't initialize Game Thread");
            event_loop.exit();
            return;
        }

        self.renderer = Some(renderer);
        self.game_thread = Some(game_thread);
    }

    fn handle_events(
        renderer: &mut Renderer,
        game_thread: &GameThread<S>,
        event_loop: &ActiveEventLoop,
    ) -> bool {
        for event in game_thread.game_event_rx.try_iter() {
            match event {
                GameAppEvent::UpdateWindowTitle(event_target, title) => {
                    if let Some(window) = renderer.window_mut(event_target) {
                        window.set_title(&title);
                    }
                }
                GameAppEvent::SetCursorMode(event_target, locked, visible) => {
                    if let Some(window) = renderer.window_mut(event_target) {
                        if locked {
                            trace!("RT: Locked cursor");
                            window
                                .set_cursor_grab(CursorGrabMode::Locked)
                                .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined))
                                .expect("Couldn't grab cursor");
                        } else {
                            trace!("RT: Unlocked cursor");
                            window
                                .set_cursor_grab(CursorGrabMode::None)
                                .expect("Couldn't ungrab cursor");
                        }
                        window.set_cursor_visible(visible);
                        if visible {
                            trace!("RT: Shown cursor");
                        } else {
                            trace!("RT: Hid cursor");
                        }
                    }
                }
                GameAppEvent::AddWindow(event_target, size) => {
                    let window = match event_loop.create_window(
                        WindowAttributes::default()
                            .with_inner_size(Size::Physical(size))
                            .with_title(format!("Syrillian Window {}", event_target.get())),
                    ) {
                        Ok(w) => w,
                        Err(e) => {
                            error!("Failed to create window: {e}");
                            return false;
                        }
                    };

                    if let Err(e) = renderer.add_window(event_target, window) {
                        error!("Failed to create window: {e}");
                        return false;
                    }
                }
                GameAppEvent::Shutdown => return false,
            }
        }
        true
    }

    fn handle_all_game_events(&mut self, event_loop: &ActiveEventLoop) -> bool {
        let Some(renderer) = self.renderer.as_mut() else {
            return true;
        };

        let Some(game_thread) = self.game_thread.as_mut() else {
            return true;
        };

        if !Self::handle_events(renderer, game_thread, event_loop) {
            return false;
        }
        true
    }
}

impl<S: AppState> ApplicationHandler for App<S> {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        match cause {
            StartCause::Poll => {
                if !self.handle_all_game_events(event_loop) {
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
        if event_loop.exiting() {
            return;
        }

        if !self.handle_all_game_events(event_loop) {
            event_loop.exit();
            return;
        }

        let Some(game_thread) = self.game_thread.as_ref() else {
            return;
        };
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };

        let target_id = renderer
            .find_render_target_id(&window_id)
            .expect("runtime missing for window");
        let drives_update = target_id.is_primary();

        match event {
            WindowEvent::RedrawRequested => {
                if drives_update && game_thread.next_frame(target_id).is_err() {
                    event_loop.exit();
                    return;
                }

                if drives_update {
                    renderer.handle_events();
                    renderer.update();
                }

                if !renderer.redraw(target_id) {
                    event_loop.exit();
                    return;
                }

                if let Some(window) = renderer.window(target_id) {
                    window.request_redraw();
                }
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                renderer.resize(target_id, size);
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

        // debug_assert!(event_start.elapsed().as_secs_f32() < 2.0);
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: DeviceId,
        event: DeviceEvent,
    ) {
        if !self.handle_all_game_events(event_loop) {
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
