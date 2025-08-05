//! A showcase of various engine features.
//!
//! w: I use this as my main test environment, which allows me to expand this and experiment
//!    with new features. Therefore, it should contain the latest and greatest. I can recommend
//!    using this for reference.

use gilrs::Button;
use log::info;
use nalgebra::{UnitQuaternion, Vector3};
use rapier3d::parry::query::Ray;
use rapier3d::prelude::QueryFilter;
use std::error::Error;

use syrillian::assets::scene_loader::SceneLoader;
use syrillian::assets::{HMaterial, StoreType};
use syrillian::assets::{Material, Shader};
use syrillian::components::{Collider3D, FirstPersonCameraController, PointLightComponent, RigidBodyComponent, RopeComponent, RotateComponent};
use syrillian::core::{GameObjectExt, GameObjectId};
use syrillian::prefabs::first_person_player::FirstPersonPlayerPrefab;
use syrillian::prefabs::prefab::Prefab;
use syrillian::prefabs::CubePrefab;
use syrillian::utils::frame_counter::FrameCounter;
use syrillian::SyrillianApp;
use syrillian::{AppState, World};
use winit::event::MouseButton;
use winit::keyboard::KeyCode;
use winit::window::Window;
// const NECO_IMAGE: &[u8; 1293] = include_bytes!("assets/neco.jpg");

const SHADER1: &str = include_str!("dynamic_shader/shader.wgsl");
const SHADER2: &str = include_str!("dynamic_shader/shader2.wgsl");

#[derive(Debug, SyrillianApp)]
struct MyMain {
    frame_counter: FrameCounter,
    player: GameObjectId,
    picked_up: Option<GameObjectId>,
}

impl Default for MyMain {
    fn default() -> Self {
        Self {
            frame_counter: FrameCounter::default(),
            player: GameObjectId::invalid(),
            picked_up: None,
        }
    }
}

impl AppState for MyMain {
    fn init(&mut self, world: &mut World, _window: &Window) -> Result<(), Box<dyn Error>> {
        world.audio_scene.load_sound("pop", "examples/assets/pop.wav");

        world.input.set_auto_cursor_lock(true);
        world.input.set_quit_on_escape(true);

        world.spawn(&City);
        self.player = world.spawn(&FirstPersonPlayerPrefab);
        self.player.at(0.0, 20.0, 0.0);

        let shader = Shader::new_fragment("Funky Shader", SHADER1).store(world);
        let shader2 = Shader::new_fragment("Funky Shader 2", SHADER2).store(world);

        let shader_mat_1 = Material::builder()
            .name("Cube Material 1".into())
            .shader(shader)
            .store(&world);
        let shader_mat_2 = Material::builder()
            .name("Cube Material 2".into())
            .shader(shader2)
            .store(&world);

        let cube_prefab1 = CubePrefab::new(shader_mat_1);
        let cube_prefab2 = CubePrefab::new(shader_mat_2);

        let mut big_cube_left = world.spawn(&cube_prefab1);
        let mut big_cube_right = world.spawn(&cube_prefab1);
        let mut cube = world.spawn(&cube_prefab2);
        let mut cube2 = world.spawn(&cube_prefab2);
        let mut cube3 = world.spawn(&cube_prefab2);

        cube.at(20., 3.9, -20.)
            .build_component::<PointLightComponent>()
            .build_component::<Collider3D>()
            .mass(1.0)
            .restitution(0.9)
            .build_component::<RotateComponent>()
            .scaling(1.);

        cube2
            .at(5.0, 6.9, -20.0)
            .build_component::<PointLightComponent>()
            .build_component::<Collider3D>()
            .mass(1.0)
            .restitution(0.9)
            .build_component::<RigidBodyComponent>()
            .enable_ccd()
            .gravity_scale(0.0)
            .angular_damping(0.5)
            .linear_damping(0.5);

        cube3
            .at(5.0, 3.9, -20.0)
            .build_component::<PointLightComponent>()
            .build_component::<Collider3D>()
            .build_component::<RigidBodyComponent>()
            .enable_ccd()
            .build_component::<RopeComponent>()
            .connect_to(cube2);

        big_cube_left
            .at(10.0, 20.0, 20.0)
            .scale(5.)
            .build_component::<RotateComponent>()
            .speed(30.);

        big_cube_right
            .at(-10.0, 20.0, 20.0)
            .scale(5.)
            .build_component::<RotateComponent>()
            .speed(-30.);

        world.print_objects();

        Ok(())
    }

