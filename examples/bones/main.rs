use std::error::Error;

use nalgebra::Vector3;
use syrillian::{scene_loader::SceneLoader, World};
use winit::window::Window;

#[tokio::main]
async fn main() {
    env_logger::builder()
        .parse_default_env()
        .init();

    let exit_result = syrillian::App::create("Bones Example", 800, 600)
        .with_init(Some(init))
        .run()
        .await;

    if let Err(e) = exit_result {
        log::error!("Exited with error: {e}");
    }
}

fn init(world: &mut World, _window: &Window) -> Result<(), Box<dyn Error>> {
    world.new_camera();

    let mut boney_obj = SceneLoader::load(world, "./testmodels/hampter/hampter.fbx")?;
    boney_obj.name = "Boney thing".to_owned();

    boney_obj.transform.set_uniform_scale(0.01);
    boney_obj.transform.set_position(Vector3::new(0.0, -5.0, -20.0));

    world.add_child(boney_obj);
    
    Ok(())
}
