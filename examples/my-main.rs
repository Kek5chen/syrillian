//! A showcase of various engine features.
//!
//! w: I use this as my main test environment, which allows me to expand this and experiment
//!    with new features. Therefore, it should contain the latest and greatest. I can recommend
//!    using this for reference.

use std::error::Error;
use syrillian::assets::Material;
use syrillian::assets::scene_loader::SceneLoader;
use syrillian::components::{Collider3D, PointLightComponent, RigidBodyComponent, RopeComponent};
use syrillian::core::GameObjectId;
use syrillian::prefabs::CubePrefab;
use syrillian::prefabs::first_person_player::FirstPersonPlayerPrefab;
use syrillian::prefabs::prefab::Prefab;
use syrillian::utils::frame_counter::FrameCounter;
use syrillian::{AppState, World};
use syrillian::SyrillianApp;
use winit::window::Window;

const NECO_IMAGE: &[u8; 1293] = include_bytes!("assets/neco.jpg");

#[derive(Debug, Default, SyrillianApp)]
struct MyMain {
    frame_counter: FrameCounter,
}

impl AppState for MyMain {
    fn init(&mut self, world: &mut World, _window: &Window) -> Result<(), Box<dyn Error>> {
        world.input.set_auto_cursor_lock(true);
        world.input.set_quit_on_escape(true);

        world.spawn(&City);
        let mut player = world.spawn(&FirstPersonPlayerPrefab);
        player.transform.set_position(0.0, 20.0, 0.0);

        let texture = world.assets.textures.load_image_from_memory(NECO_IMAGE)?;
        let neco_material = world.assets.materials.add(
            Material::builder()
                .name("Neco Arc".into())
                .diffuse_texture(texture)
                .opacity(1.0)
                .build(),
        );

        let cube_prefab = CubePrefab::new(neco_material);
        let mut cube = world.spawn(&cube_prefab);
        let mut cube2 = world.spawn(&cube_prefab);

        cube.transform.set_position(20.0, 20.9, -20.0);
        cube2.transform.set_position(5.0, 20.9, -20.0);

        cube.add_component::<PointLightComponent>();
        cube.add_component::<Collider3D>()
            .get_collider_mut()
            .unwrap()
            .set_mass(1.0);
        cube.add_component::<RigidBodyComponent>();
        cube.add_component::<RopeComponent>().connect_to(player);

        cube2.add_component::<PointLightComponent>();
        cube2
            .add_component::<Collider3D>()
            .get_collider_mut()
            .unwrap()
            .set_mass(1.0);
        cube2.add_component::<RigidBodyComponent>();
        cube2.add_component::<RopeComponent>().connect_to(player);

        world.add_child(cube);
        world.add_child(cube2);

        world.print_objects();

        Ok(())
    }

    fn update(&mut self, world: &mut World, window: &Window) -> Result<(), Box<dyn Error>> {
        self.frame_counter.new_frame_from_world(world);
        window.set_title(&self.format_title());

        Ok(())
    }
}

impl MyMain {
    fn format_title(&self) -> String {
        let debug_or_release = if cfg!(debug_assertions) {
            "[DEBUG]"
        } else {
            "[RELEASE]"
        };

        format!(
            "{} {} - FPS: [ {} ]",
            debug_or_release,
            syrillian::ENGINE_STR,
            self.frame_counter.fps(),
        )
    }

}

pub struct City;
impl Prefab for City {
    fn prefab_name(&self) -> &'static str {
        "City"
    }

    fn build(&self, world: &mut World) -> GameObjectId {
        let Ok(mut city) = SceneLoader::load(world, "./testmodels/testmap/testmap.fbx") else {
            panic!(
                "Failed to load the city file. Please run this example from the project root directory."
            );
        };

        city.transform.set_uniform_scale(0.01);

        // add colliders to city
        city.add_child_components_then(Collider3D::please_use_mesh);

        city
    }
}
