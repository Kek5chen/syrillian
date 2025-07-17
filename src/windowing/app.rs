use crate::AppState;
use crate::components::CameraComponent;
use crate::rendering::Renderer;
use crate::world::World;
use futures::executor::block_on;
use log::{error, info};
use std::error::Error;
use winit::application::ApplicationHandler;
use winit::error::EventLoopError;
use winit::event::{DeviceEvent, DeviceId, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{WindowAttributes, WindowId};

pub struct App<S: AppState> {
    renderer: Option<Renderer>,
    world: Box<World>,
    window_attributes: WindowAttributes,
    state: S,
}

pub struct AppSettings<S: AppState> {
    pub window: WindowAttributes,
    pub state: S,
}

pub trait AppRuntime: AppState {
    fn configure(self, title: &str, width: u32, height: u32) -> AppSettings<Self>;

    fn default_config(self) -> AppSettings<Self>;
}

impl<S: AppState> App<S> {
    pub fn renderer(&self) -> &Renderer {
        self.renderer.as_ref().unwrap()
    }
}

impl<S: AppState> AppSettings<S> {
    pub async fn run(self) -> Result<(), Box<dyn Error>> {
        let (event_loop, app) = self.init_state().await?;
        app.run(event_loop).await
    }

    async fn init_state(self) -> Result<(EventLoop<()>, App<S>), Box<dyn Error>> {
        let event_loop = match EventLoop::new() {
            Err(EventLoopError::NotSupported(_)) => {
                return Err("No graphics backend found that could be used.".into());
            }
            e => e?,
        };
        event_loop.set_control_flow(ControlFlow::Poll);

        let world = unsafe { World::new() };

        let app = App {
            renderer: None,
            world,
            window_attributes: self.window,
            state: self.state,
        };

        Ok((event_loop, app))
    }
}

impl<S: AppState> App<S> {
    pub async fn run(mut self, event_loop: EventLoop<()>) -> Result<(), Box<dyn Error>> {
        event_loop.run_app(&mut self)?;
        Ok(())
    }
}

impl<S: AppState> ApplicationHandler for App<S> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        info!("(Re)initializing render state!");
        let window = event_loop
            .create_window(self.window_attributes.clone())
            .unwrap();

        let asset_store = self.world.assets.clone();

        let mut renderer = match block_on(Renderer::new(window, asset_store)) {
            Ok(r) => r,
            Err(e) => {
                error!("Error when creating renderer: {e}");
                event_loop.exit();
                return;
            }
        };

        renderer.init();

        if let Err(e) = self.state.init(&mut self.world, renderer.window()) {
            panic!("World init function hook returned: {e}");
        }

        self.world.initialize_runtime(&renderer);

        self.renderer = Some(renderer);
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

        let Some(renderer) = self.renderer.as_mut() else {
            error!("No renderer.");
            return;
        };
        let world = self.world.as_mut();
        if world.is_shutting_down() {
            event_loop.exit();
            return;
        }

        if window_id != renderer.window().id() {
            return;
        }

        world.input.process_event(renderer.window_mut(), &event);

        match event {
            WindowEvent::RedrawRequested => {
                if let Err(e) = self.state.update(world, renderer.window()) {
                    error!("Error happened when calling update function hook: {e}");
                }

                world.update();
                renderer.state.update();
                if !renderer.render_world(world) {
                    event_loop.exit();
                }
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                renderer.resize(size);

                // For I have sinned, this now becomes my recovery.
                // I was forgiven, shall it come haunt me later.
                if let Some(cam) = world.active_camera {
                    if let Some(cam_comp) = cam.get_component::<CameraComponent>() {
                        if let Ok(mut comp) = cam_comp.try_borrow_mut() {
                            comp.resize(size.width as f32, size.height as f32);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        let renderer = self.renderer.as_mut().unwrap();
        let world = self.world.as_mut();
        world
            .input
            .process_device_input_event(renderer.window_mut(), &event);
    }
}
