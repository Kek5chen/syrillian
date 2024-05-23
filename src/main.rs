use std::error::Error;

use cgmath::Vector3;
use env_logger::Env;
use log::{error, LevelFilter};
use winit::window::Window;

use crate::app::App;
use crate::asset_management::mesh::Mesh;
use crate::buffer::{CUBE, CUBE_INDICES};
use crate::logichooks::LogicHooks;
use crate::mesh_renderer::MeshRenderer;
use crate::world::World;

mod app;
mod asset_management;
mod buffer;
mod components;
mod drawable;
mod logichooks;
mod mesh_renderer;
mod object;
mod renderer;
mod scene_loader;
mod state;
mod transform;
mod world;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let log_env = Env::new().filter("RUST_LOG");
    env_logger::builder()
        .parse_default_env() // Default env
        .filter_level(LevelFilter::Info) // Use at least info level
        .parse_env(log_env) // Or override with whatever env says
        .init();

    let hooks = LogicHooks {
        init: Some(init),
        update: Some(update),
        deinit: None,
    };
    let app = App::create("game-rs", 800, 600);
    if let Err(e) = app.run(hooks).await {
        error!("{e}")
    }

    Ok(())
}

fn init(world: &mut World, window: &Window) -> Result<(), Box<dyn Error>> {
    let obj1 = world.new_object("Mow");
    let obj2 = world.new_object("Meoow");
    let camera = world.new_camera();

    camera
        .borrow_mut()
        .transform
        .set_position(Vector3::new(0.0, 1.0, 5.0));

    let mesh = Mesh::new(CUBE.to_vec(), Some(CUBE_INDICES.to_vec()));
    let mesh_id = world.assets.meshes.add_mesh(mesh);
    obj2.borrow_mut()
        .set_drawable(Some(MeshRenderer::new(mesh_id)));
    obj1.borrow_mut().add_child(obj2);
    world.add_child(obj1);
    world.add_child(camera);

    world.print_objects();

    Ok(())
}

fn update(world: &mut World, window: &Window) -> Result<(), Box<dyn Error>> {
    window.set_title(&format!(
        "{} - v.{} - built on {} at {} - FPS: [ {} ] #{}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        env!("BUILD_DATE"),
        env!("BUILD_TIME"),
        (1.0 / world.get_delta_time().as_secs_f32()) as u32,
        env!("GIT_HASH"),
    ));
    
    Ok(())
}
