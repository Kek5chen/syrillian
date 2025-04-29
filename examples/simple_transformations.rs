use std::error::Error;
use nalgebra::{UnitQuaternion, Vector3};
use winit::window::Window;
use syrillian::app::App;
use syrillian::scene_loader::SceneLoader;
use syrillian::world::World;

#[tokio::main]
async fn main() {
    env_logger::init();

    App::create("Simple Transformations", 800, 600)
        .with_init(Some(init))
        .run()
        .await
        .expect("Couldn't run app");
}

fn init(world: &mut World, _window: &Window) -> Result<(), Box<dyn Error>> {
    let mut scene = SceneLoader::load(world, "testmodels/simple_trans.fbx")?;
    scene.transform.set_position(Vector3::new(0.0, 0.0, -10.0));
    scene.transform.set_rotation(UnitQuaternion::from_euler_angles(0.0, 90.0, 0.0));
    scene.transform.set_uniform_scale(0.01);

    let camera = world.new_camera();

    world.add_child(camera);
    world.add_child(scene);

    world.print_objects();

    Ok(())
}