    fn update(&mut self, world: &mut World, window: &Window) -> Result<(), Box<dyn Error>> {
        self.frame_counter.new_frame_from_world(world);
        window.set_title(&self.format_title());



        let mut zoom_down = world.input.gamepad.button(Button::LeftTrigger2);
        if world.input.is_button_pressed(MouseButton::Right) {
            zoom_down = 1.0;
        }

        if let Some(camera) = world.active_camera {
            if let Some(camera) = camera.get_component::<FirstPersonCameraController>() {
                let mut camera = camera.borrow_mut();
                camera.set_zoom(zoom_down);
            }
        }

        self.do_raycast_test(world);


        // If q is pressed, emit a sound at the origin
        if world.input.is_key_down(KeyCode::KeyQ) {
            let origin = Vector3::new(0.0,0.0,0.0);
            world.audio_scene.play_sound("pop", origin);
        }

        // Set receiver source to the camera / player
        world.audio_scene.set_receiver_position(world.active_camera.unwrap().transform.position());

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

    fn do_raycast_test(&mut self, world: &mut World) -> Option<()> {
        let camera = world.active_camera?;

        let pick_up = world.input.gamepad.is_button_down(Button::RightTrigger)
            || world.input.is_button_down(MouseButton::Left);
        let drop = world.input.gamepad.is_button_released(Button::RightTrigger)
            || world.input.is_button_released(MouseButton::Left);

        if pick_up {
            let ray = Ray::new(
                camera.transform.position().into(),
                camera.transform.forward(),
            );
            let player_collider = self
                .player
                .get_component::<Collider3D>()?
                .borrow()
                .phys_handle;
            let intersect = world.physics.cast_ray(
                &ray,
                5.,
                false,
                QueryFilter::only_dynamic().exclude_collider(player_collider),
            );

            #[cfg(debug_assertions)]
            {
                use syrillian::components::CameraComponent;
                if let Some(camera) = camera.get_component::<CameraComponent>() {
                    camera.borrow_mut().push_debug_ray(ray, 5.);
                }
            }

            match intersect {
                None => info!("No ray intersection"),
                Some((dt, obj)) => {
                    info!("Intersection after {dt}s, against: {}", obj.name);
                    self.picked_up = Some(obj);
                }
            }
        } else if drop {
            if let Some(obj) = self.picked_up {
                let rb = obj.get_component::<RigidBodyComponent>()?;
                rb.borrow_mut().set_kinematic(false);
            }
            self.picked_up = None;
        }

        if let Some(mut obj) = self.picked_up {
            let scale = obj.transform.scale();
            let target_position = camera.transform.position()
                + camera.transform.forward() * scale.magnitude().max(1.) * 2.;
            let position = obj.transform.position();
            let target_rotation = camera.transform.rotation();
            let rotation = obj.transform.rotation();
            let unit_quat = UnitQuaternion::from_quaternion(rotation.lerp(&target_rotation, 0.03));
            obj.transform
                .set_position_vec(position.lerp(&target_position, 1.03));
            obj.transform.set_rotation(unit_quat);
            let rb = obj.get_component::<RigidBodyComponent>()?;
            rb.borrow_mut().set_kinematic(true);
        }

        if world.input.is_key_down(KeyCode::KeyC) || world.input.gamepad.is_button_down(Button::West) {
            let pos = camera.transform.position() + camera.transform.forward() * 3.;
            world.spawn(&CubePrefab { material: HMaterial::DEFAULT })
                .at_vec(pos)
                .build_component::<Collider3D>()
                .build_component::<RigidBodyComponent>();

            let sleeping_bodies = world.physics.rigid_body_set.iter().filter(|c| c.1.is_sleeping()).count();
            println!("{sleeping_bodies} Bodies are currently sleeping");
        }

        None
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

        city.transform.set_scale(0.01);

        // add colliders to city
        city.add_child_components_then(Collider3D::please_use_mesh);

        city
    }
}
