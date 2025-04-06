use env_logger::Env;
use log::{error, LevelFilter};
use nalgebra::Vector3;
use rapier3d::prelude::*;
use std::any::Any;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::error::Error;
use std::sync::Mutex;
use wgpu::TextureFormat;
use winit::event::MouseButton;
use winit::keyboard::KeyCode;
use winit::window::{CursorGrabMode, Window};

use crate::camera_controller::CameraController;
use crate::player_movement::PlayerMovement;
use syrillian::app::App;
use syrillian::asset_management::materialmanager::Material;
use syrillian::asset_management::mesh::Mesh;
use syrillian::asset_management::shadermanager::DIM3_SHADER_ID;
use syrillian::buffer::{CUBE, CUBE_INDICES};
use syrillian::components::collider::MeshShapeExtra;
use syrillian::components::{Collider3D, RigidBodyComponent, RotateComponent};
use syrillian::drawables::mesh_renderer::MeshRenderer;
use syrillian::scene_loader::SceneLoader;
use syrillian::world::World;

mod camera_controller;
mod player_movement;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let log_env = Env::new().filter("RUST_LOG");
    env_logger::builder()
        .parse_default_env() // Default env
        .filter_level(LevelFilter::Info) // Use at least info level
        .parse_env(log_env) // Or override with whatever env says
        .init();

    let mut app = App::create("SYRILLIAN", 800, 600);
    app.with_init(Some(funnyinit));
    app.with_update(Some(update));

    if let Err(e) = app.run().await {
        error!("{e}");
    }

    Ok(())
}

fn funnyinit(world: &mut World, _window: &Window) -> Result<(), Box<dyn Error>> {
    // add city
    let mut city = SceneLoader::load(world, "./testmodels/testmap/testmap.fbx")?;

    city.transform.set_uniform_scale(0.01);

    // add colliders to city
    for child in &mut city.children {
        let collider = child.add_component::<Collider3D>();
        let drawable = &child.drawable;
        let renderer = match match drawable {
            None => continue,
            Some(renderer) => (renderer.as_ref() as &dyn Any).downcast_ref::<MeshRenderer>(),
        } {
            None => continue,
            Some(renderer) => renderer,
        };

        let collider = collider.get_collider_mut();
        let shape = SharedShape::mesh(renderer.mesh()).unwrap();
        collider.unwrap().set_shape(shape)
    }

    world.add_child(city);

    // Prepare camera
    let mut camera = world.new_camera();
    camera.add_component::<CameraController>();
    camera.transform.set_position(Vector3::new(0.0, 1., 0.0));

    // Prepare character controller
    let mut char_controller = world.new_object("CharacterController");
    char_controller
        .transform
        .set_position(Vector3::new(0.0, 100.0, 0.0));

    let collider = char_controller.add_component::<Collider3D>();
    collider
        .get_collider_mut()
        .unwrap()
        .set_shape(SharedShape::capsule_y(1.0, 0.25));

    let _rigid_body = char_controller.add_component::<RigidBodyComponent>();
    char_controller.add_component::<PlayerMovement>();

    char_controller.add_child(camera);
    world.add_child(char_controller);

    world.input.set_mouse_mode(true);

    const NECO_ARC_JPG: &[u8; 1293] = include_bytes!("neco.jpg");

    let diffuse_image = image::load_from_memory(NECO_ARC_JPG)?;
    let rgba = diffuse_image.into_rgba8();
    let mut data = Vec::with_capacity((rgba.width() * rgba.height() * 4) as usize);
    for pixel in rgba.pixels() {
        data.push(pixel[2]); // B
        data.push(pixel[1]); // G
        data.push(pixel[0]); // R
        data.push(pixel[3]); // A
    }
    let neco_arc_tex = world.assets.textures.add_texture(
        rgba.width(),
        rgba.height(),
        TextureFormat::Bgra8UnormSrgb,
        Some(data),
    );

    let neco_material = world.assets.materials.add_material(Material {
        name: "necoarc".to_string(),
        diffuse: Vector3::new(1.0, 1.0, 1.0),
        diffuse_texture: Some(neco_arc_tex),
        normal_texture: None,
        shininess: 0.0,
        shininess_texture: None,
        opacity: 1.0,
        shader: DIM3_SHADER_ID,
    });

    let cube_mesh = world.assets.meshes.add_mesh(Mesh::new(
        CUBE.to_vec(),
        Some(CUBE_INDICES.to_vec()),
        Some(vec![(neco_material, 0..CUBE_INDICES.len() as u32)]),
    ));

    let mut cube = world.new_object("Cube");
    let _ = cube.drawable.insert(MeshRenderer::new(cube_mesh));
    cube.transform.set_position(Vector3::new(20.0, -3.9, -40.0));

    cube.add_component::<RotateComponent>();

    world.add_child(cube);

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

    if world.input.is_key_down(KeyCode::Escape) {
        if world.input.get_mouse_mode() == CursorGrabMode::None {
            world.shutdown();
        } else {
            world.input.set_mouse_mode(false);
        }
    }

    if world.input.is_button_pressed(MouseButton::Left)
        || world.input.is_button_pressed(MouseButton::Right)
    {
        world.input.set_mouse_mode(true);
    }

    Ok(())
}
