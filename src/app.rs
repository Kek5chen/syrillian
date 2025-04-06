use std::error::Error;
use futures::executor::block_on;
use log::{error, info};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalSize, Size};
use winit::error::EventLoopError;
use winit::event::{DeviceEvent, DeviceId, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{WindowAttributes, WindowId};

use crate::components::CameraComp;
use crate::logichooks::{HookFunc, LogicHooks};
use crate::renderer::Renderer;
use crate::world::World;

pub struct PrematureApp {
    window_attributes: WindowAttributes,
    init_cb: Option<HookFunc>,
    update_cb: Option<HookFunc>,
    deinit_cb: Option<HookFunc>,
}

pub struct App {
    renderer: Option<Renderer>,
    world: Box<World>,
    window_attributes: WindowAttributes,
    pub hook_funcs: LogicHooks,
}

#[allow(unused)]
impl Default for PrematureApp {
    fn default() -> PrematureApp {
        PrematureApp {
            window_attributes: WindowAttributes::default()
                .with_inner_size(Size::Physical(PhysicalSize {
                    width: 800,
                    height: 600,
                }))
                .with_title("Default Window"),
            init_cb: None,
            update_cb: None,
            deinit_cb: None,
        }
    }
}

impl App {
    #[allow(unused)]
    pub fn create(title: &str, width: u32, height: u32) -> PrematureApp {
        PrematureApp {
            window_attributes: WindowAttributes::default()
                .with_inner_size(Size::Physical(PhysicalSize { width, height }))
                //.with_resizable(false)
                .with_title(title),
            init_cb: None,
            update_cb: None,
            deinit_cb: None,
        }
    }

    pub fn renderer(&self) -> &Renderer {
        self.renderer.as_ref().unwrap()
    }
}

impl PrematureApp {
    async fn init_state(&mut self) -> Result<(EventLoop<()>, App), Box<dyn Error>> {
        let event_loop = match EventLoop::new() {
            Err(EventLoopError::NotSupported(_)) => {
                return Err("No graphics backend found that could be used.".into())
            }
            e => e?,
        };
        event_loop.set_control_flow(ControlFlow::Poll);
        
        let world = unsafe { World::new() };

        let app = App {
            renderer: None,
            world,
            window_attributes: self.window_attributes.clone(),
            hook_funcs: LogicHooks {
                init: self.init_cb,
                update: self.update_cb,
                deinit: self.deinit_cb,
            }
        };

        Ok((event_loop, app))
    }

    pub fn with_init(&mut self, init: Option<HookFunc>) {
        self.init_cb = init;
    }
    
    pub fn with_update(&mut self, update: Option<HookFunc>) {
        self.update_cb = update;
    }

    pub fn with_deinit(&mut self, deinit: Option<HookFunc>) {
        self.deinit_cb = deinit;
    }
    
    pub async fn run(mut self) -> Result<(), Box<dyn Error>> {
        let (event_loop, app) = self.init_state().await?;
        app.run(event_loop).await
    }
}

impl App {
    pub async fn run(
        mut self,
        event_loop: EventLoop<()>,
    ) -> Result<(), Box<dyn Error>> {
        event_loop.run_app(&mut self).unwrap();

        Ok(())
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        info!("RESUMED!");
        let window = event_loop
            .create_window(self.window_attributes.clone())
            .unwrap();

        self.world.assets.invalidate();

        let mut renderer = block_on(Renderer::new(window));
        let state = &renderer.state;

        self.world.assets.init_runtime(state.device.clone(), state.queue.clone());

        renderer.init();

        self.renderer = Some(renderer);

        if let Some(init) = self.hook_funcs.init {
            if let Err(e) = init(&mut self.world, self.renderer.as_ref().unwrap().window()) {
                error!("World init function hook returned: {e}");
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let renderer =  self.renderer.as_mut().unwrap();
        let world = self.world.as_mut();
        if world.is_shutting_down() {
            event_loop.exit();
            return;
        }

        if window_id != renderer.window().id() {
            return;
        }

        world.input.process_event(renderer.window_mut(), &event);
        if renderer.state.input(&event) {
           return;
        }

        match event {
            WindowEvent::RedrawRequested => {
                if let Some(update_func) = self.hook_funcs.update {
                    if let Err(e) = update_func(world, renderer.window()) {
                        error!("Error happened when calling update function hook: {e}");
                    }
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
                    if let Some(cam_comp) = cam.get_component::<CameraComp>() {
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
        let renderer =  self.renderer.as_mut().unwrap();
        let world = self.world.as_mut();
        world.input.process_mouse_event(renderer.window_mut(), &event);
    }
}
