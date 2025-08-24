//! A showcase of various engine features.
//!
//! w: I use this as my main test environment, which allows me to expand this and experiment
//!    with new features. Therefore, it should contain the latest and greatest. I can recommend
//!    using this for reference.

use gilrs::Button;
use kira::effect::reverb::ReverbBuilder;
use kira::track::SpatialTrackBuilder;
use log::info;
use nalgebra::UnitQuaternion;
use rapier3d::parry::query::Ray;
use rapier3d::prelude::QueryFilter;
use slotmap::Key;
use std::error::Error;
use syrillian::assets::scene_loader::SceneLoader;
use syrillian::assets::{HMaterial, HSound, Sound, StoreType};
use syrillian::assets::{Material, Shader};
use syrillian::components::audio::AudioEmitter;
use syrillian::components::glyph::TextAlignment;
use syrillian::components::{CRef, Collider3D, Component, FirstPersonCameraController, PointLightComponent, RigidBodyComponent, RopeComponent, RotateComponent, SpotLightComponent, SpringComponent, Text2D, Text3D};
use syrillian::core::{GameObjectExt, GameObjectId};
use syrillian::prefabs::first_person_player::FirstPersonPlayerPrefab;
use syrillian::prefabs::prefab::Prefab;
use syrillian::prefabs::CubePrefab;
use syrillian::rendering::lights::Light;
use syrillian::utils::frame_counter::FrameCounter;
use syrillian::SyrillianApp;
use syrillian::{AppState, World};
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

#[cfg(debug_assertions)]
use syrillian::rendering::DebugRenderer;
// const NECO_IMAGE: &[u8; 1293] = include_bytes!("assets/neco.jpg");

const SHADER1: &str = include_str!("dynamic_shader/shader.wgsl");
const SHADER2: &str = include_str!("dynamic_shader/shader2.wgsl");
const SHADER3: &str = include_str!("dynamic_shader/shader3.wgsl");

#[derive(Debug, SyrillianApp)]
struct MyMain {
    frame_counter: FrameCounter,
    player: GameObjectId,
    player_rb: CRef<RigidBodyComponent>,
    picked_up: Option<GameObjectId>,
    text3d: GameObjectId,
    light1: CRef<SpotLightComponent>,
    light2: CRef<SpotLightComponent>,
    pop_sound: Option<HSound>,
    sound_cube_emitter: CRef<AudioEmitter>,
    sound_cube2_emitter: CRef<AudioEmitter>,
}

impl Default for MyMain {
    fn default() -> Self {
        Self {
            frame_counter: FrameCounter::default(),
            player: GameObjectId::null(),
            player_rb: CRef::null(),
            picked_up: None,
            text3d: GameObjectId::null(),
            light1: CRef::null(),
            light2: CRef::null(),
            pop_sound: None,
            sound_cube_emitter: CRef::null(),
            sound_cube2_emitter: CRef::null(),
        }
    }
}

