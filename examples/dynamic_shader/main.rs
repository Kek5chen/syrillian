//! Example to showcase dynamic shader switching / cache refresh.

use log::{debug, error, info};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::error::Error;
use std::fs;
use std::sync::mpsc;
use std::sync::mpsc::TryRecvError;
use std::time::Instant;
use syrillian::assets::{HMaterial, HShader, Material, Shader};
use syrillian::components::RotateComponent;
use syrillian::core::GameObjectId;
use syrillian::prefabs::CubePrefab;
use syrillian::{AppState, World};
use syrillian_macros::SyrillianApp;
use wgpu::naga::valid::{Capabilities, ValidationFlags};
use wgpu::PolygonMode;
use winit::window::Window;

const SHADER_PATH: &str = "examples/dynamic_shader/shader.wgsl";
const DEFAULT_VERT: &str = include_str!("../../src/engine/rendering/shaders/default_vertex3d.wgsl");

#[derive(SyrillianApp)]
struct DynamicShaderExample {
    last_successful_shader: Option<String>,
    last_refresh_time: Instant,
    shader_id: HShader,
    material_id: HMaterial,
    _watcher: RecommendedWatcher,
    file_events: mpsc::Receiver<notify::Result<Event>>,
    cube: GameObjectId,
}

impl Default for DynamicShaderExample {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(tx).expect("failed to create watcher");
        watcher
            .watch(SHADER_PATH.as_ref(), RecursiveMode::NonRecursive)
            .expect("failed to start watcher");
        watcher
            .configure(Config::default().with_compare_contents(true))
            .expect("failed to configure notify watcher");

        DynamicShaderExample {
            last_successful_shader: None,
            last_refresh_time: Instant::now(),
            shader_id: HShader::FALLBACK,
            material_id: HMaterial::FALLBACK,
            _watcher: watcher,
            file_events: rx,
            cube: GameObjectId::invalid(),
        }
    }
}

impl DynamicShaderExample {
    fn check_valid(source: &str) -> Result<(), String> {
        let code = Shader::Default {
            name: "Dynamic Shader".to_string(),
            code: source.to_string(),
            polygon_mode: PolygonMode::Fill,
        }
        .gen_code();

        let module =
            wgpu::naga::front::wgsl::parse_str(&code).map_err(|e| e.emit_to_string(&code))?;

        let mut validator =
            wgpu::naga::valid::Validator::new(ValidationFlags::all(), Capabilities::all());
        validator
            .validate(&module)
            .map_err(|e| e.emit_to_string(&code))?;

        Ok(())
    }

    fn activate_shader(&mut self, world: &mut World, source: String) {
        let source_2 = source.clone(); // not the real one lol
        self.last_successful_shader = Some(source);

        if self.shader_id == HShader::FALLBACK {
            let shader = world
                .assets
                .shaders
                .add_default_shader("Dynamic Shader".to_string(), source_2);
            let material = world.assets.materials.add(
                Material::builder()
                    .name("Dynamic Shader Material".to_string())
                    .shader(shader)
                    .build(),
            );

            self.shader_id = shader;
            self.material_id = material;
        } else {
            world
                .assets
                .shaders
                .get_mut(self.shader_id)
                .set_code(source_2);
        }
    }

    fn try_laod_shader(&mut self, world: &mut World) -> Result<(), Box<dyn Error>> {
        let mut source = fs::read_to_string(SHADER_PATH)?;
        source.insert_str(0, DEFAULT_VERT);

        if let Err(msg) = Self::check_valid(&source) {
            error!("{}", msg);
            Err(msg)?
        }

        self.activate_shader(world, source);

        Ok(())
    }

    fn refresh_shader(&mut self, world: &mut World) -> Result<(), Box<dyn Error>> {
        self.try_laod_shader(world)?;
        self.respawn_cube(world);
        info!("Shader refreshed");

        Ok(())
    }

    fn poll(&mut self, world: &mut World) -> Result<(), Box<dyn Error>> {
        debug!("Polling for changes..");

        match self.file_events.try_recv() {
            Ok(event) => event?,
            Err(TryRecvError::Disconnected) => panic!("file events channel closed"),
            Err(TryRecvError::Empty) => {
                debug!("No changes");
                return Ok(());
            }
        };

        self.refresh_shader(world)?;

        Ok(())
    }

    fn respawn_cube(&mut self, world: &mut World) {
        let mut iter = 0.;
        let mut y_rot = 0.;
        if self.cube.exists() {
            let old_comp = self.cube.get_component::<RotateComponent>().unwrap();
            let old_comp = old_comp.borrow();
            iter = old_comp.iteration;
            y_rot = old_comp.y_rot;
            drop(old_comp);

            self.cube.delete();
        }

        self.cube = world.spawn(&CubePrefab {
            material: self.material_id,
        });

        self.cube.transform.set_uniform_scale(2.0);
        self.cube.transform.set_position(0., 0., -5.0);
        let new_comp = self.cube.add_component::<RotateComponent>();
        new_comp.iteration = 90.;
        new_comp.y_rot = 45.;
        new_comp.rotate_speed = 0.0;
    }
}

impl AppState for DynamicShaderExample {
    fn init(&mut self, world: &mut World, _window: &Window) -> Result<(), Box<dyn Error>> {
        _ = self.try_laod_shader(world);
        self.respawn_cube(world);

        world.new_camera();

        Ok(())
    }
    fn update(&mut self, world: &mut World, _window: &Window) -> Result<(), Box<dyn Error>> {
        if self.last_refresh_time.elapsed().as_secs() > 0 {
            self.poll(world)?;
            self.last_refresh_time = Instant::now();
        }

        Ok(())
    }
}
