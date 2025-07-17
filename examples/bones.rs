//! Skeletal Mesh and Animation experimentation example.
//!
//! The goal of this is to test if bones are working as expected, and to
//! aid in the development in the first place.

use std::error::Error;

use syrillian::assets::scene_loader::SceneLoader;
use syrillian::{AppState, World};
use syrillian_macros::SyrillianApp;
use winit::window::Window;

// TODO: Bones don't work yet. Yes I shipped something brokey.
#[derive(Debug, Default, SyrillianApp)]
struct BonesExample;

impl AppState for BonesExample {
    fn init(&mut self, world: &mut World, _window: &Window) -> Result<(), Box<dyn Error>> {
        world.new_camera();

        let mut boney_obj = SceneLoader::load(world, "./testmodels/hampter/hampter.fbx")?;
        boney_obj.name = "Boney thing".to_owned();

        boney_obj.transform.set_uniform_scale(0.01);
        boney_obj.transform.set_position(0.0, -5.0, -20.0);

        world.add_child(boney_obj);

        Ok(())
    }
}