impl AppState for MyMain {
    fn init(&mut self, world: &mut World) -> Result<(), Box<dyn Error>> {
        world.input.set_auto_cursor_lock(true);
        world.input.set_quit_on_escape(true);

        world.spawn(&City);

        self.player = world.spawn(&FirstPersonPlayerPrefab);
        self.player_rb = self.player.get_component::<RigidBodyComponent>().unwrap();

        // or freecam if you want
        // self.player = world.new_camera();
        // self.player.add_component::<FreecamController>();

        self.player.at(0.0, 20.0, 0.0);

        let shader = Shader::new_fragment("Funky Shader", SHADER1).store(world);
        let shader2 = Shader::new_fragment("Funky Shader 2", SHADER2).store(world);
        let shader3 = Shader::new_fragment("Funky Shader 3", SHADER3).store(world);

        let shader_mat_1 = Material::builder()
            .name("Cube Material 1".into())
            .shader(shader)
            .store(&world);
        let shader_mat_2 = Material::builder()
            .name("Cube Material 2".into())
            .shader(shader2)
            .store(&world);
        let shader_mat_3 = Material::builder()
            .name("Cube Material 3".into())
            .shader(shader3)
            .store(&world);

        let cube_prefab1 = CubePrefab::new(shader_mat_1);
        let cube_prefab2 = CubePrefab::new(shader_mat_2);
        let cube_prefab3 = CubePrefab::new(shader_mat_3);

        let mut big_cube_left = world.spawn(&cube_prefab1);
        let mut big_cube_right = world.spawn(&cube_prefab3);
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

        big_cube_left.at(100.0, 10.0, 200.0).scale(100.);

        big_cube_right.at(-100.0, 10.0, 200.0).scale(100.);

        let mut pop_sound = Sound::load_sound("./examples/assets/pop.wav")?;
        pop_sound.set_start_position(0.2);

        let pop_sound = pop_sound.store(world);
        self.pop_sound = Some(pop_sound);

        let sound_cube_prefab = CubePrefab::new(shader_mat_1);

        let mut sound_cube = world.spawn(&sound_cube_prefab);
        let mut sound_cube2 = world.spawn(&sound_cube_prefab);

        sound_cube
            .at(10.0, 150.0, 10.0)
            .build_component::<Collider3D>()
            .build_component::<RigidBodyComponent>()
            .enable_ccd();

        self.sound_cube_emitter = sound_cube.add_component::<AudioEmitter>();
        self.sound_cube_emitter.set_sound(pop_sound);

        sound_cube2
            .at(10.0, 150.0, 10.0)
            .build_component::<Collider3D>()
            .build_component::<RigidBodyComponent>()
            .enable_ccd();

        let mut reverb_track = SpatialTrackBuilder::new();
        reverb_track.add_effect(ReverbBuilder::new());
        self.sound_cube2_emitter = sound_cube2.add_component::<AudioEmitter>();
        self.sound_cube2_emitter.set_track(world, reverb_track).set_sound(pop_sound);

        {
            let mut text = world.new_object("Text 3D");
            let mut text3d = text.add_component::<Text3D>();

            text3d.set_size(1.0);
            text3d.set_alignment(TextAlignment::Center);
            text.transform.set_position(-10., 2., 0.);
            text.transform.set_euler_rotation(0., 90., 0.);
            text3d.set_rainbow_mode(true);

            world.add_child(text);
            self.text3d = text;
        }

        // fixme: render order matters because this is transparent and 2d
        {
            let mut text = world.new_object("Text");
            let mut text2d = text.add_component::<Text2D>();

            text2d.set_text("Meow");
            text2d.set_font("Impact");
            text2d.set_size(50.);
            text2d.set_position(0., 50.);
            text2d.set_rainbow_mode(true);

            world.add_child(text);
        }

        {
            let mut spring_bottom = world
                .spawn(&CubePrefab::new(HMaterial::DEFAULT))
                .at(-5., 10., -20.)
                .build_component::<Collider3D>()
                .mass(1.0)
                .build_component::<RigidBodyComponent>()
                .enable_ccd()
                .id; // FIXME: Workaround. Should have a .finish()
            let spring_top = world
                .spawn(&CubePrefab::new(HMaterial::DEFAULT))
                .at(-5., 20., -20.)
                .build_component::<Collider3D>()
                .mass(1.0)
                .build_component::<RigidBodyComponent>()
                .enable_ccd()
                .id; // FIXME: Workaround. Should have a .finish()

            let mut spring = spring_bottom.add_component::<SpringComponent>();
            spring.connect_to(spring_top);
            spring.set_rest_length(10.);
        }

        // world.spawn(&SunPrefab);
        let mut spot = world.new_object("Spot");
        spot.at(5., 5., -5.)
            .transform
            .set_euler_rotation(0., 80., 0.);

        self.light1 = spot.add_component::<SpotLightComponent>();
        self.light1.set_color(1.0, 0.2, 0.2);
        self.light1.set_intensity(1000.);
        self.light1.set_inner_angle(20.);
        self.light1.set_outer_angle(30.);

        let mut spot = world.new_object("Spot 2");
        spot.at(5., 5., -10.)
            .transform
            .set_euler_rotation(0., 100., 0.);

        self.light2 = spot.add_component::<SpotLightComponent>();
        self.light2.set_color(0.2, 0.2, 1.0);
        self.light2.set_intensity(1000.);
        self.light2.set_inner_angle(20.);
        self.light2.set_outer_angle(30.);

        world.print_objects();

        Ok(())
    }

