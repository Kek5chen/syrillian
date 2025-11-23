use crate::world::World;
use crate::{AppRuntime, AppSettings};
use std::error::Error;
use winit::dpi::{PhysicalSize, Size};
use winit::window::WindowAttributes;

#[allow(unused)]
pub trait AppState: Sized + Send + 'static {
    fn init(&mut self, world: &mut World) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    fn update(&mut self, world: &mut World) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    fn late_update(&mut self, world: &mut World) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    fn destroy(&mut self, world: &mut World) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

impl<S: AppState> AppRuntime for S {
    fn configure(self, title: &str, width: u32, height: u32) -> AppSettings<Self> {
        AppSettings {
            windows: vec![
                WindowAttributes::default()
                    .with_inner_size(Size::Physical(PhysicalSize { width, height }))
                    .with_title(title),
            ],
            state: self,
        }
    }

    fn default_config(self) -> AppSettings<Self> {
        AppSettings {
            windows: vec![
                WindowAttributes::default()
                    .with_inner_size(Size::Physical(PhysicalSize {
                        width: 800,
                        height: 600,
                    }))
                    .with_title("Syrillian Window"),
            ],
            state: self,
        }
    }
}
