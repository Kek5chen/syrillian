//! A showcase of various engine features.
//!
//! w: I use this as my main test environment, which allows me to expand this and experiment
//!    with new features. Therefore, it should contain the latest and greatest. I can recommend
//!    using this for reference.

use std::error::Error;
use syrillian::assets::scene_loader::SceneLoader;
use syrillian::assets::{Material, Shader};
use syrillian::components::{
    Collider3D, PointLightComponent, RigidBodyComponent, RopeComponent, RotateComponent,
};
use syrillian::core::GameObjectId;
use syrillian::prefabs::first_person_player::FirstPersonPlayerPrefab;
use syrillian::prefabs::prefab::Prefab;
use syrillian::prefabs::sphere::SpherePrefab;
use syrillian::prefabs::CubePrefab;
use syrillian::utils::frame_counter::FrameCounter;
use syrillian::SyrillianApp;
use syrillian::{AppState, World};
use wgpu::PolygonMode;
use winit::window::Window;
// const NECO_IMAGE: &[u8; 1293] = include_bytes!("assets/neco.jpg");

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

        let fs = include_str!("dynamic_shader/shader2.wgsl");
        let fs2 = include_str!("dynamic_shader/shader.wgsl");
        let code = include_str!("../src/engine/rendering/shaders/default_vertex3d.wgsl");

        let shader = world.assets.shaders.add(Shader::Default {
            name: "Funky Shader".to_string(),
            code: fs.to_string() + code,
            polygon_mode: PolygonMode::Fill,
        });

        let shader2 = world.assets.shaders.add(Shader::Default {
            name: "Funky Shader 2".to_string(),
            code: fs2.to_string() + code,
            polygon_mode: PolygonMode::Fill,
        });

        let shader_mat_1 = world.assets.materials.add(
            Material::builder()
                .name("Neco Arc".into())
                .opacity(1.0)
                .shader(shader)
                .build(),
        );
        let shader_mat_2 = world.assets.materials.add(
            Material::builder()
                .name("Neco Arc".into())
                .opacity(1.0)
                .shader(shader2)
                .build(),
        );


        let cube_prefab1 = CubePrefab::new(shader_mat_1);
        let cube_prefab2 = CubePrefab::new(shader_mat_2);
        let mut cube = world.spawn(&cube_prefab1);
        let mut cube2 = world.spawn(&cube_prefab1);
        let mut cube3 = world.spawn(&cube_prefab1);
        let mut big_cube_left = world.spawn(&cube_prefab2);
        let mut big_cube_right = world.spawn(&cube_prefab2);

        cube.transform.set_position(20.0, 3.9, -20.0);
        cube2.transform.set_position(5.0, 6.9, -20.0);
        cube3.transform.set_position(5.0, 3.9, -20.0);
        big_cube_left.transform.set_position(10.0, 20.0, 20.0);
        big_cube_right.transform.set_position(-10.0, 20.0, 20.0);

        cube.add_component::<PointLightComponent>();
        cube2.add_component::<PointLightComponent>();
        cube3.add_component::<PointLightComponent>();

        cube.add_component::<RotateComponent>().scale_coefficient = 1.;
        big_cube_left.add_component::<RotateComponent>().rotate_speed = 30.;
        big_cube_right.add_component::<RotateComponent>().rotate_speed = -30.;
        big_cube_left.transform.set_uniform_scale(5.);
        big_cube_right.transform.set_uniform_scale(5.);

        let collider = cube2
            .add_component::<Collider3D>()
            .get_collider_mut()
            .unwrap();
        collider.set_mass(1.0);
        collider.set_restitution(0.9);
        let rb = cube2
            .add_component::<RigidBodyComponent>()
            .get_body_mut()
            .unwrap();
        rb.set_gravity_scale(0.0, false);
        rb.set_angular_damping(0.5);
        rb.set_linear_damping(0.5);
        rb.enable_ccd(true);

        cube3
            .add_component::<RigidBodyComponent>()
            .get_body_mut()
            .unwrap()
            .enable_ccd(true);
        cube3.add_component::<Collider3D>();
        cube3.add_component::<RopeComponent>().connect_to(cube2);

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