    fn update(&mut self, world: &mut World) -> Result<(), Box<dyn Error>> {
        self.frame_counter.new_frame_from_world(world);
        world.set_window_title(self.format_title());

        let mut zoom_down = world.input.gamepad.button(Button::LeftTrigger2);
        if world.input.is_button_pressed(MouseButton::Right) {
            zoom_down = 1.0;
        }

        if let Some(mut camera) = world
            .active_camera()
            .upgrade(world)
            .and_then(|cam| cam.parent().get_component::<FirstPersonCameraController>())
        {
            camera.set_zoom(zoom_down);
        }

        let mut text3d = self.text3d.get_component::<Text3D>().unwrap();
        text3d.set_text(format!(
            "There are {} Objects in the World",
            world.objects.len(),
        ));

        if world.input.is_key_down(KeyCode::KeyF) {
            let is_kinematic = self.player_rb.is_kinematic();
            self.player_rb.set_kinematic(!is_kinematic);
        }

        self.do_raycast_test(world);

        if world.input.is_key_down(KeyCode::KeyU) {
            self.sound_cube_emitter.toggle_looping();
        }
        if world.input.is_key_down(KeyCode::KeyI) {
            self.sound_cube2_emitter.toggle_looping();
        }
        if world.input.is_key_down(KeyCode::KeyP) {
            if world.input.is_key_pressed(KeyCode::ShiftLeft) {
                self.sound_cube_emitter.stop();
            } else {
                self.sound_cube_emitter.play();
            }
        }
        if world.input.is_key_down(KeyCode::KeyO) {
            if world.input.is_key_pressed(KeyCode::ShiftLeft) {
                self.sound_cube2_emitter.stop();
            } else {
                self.sound_cube2_emitter.play();
            }
        }

        #[cfg(debug_assertions)]
        if world.input.is_key_down(KeyCode::KeyL) {
            let mode = DebugRenderer::next_mode();

            let Some(mut collider) = self.player.get_component::<Collider3D>() else {
                return Ok(());
            };
            if collider.is_local_debug_render_enabled() {
                collider.set_local_debug_render_enabled(false);
            } else {
                if mode == 0 {
                    collider.set_local_debug_render_enabled(true);
                }
            }
        }

        // sleep(Duration::from_millis(100));

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
        let camera = world.active_camera().upgrade(world)?;
        let camera_obj = camera.parent();

        let pick_up = world.input.gamepad.is_button_down(Button::RightTrigger)
            || world.input.is_button_down(MouseButton::Left);
        let drop = world.input.gamepad.is_button_released(Button::RightTrigger)
            || world.input.is_button_released(MouseButton::Left);

        if pick_up {
            let ray = Ray::new(
                camera_obj.transform.position().into(),
                camera_obj.transform.forward(),
            );
            let player_collider = self.player.get_component::<Collider3D>()?.phys_handle;
            let intersect = world.physics.cast_ray(
                &ray,
                5.,
                false,
                QueryFilter::only_dynamic().exclude_collider(player_collider),
            );

            #[cfg(debug_assertions)]
            {
                let mut camera = camera;
                camera.push_debug_ray(ray, 5.);
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
                let mut rb = obj.get_component::<RigidBodyComponent>()?;
                rb.set_kinematic(false);
            }
            self.picked_up = None;
        }

        if let Some(mut obj) = self.picked_up {
            let delta = world.delta_time().as_secs_f32();
            let scale = obj.transform.scale();
            let target_position = camera_obj.transform.position()
                + camera_obj.transform.forward() * scale.magnitude().max(1.) * 2.;
            let position = obj.transform.position();
            let target_rotation =
                UnitQuaternion::face_towards(&camera_obj.transform.up(), &camera_obj.transform.forward());
            let rotation = obj.transform.rotation();
            let unit_quat = UnitQuaternion::from_quaternion(rotation.lerp(&target_rotation, 1.03 * delta));
            obj.transform
                .set_position_vec(position.lerp(&target_position, 10.03 * delta));
            obj.transform.set_rotation(unit_quat);
            let mut rb = obj.get_component::<RigidBodyComponent>()?;
            rb.set_kinematic(true);
        }

        if world.input.is_key_down(KeyCode::KeyC)
            || world.input.gamepad.is_button_down(Button::West)
        {
            let pos = camera_obj.transform.position() + camera_obj.transform.forward() * 3.;
            world
                .spawn(&CubePrefab {
                    material: HMaterial::DEFAULT,
                })
                .at_vec(pos)
                .build_component::<Collider3D>()
                .build_component::<RigidBodyComponent>();

            let sleeping_bodies = world
                .physics
                .rigid_body_set
                .iter()
                .filter(|c| c.1.is_sleeping())
                .count();
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
