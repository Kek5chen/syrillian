use std::error::Error;

use log::error;
use nalgebra::Vector3;
use syrillian::{asset_management::{materialmanager::Material, Mesh, DIM3_SHADER_ID}, buffer::{CUBE, CUBE_INDICES}, components::RotateComponent, drawables::{Image, MeshRenderer}, App, World};
use winit::window::Window;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let mut app = App::create("Neco Arc", 800, 600);
    app.with_init(Some(init));

    if let Err(e) = app.run().await {
        error!("{e}");
    }

    Ok(())
}

fn init(world: &mut World, _window: &Window) -> Result<(), Box<dyn Error>> {
    world.new_camera();
    
    let mut neco = world.new_object("Neco Arc");
    
    const NECO_ARC_JPG: &[u8; 1293] = include_bytes!("../neco.jpg");

    let texture = world.assets.textures.load_image_from_memory(NECO_ARC_JPG)?;
    let material = world.assets.materials.add_material(Material {
        name: "Neco Arc".into(),
        diffuse: Vector3::zeros(),
        diffuse_texture: Some(texture),
        normal_texture: None,
        shininess: 0.0,
        shininess_texture: None,
        opacity: 1.0,
        shader: Some(DIM3_SHADER_ID),
    });
    let mesh = world.assets.meshes.add_mesh(
        Mesh::new(
            CUBE.to_vec(), 
            Some(CUBE_INDICES.to_vec()), 
            Some(vec![(material, 0..CUBE_INDICES.len() as u32)])
        )
    );

    let drawable = MeshRenderer::new(mesh);
    neco.set_drawable(Some(drawable));

    neco.add_component::<RotateComponent>();

    neco.transform.set_position(Vector3::new(0.0, 0.0, -5.0));

    world.add_child(neco);

    let mut image_obj = world.new_object("Image");
    let image = Image::new(material);

    image_obj.set_drawable(Some(image));

    world.add_child(image_obj);


    Ok(())
}
