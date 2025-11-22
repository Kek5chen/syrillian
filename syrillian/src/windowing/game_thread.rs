use crate::assets::AssetStore;
use crate::rendering::message::RenderMsg;
use crate::{AppState, World};
use log::{error, info};
use std::sync::mpsc::{SendError, TryRecvError};
use std::sync::{Arc, mpsc};
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, DeviceId, WindowEvent};

#[cfg(not(target_arch = "wasm32"))]
use std::marker::PhantomData;
#[cfg(not(target_arch = "wasm32"))]
use std::thread::JoinHandle;

#[derive(Debug, Clone)]
pub enum RenderAppEvent {
    Init,
    Input(WindowEvent),
    DeviceEvent(DeviceId, DeviceEvent),
    StartFrame,
    Resize(PhysicalSize<u32>),
}

#[derive(Debug, Clone)]
pub enum GameAppEvent {
    UpdateWindowTitle(String),
    SetCursorMode(bool, bool),
    Shutdown,
}

impl GameAppEvent {
    pub fn cursor_mode(locked: bool, visible: bool) -> GameAppEvent {
        Self::SetCursorMode(locked, visible)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub struct GameThread<S: AppState> {
    _thread: JoinHandle<()>,
    render_event_tx: mpsc::Sender<RenderAppEvent>,
    game_event_rx: mpsc::Receiver<GameAppEvent>,

    _state: PhantomData<S>,
}

#[cfg(target_arch = "wasm32")]
pub struct GameThread<S: AppState> {
    thread: GameThreadInner<S>,
    render_event_tx: mpsc::Sender<RenderAppEvent>,
    game_event_rx: mpsc::Receiver<GameAppEvent>,
}

struct GameThreadInner<S: AppState> {
    world: Box<World>,
    state: S,
    render_event_rx: mpsc::Receiver<RenderAppEvent>,
    _game_event_tx: mpsc::Sender<GameAppEvent>,
}

impl<S: AppState> GameThreadInner<S> {
    #[cfg(not(target_arch = "wasm32"))]
    fn spawn(
        state: S,
        asset_store: Arc<AssetStore>,
        render_tx: mpsc::Sender<RenderMsg>,
        game_event_tx: mpsc::Sender<GameAppEvent>,
        render_event_rx: mpsc::Receiver<RenderAppEvent>,
    ) -> JoinHandle<()> {
        std::thread::spawn(move || {
            Self::spawn_local(
                state,
                asset_store,
                render_tx,
                game_event_tx,
                render_event_rx,
            )
            .run();
            log::debug!("Game thread exited");
        })
    }

    fn spawn_local(
        state: S,
        asset_store: Arc<AssetStore>,
        render_tx: mpsc::Sender<RenderMsg>,
        game_event_tx: mpsc::Sender<GameAppEvent>,
        render_event_rx: mpsc::Receiver<RenderAppEvent>,
    ) -> GameThreadInner<S> {
        let world = unsafe { World::new(asset_store, render_tx, game_event_tx.clone()) };

        GameThreadInner {
            world,
            state,
            render_event_rx,
            _game_event_tx: game_event_tx,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<S: AppState> GameThread<S> {
    pub fn new(state: S, asset_store: Arc<AssetStore>, render_tx: mpsc::Sender<RenderMsg>) -> Self {
        let (render_event_tx, render_event_rx) = mpsc::channel();
        let (game_event_tx, game_event_rx) = mpsc::channel();

        let thread = GameThreadInner::spawn(
            state,
            asset_store,
            render_tx,
            game_event_tx,
            render_event_rx,
        );

        GameThread {
            _thread: thread,
            render_event_tx,
            game_event_rx,
            _state: PhantomData,
        }
    }

    pub fn init(&self) -> bool {
        self.render_event_tx.send(RenderAppEvent::Init).is_ok()
    }

    pub fn input(&self, event: WindowEvent) -> Result<(), Box<SendError<RenderAppEvent>>> {
        self.render_event_tx
            .send(RenderAppEvent::Input(event))
            .map_err(Box::new)
    }

    pub fn device_event(
        &self,
        device_id: DeviceId,
        event: DeviceEvent,
    ) -> Result<(), Box<SendError<RenderAppEvent>>> {
        self.render_event_tx
            .send(RenderAppEvent::DeviceEvent(device_id, event))
            .map_err(Box::new)
    }

    pub fn resize(&self, size: PhysicalSize<u32>) -> Result<(), Box<SendError<RenderAppEvent>>> {
        self.render_event_tx
            .send(RenderAppEvent::Resize(size))
            .map_err(Box::new)
    }

    pub fn receive_events(&self) -> impl Iterator<Item = GameAppEvent> {
        self.game_event_rx.try_iter()
    }

    // TODO: Think about if render frame and world should be linked
    pub fn next_frame(&self) -> Result<(), Box<SendError<RenderAppEvent>>> {
        self.render_event_tx
            .send(RenderAppEvent::StartFrame)
            .map_err(Box::new)
    }
}

#[cfg(target_arch = "wasm32")]
impl<S: AppState> GameThread<S> {
    pub fn new(state: S, asset_store: Arc<AssetStore>, render_tx: mpsc::Sender<RenderMsg>) -> Self {
        let (render_event_tx, render_event_rx) = mpsc::channel();
        let (game_event_tx, game_event_rx) = mpsc::channel();

        let thread = GameThreadInner::spawn_local(
            state,
            asset_store,
            render_tx,
            game_event_tx,
            render_event_rx,
        );

        GameThread {
            thread,
            render_event_tx,
            game_event_rx,
        }
    }
    pub fn init(&mut self) -> bool {
        self.thread.init()
    }

    pub fn input(&self, event: WindowEvent) -> Result<(), Box<SendError<RenderAppEvent>>> {
        self.render_event_tx
            .send(RenderAppEvent::Input(event))
            .map_err(Box::new)
    }

    pub fn device_event(
        &self,
        device_id: DeviceId,
        event: DeviceEvent,
    ) -> Result<(), Box<SendError<RenderAppEvent>>> {
        self.render_event_tx
            .send(RenderAppEvent::DeviceEvent(device_id, event))
            .map_err(Box::new)
    }

    pub fn resize(&self, size: PhysicalSize<u32>) -> Result<(), Box<SendError<RenderAppEvent>>> {
        self.render_event_tx
            .send(RenderAppEvent::Resize(size))
            .map_err(Box::new)
    }

    pub fn receive_events(&self) -> impl Iterator<Item = GameAppEvent> {
        self.game_event_rx.try_iter()
    }

    // TODO: Think about if render frame and world should be linked
    pub fn next_frame(&self) -> Result<(), Box<SendError<RenderAppEvent>>> {
        self.render_event_tx
            .send(RenderAppEvent::StartFrame)
            .map_err(Box::new)
    }
}
impl<S: AppState> GameThreadInner<S> {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn run(mut self) {
        loop {
            if !self.pump_events() {
                break;
            }
        }
    }

    pub fn pump_events(&mut self) -> bool {
        let mut update_signaled = false;
        let mut keep_running = true;
        loop {
            let event = match self.render_event_rx.try_recv() {
                Ok(event) => event,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    info!("Window Event Loop exited. Exiting event loop.");
                    return false;
                }
            };

            keep_running = match event {
                RenderAppEvent::Init => self.init(),
                RenderAppEvent::Input(event) => self.input(event),
                RenderAppEvent::Resize(size) => self.resize(size),
                RenderAppEvent::StartFrame => {
                    update_signaled = true;
                    true
                }
                RenderAppEvent::DeviceEvent(id, event) => self.device_event(id, &event),
            };

            if !keep_running {
                break;
            }
        }

        if keep_running {
            if update_signaled {
                keep_running = self.update();
            }
        } else {
            info!("Game signaled exit. Exiting event loop.");
        }

        keep_running
    }

    pub fn init(&mut self) -> bool {
        if let Err(e) = self.state.init(&mut self.world) {
            error!("World init function hook returned: {e}");
            return false;
        }

        true
    }

    pub fn input(&mut self, event: WindowEvent) -> bool {
        self.world.input.process_event(&event);

        true
    }

    pub fn device_event(&mut self, _: DeviceId, event: &DeviceEvent) -> bool {
        self.world.input.process_device_input_event(event);

        true
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) -> bool {
        // TODO: consider updating this in the CameraComponent
        if let Some(mut cam) = self.world.active_camera().upgrade(&self.world) {
            cam.resize(size.width as f32, size.height as f32);
        }

        true
    }

    // TODO: Think about if renderer delta time should be linked to world tick time
    pub fn update(&mut self) -> bool {
        let world = self.world.as_mut();
        if world.is_shutting_down() {
            world.teardown();
            return false;
        }

        if let Err(e) = self.state.update(world) {
            error!("Error happened when calling update function hook: {e}");
        }

        world.fixed_update();
        world.update();

        if let Err(e) = self.state.late_update(world) {
            error!("Error happened when calling late update function hook: {e}");
        }

        world.post_update();

        world.next_frame();

        true
    }
}
