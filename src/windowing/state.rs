use std::error::Error;
use winit::dpi::{PhysicalSize, Size};
use winit::window::{Window, WindowAttributes};
use crate::{AppRuntime, AppSettings};
use crate::world::World;

#[allow(unused)]
pub trait AppState: Sized {
    fn init(&mut self, world: &mut World, window: &Window) -> Result<(), Box<dyn Error>> { Ok(()) }
    fn update(&mut self, world: &mut World, window: &Window) -> Result<(), Box<dyn Error>> { Ok(()) }
    fn destroy(&mut self, world: &mut World, window: &Window) -> Result<(), Box<dyn Error>> { Ok(()) }
}

impl<S: AppState> AppRuntime for S {
    fn configure(self, title: &str, width: u32, height: u32) -> AppSettings<Self> {
        AppSettings {
            window: WindowAttributes::default()
                .with_inner_size(Size::Physical(PhysicalSize { width, height }))
                //.with_resizable(false)
                .with_title(title),
            state: self,
        }
    }

    fn default_config(self) -> AppSettings<Self> {
        AppSettings {
            window: WindowAttributes::default()
                .with_inner_size(Size::Physical(PhysicalSize {
                    width: 800,
                    height: 600,
                }))
                .with_title("Syrillian Window"),
            state: self,
        }
    }
}
