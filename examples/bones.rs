use std::error::Error;

use nalgebra::Vector3;
use syrillian::{AppState, World};
use syrillian::assets::scene_loader::SceneLoader;
use winit::window::Window;
use syrillian_macros::SyrillianApp;

#[derive(Debug, Default, SyrillianApp)]
struct BonesExample;

impl AppState for BonesExample {
    fn init(&mut self, world: &mut World, _window: &Window) -> Result<(), Box<dyn Error>> {
        world.new_camera();

        let mut boney_obj = SceneLoader::load(world, "./testmodels/hampter/hampter.fbx")?;
        boney_obj.name = "Boney thing".to_owned();

        boney_obj.transform.set_uniform_scale(0.01);
        boney_obj
            .transform
            .set_position(Vector3::new(0.0, -5.0, -20.0));

        world.add_child(boney_obj);

        Ok(())
    }
}
