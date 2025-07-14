use std::cell::RefCell;
use std::collections::VecDeque;
use std::error::Error;
use std::sync::Mutex;

use log::{LevelFilter, error};
use nalgebra::Vector3;
use winit::window::Window;

use syrillian::app::App;
use syrillian::assets::scene_loader::SceneLoader;
use syrillian::components::RotateComponent;
use syrillian::world::World;

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(LevelFilter::Info) // Use at least info level
        .parse_default_env()
        .init();

    let app = App::create("SYRILLIAN", 800, 600)
        .with_init(Some(init))
        .with_update(Some(update));

    if let Err(e) = app.run().await {
        error!("{e}");
    }
}

fn init(world: &mut World, _window: &Window) -> Result<(), Box<dyn Error>> {
    let mut obj2 = SceneLoader::load(world, "testmodels/parenting_and_object_types.fbx")?;
    let mut obj1 = world.new_object("Mow");
    let mut camera = world.new_camera();

    camera.transform.set_position(Vector3::new(0.0, 1.0, 50.0));

    obj2.transform.set_uniform_scale(0.03);
    obj2.add_component::<RotateComponent>();
    obj1.add_child(obj2);
    world.add_child(obj1);
    world.add_child(camera);

    world.print_objects();

    Ok(())
}

static LAST_FRAME_TIMES: Mutex<RefCell<VecDeque<f32>>> = Mutex::new(RefCell::new(VecDeque::new()));
const RUNNING_SIZE: usize = 60;

fn update(world: &mut World, window: &Window) -> Result<(), Box<dyn Error>> {
    let last_times = LAST_FRAME_TIMES.lock()?;
    let mut last_times = last_times.borrow_mut();

    let frame_time = world.get_delta_time().as_secs_f32();
    if last_times.len() >= RUNNING_SIZE {
        last_times.pop_front();
    }
    last_times.push_back(frame_time);

    let mean_delta_time: f32 = last_times.iter().sum::<f32>() / last_times.len() as f32;
    window.set_title(&format!(
        "{} - v.{} - built on {} at {} - FPS: [ {} ] #{}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        env!("BUILD_DATE"),
        env!("BUILD_TIME"),
        (1.0 / mean_delta_time) as u32,
        env!("GIT_HASH"),
    ));

    Ok(())
}
