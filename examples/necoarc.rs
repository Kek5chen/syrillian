use std::error::Error;

use syrillian::assets::Material;
use syrillian::components::RotateComponent;
use syrillian::drawables::{Image, ImageScalingMode};
use winit::window::Window;
use syrillian::{AppState, World};
use syrillian::prefabs::CubePrefab;
use syrillian_macros::SyrillianApp;

const NECO_IMAGE: &[u8; 1293] = include_bytes!("assets/neco.jpg");

#[derive(Debug, Default, SyrillianApp)]
struct NecoArc;

impl AppState for NecoArc {
    fn init(&mut self, world: &mut World, _window: &Window) -> Result<(), Box<dyn Error>> {
        world.input.set_quit_on_escape(true);
        world.new_camera();

        let texture = world.assets.textures.load_image_from_memory(NECO_IMAGE)?;

        let material = world.assets.materials.add(Material::builder()
            .name("Neco Arc".into())
            .diffuse_texture(texture)
            .opacity(1.0)
            .build());

        let mut neco = world.spawn(&CubePrefab::new(material));
        neco.add_component::<RotateComponent>();
        neco.transform.set_position(0.0, 0.0, -5.0);

        let mut image_obj = world.new_object("Image");
        image_obj.set_drawable(Image::new_with_size(
            material,
            ImageScalingMode::RelativeStretch {
                left: 0.0,
                right: 1.0,
                top: 1.0,
                bottom: 0.666,
            },
        ));
        world.add_child(image_obj);

        let mut image_obj = world.new_object("Image 2");
        image_obj.set_drawable(Image::new_with_size(
            material,
            ImageScalingMode::RelativeStretch {
                left: 0.0,
                right: 1.0,
                top: 0.333,
                bottom: 0.0,
            },
        ));
        world.add_child(image_obj);

        Ok(())
    }
}
