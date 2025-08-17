use crate::components::CameraComponent;
use crate::rendering::Renderer;
use crate::world::World;
use crate::AppState;
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
    pub fn run(mut self, event_loop: EventLoop<()>) -> Result<(), Box<dyn Error>> {
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

        let renderer = match Renderer::new(window, asset_store) {
            Ok(r) => r,
            Err(e) => {
                error!("Error when creating renderer: {e}");
                event_loop.exit();
                return;
            }
        };

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

                world.fixed_update();
                world.update();

                if let Err(e) = self.state.late_update(world, renderer.window()) {
                    error!("Error happened when calling late update function hook: {e}");
                }

                renderer.update_world(world);
                world.post_update();

                if !renderer.render_frame(world) {
                    event_loop.exit();
                }

                if let Err(e) = self.state.draw(world, renderer) {
                    error!("Error happened when calling late update function hook: {e}");
                }

                world.next_frame();
                renderer.window.request_redraw();
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                renderer.resize(size);

                if let Some(mut cam) = world
                    .active_camera
                    .and_then(|cam| cam.get_component::<CameraComponent>())
                {
                    cam.resize(size.width as f32, size.height as f32);
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
