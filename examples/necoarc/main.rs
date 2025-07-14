use std::error::Error;

use log::{LevelFilter, error};
use nalgebra::Vector3;
use syrillian::components::RotateComponent;
use syrillian::core::Bones;
use syrillian::drawables::{Image, ImageScalingMode, MeshRenderer};
use syrillian::utils::{CUBE_IDX, CUBE_VERT};
use syrillian::{App, World};
use winit::window::Window;
use syrillian::assets::{HShader, Material, Mesh};

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(LevelFilter::Info)
        .parse_default_env()
        .init();

    let app = App::create("Neco Arc", 800, 600).with_init(Some(init));

    if let Err(e) = app.run().await {
        error!("{e}");
    }
}

fn init(world: &mut World, _window: &Window) -> Result<(), Box<dyn Error>> {
    world.new_camera();

    let mut neco = world.new_object("Neco Arc");

    const NECO_ARC_JPG: &[u8; 1293] = include_bytes!("../neco.jpg");

    let texture = world.assets.textures.load_image_from_memory(NECO_ARC_JPG)?;
    let material = world.assets.materials.add(Material {
        name: "Neco Arc".into(),
        diffuse: Vector3::zeros(),
        diffuse_texture: Some(texture),
        normal_texture: None,
        shininess: 0.0,
        shininess_texture: None,
        opacity: 1.0,
        shader: Some(HShader::DIM3),
    });
    let mesh = world.assets.meshes.add(Mesh::new(
        CUBE_VERT.to_vec(),
        Some(CUBE_IDX.to_vec()),
        Some(vec![(material, 0..CUBE_IDX.len() as u32)]),
        Bones::default(),
    ));

    let drawable = MeshRenderer::new(mesh);
    neco.set_drawable(Some(drawable));
    neco.add_component::<RotateComponent>();
    neco.transform.set_position(Vector3::new(0.0, 0.0, -5.0));
    world.add_child(neco);

    let mut image_obj = world.new_object("Image");
    image_obj.set_drawable(Some(Image::new_with_size(
        material,
        ImageScalingMode::RelativeStretch {
            left: 0.0,
            right: 1.0,
            top: 1.0,
            bottom: 0.666,
        },
    )));
    world.add_child(image_obj);

    let mut image_obj = world.new_object("Image 2");
    image_obj.set_drawable(Some(Image::new_with_size(
        material,
        ImageScalingMode::RelativeStretch {
            left: 0.0,
            right: 1.0,
            top: 0.333,
            bottom: 0.0,
        },
    )));
    world.add_child(image_obj);

    Ok(())
}
