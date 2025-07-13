use log::{error, trace, warn};
use std::borrow::Cow;
use std::collections::HashMap;
use std::default::Default;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::rc::Rc;
use wgpu::*;

use crate::World;
use crate::asset_management::{MATERIAL_UBGL_ID, MODEL_UBGL_ID, POST_PROCESS_BGL_ID};
use crate::core::Vertex3D;

use super::bindgroup_layout_manager::{LIGHT_UBGL_ID, RENDER_UBGL_ID};

const POST_PROCESS_SHADER_PRE_CONTEXT: &str =
    include_str!("../rendering/shaders/engine_reserved_groups/post_process.wgsl");
const SHADER_PRE_CONTEXT: &str =
    include_str!("../rendering/shaders/engine_reserved_groups/basic.wgsl");

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ShaderStage {
    Default,
    PostProcess,
}

#[derive(Debug, Clone)]
pub struct Shader {
    pub name: String,
    pub code: String,
    pub polygon_mode: PolygonMode,
    pub draw_over: bool,
    pub stage: ShaderStage,
}

#[derive(Debug, Clone)]
pub struct RuntimeShader {
    pub name: String,
    pub module: ShaderModule,
    pub pipeline_layout: PipelineLayout,
    pub pipeline: RenderPipeline,
    pub draw_over: bool,
}

pub type ShaderId = usize;
// The fallback shader if a pipeline fails
pub const FALLBACK_SHADER_ID: ShaderId = 0;
// The default 3D shader.
pub const DIM3_SHADER_ID: ShaderId = 1;
// The default 2D shader.
pub const DIM2_SHADER_ID: ShaderId = 2;

// Default post processing shader
pub const POST_PROCESS_SHADER_ID: ShaderId = 3;
// An addon shader ID that is used for drawing debug edges on meshes
#[cfg(debug_assertions)]
pub const DEBUG_EDGES_SHADER_ID: ShaderId = 4;

#[derive(Debug)]
pub struct ShaderManager {
    next_id: ShaderId,
    raw_shaders: HashMap<ShaderId, Shader>,
    runtime_shaders: HashMap<ShaderId, RuntimeShader>,
    device: Option<Rc<Device>>,
}

impl Shader {
    pub fn make_shader_code(&self) -> Cow<'static, str> {
        Cow::Owned(format!("{}\n{}", &self.code, SHADER_PRE_CONTEXT))
    }

    pub fn make_post_process_shader_code(&self) -> Cow<'static, str> {
        Cow::Owned(format!(
            "{}\n{}",
            &self.code, POST_PROCESS_SHADER_PRE_CONTEXT
        ))
    }

    pub fn initialize_default_runtime(
        &self,
        device: &Device,
        render_uniform_bind_group_layout: &BindGroupLayout,
        model_uniform_bind_group_layout: &BindGroupLayout,
        material_uniform_bind_group_layout: &BindGroupLayout,
        light_uniform_bind_group_layout: &BindGroupLayout,
    ) -> RuntimeShader {
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some(&self.name),
            source: ShaderSource::Wgsl(self.make_shader_code()),
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(&format!("{} Pipeline Layout", self.name)),
            bind_group_layouts: &[
                render_uniform_bind_group_layout,
                model_uniform_bind_group_layout,
                material_uniform_bind_group_layout,
                light_uniform_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let pipeline =
            RenderPipelineBuilder::dim3(&self.name, &pipeline_layout, &shader, self.polygon_mode)
                .build(&device);

        RuntimeShader {
            name: self.name.clone(),
            module: shader,
            pipeline_layout,
            pipeline,
            draw_over: self.draw_over,
        }
    }

    pub fn initialize_post_process_runtime(
        &self,
        device: &Device,
        post_process_bind_group_layout: &BindGroupLayout,
    ) -> RuntimeShader {
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some(&self.name),
            source: ShaderSource::Wgsl(self.make_post_process_shader_code()),
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(&format!("{} PostProcess Pipeline Layout", self.name)),
            bind_group_layouts: &[post_process_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = RenderPipelineBuilder::post_process(&self.name, &pipeline_layout, &shader)
            .build(&device);

        RuntimeShader {
            name: self.name.clone(),
            module: shader,
            pipeline_layout,
            pipeline,
            draw_over: self.draw_over,
        }
    }
}

impl Default for ShaderManager {
    fn default() -> Self {
        let mut shader_manager = ShaderManager {
            next_id: 0,
            raw_shaders: HashMap::new(),
            runtime_shaders: HashMap::new(),
            device: None,
        };

        shader_manager.init();

        shader_manager
    }
}

#[allow(dead_code)]
impl ShaderManager {
    pub fn init(&mut self) {
        let shader = self.add_default_shader(
            "Fallback".to_string(),
            include_str!("../rendering/shaders/fallback_shader3d.wgsl").to_string(),
        );
        assert_eq!(shader, FALLBACK_SHADER_ID);

        let shader = self.add_default_shader(
            "3D Default Pipeline".to_string(),
            include_str!("../rendering/shaders/shader3d.wgsl").to_string(),
        );
        assert_eq!(shader, DIM3_SHADER_ID);

        let shader = self.add_default_shader(
            "2D Default Pipeline".to_string(),
            include_str!("../rendering/shaders/shader2d.wgsl").to_string(),
        );
        assert_eq!(shader, DIM2_SHADER_ID);

        let shader = self.add_post_process_shader(
            "PostProcess".to_string(),
            include_str!("../rendering/shaders/fullscreen_passhthrough.wgsl").to_string(),
        );
        assert_eq!(shader, POST_PROCESS_SHADER_ID);

        #[cfg(debug_assertions)]
        {
            let shader = self.add_shader(Shader {
                name: "3D Debug Edges Shader".to_owned(),
                code: include_str!("../rendering/shaders/debug/edges.wgsl").to_string(),
                polygon_mode: PolygonMode::Line,
                draw_over: true,
                stage: ShaderStage::Default,
            });
            assert_eq!(shader, DEBUG_EDGES_SHADER_ID);
        }
    }

    pub fn invalidate_runtime(&mut self) {
        warn!("Invalidating shader runtime.");
        self.runtime_shaders.clear();
        self.device = None;
    }

    pub fn init_runtime(&mut self, device: Rc<Device>) {
        self.device = Some(device);
        let shaders: Vec<ShaderId> = self.raw_shaders.keys().cloned().collect();
        for id in shaders {
            if self._init_single_runtime(id, None).is_none() {
                warn!("Failed to initialize shader with ID {id}");
            }
        }

        #[cfg(debug_assertions)]
        {
            let mut default = 0;
            let mut post_process = 0;

            for shader in self.runtime_shaders.values() {
                if shader.draw_over {
                    post_process += 1;
                } else {
                    default += 1;
                }
            }

            log::debug!("Initialized {default} Default and {post_process} Post Processing Shaders");
        }
    }

    pub fn add_default_shader_from_file<T>(
        &mut self,
        name: &str,
        path: T,
    ) -> Result<ShaderId, Box<dyn Error>>
    where
        T: AsRef<Path>,
    {
        let content = fs::read_to_string(path)?;
        Ok(self.add_default_shader(name.to_owned(), content.to_owned()))
    }

    pub fn add_post_process_shader(&mut self, name: String, code: String) -> ShaderId {
        self.add_shader(Shader {
            name,
            code,
            polygon_mode: PolygonMode::Fill,
            draw_over: true,
            stage: ShaderStage::PostProcess,
        })
    }

    pub fn add_default_shader(&mut self, name: String, code: String) -> ShaderId {
        self.add_shader(Shader {
            name,
            code,
            polygon_mode: PolygonMode::Fill,
            draw_over: false,
            stage: ShaderStage::Default,
        })
    }

    pub fn add_shader(&mut self, shader: Shader) -> ShaderId {
        let id = self.next_id;

        trace!("Added shader: {}", shader.name);

        self.raw_shaders.insert(self.next_id, shader);
        self.next_id += 1;

        id
    }

    // Get a shader by its ID or the fallback shader in case of no given Id
    // If the shader hasn't been registered yet in raw format, it will be replaced with a
    // fallback shader
    pub fn get_shader(&mut self, id: Option<ShaderId>) -> &RuntimeShader {
        self.get_shader_or_fallback(id.unwrap_or(FALLBACK_SHADER_ID))
    }

    // Get the shader by id, or register a fallback shader in its place
    pub fn get_shader_or_fallback(&mut self, id: ShaderId) -> &RuntimeShader {
        if self.runtime_shaders.contains_key(&id) {
            self.runtime_shaders.get(&id).expect("Shader should exist")
        } else {
            self._init_single_runtime_or_fallback(id)
        }
    }

    // Get the shader by id or fail
    pub fn get_shader_opt(&self, id: ShaderId) -> Option<&RuntimeShader> {
        self.runtime_shaders.get(&id).or_else(|| {
            warn!("Shader not found for id #{}", id);
            None
        })
    }

    // Get the shader by id or fail
    pub fn get_shader_or_init(&mut self, id: ShaderId) -> Option<&RuntimeShader> {
        if self.runtime_shaders.contains_key(&id) {
            self.runtime_shaders.get(&id)
        } else {
            let shader = self._init_single_runtime(id, None);

            if shader.is_none() {
                warn!("Shader for id #{} couldn't be initialized", id);
            }

            shader
        }
    }

    fn _init_single_runtime_or_fallback(&mut self, id: ShaderId) -> &RuntimeShader {
        if !self.raw_shaders.contains_key(&id) {
            error!(
                "Requested shader with id #{id} does not exist. Replaced with a fallback shader"
            );

            let fallback = self
                .get_shader_opt(FALLBACK_SHADER_ID)
                .expect("Fallback shader should exist")
                .clone();

            if let Some(bounced) = self.runtime_shaders.insert(id, fallback) {
                warn!(
                    "Bounced shader {:?} from its slot at id #{id}",
                    bounced.name
                );
            }

            self.runtime_shaders.get(&id).expect("Shader was inserted")
        } else {
            self._init_single_runtime(id, None)
                .expect("Raw Shader should exist")
        }
    }

    fn _init_single_runtime(
        &mut self,
        raw_id: ShaderId,
        as_run_id: Option<ShaderId>,
    ) -> Option<&RuntimeShader> {
        trace!("Initializing runtime shader with raw id: #{raw_id}");

        let raw_shader = self.raw_shaders.get(&raw_id)?;
        let world = World::instance();
        let bgls = &world.assets.bind_group_layouts;

        let runtime_shader = if raw_shader.stage == ShaderStage::PostProcess {
            let post_process_ubgl = bgls.get_bind_group_layout(POST_PROCESS_BGL_ID).unwrap();
            raw_shader.initialize_post_process_runtime(
                self.device.clone().unwrap().as_ref(),
                post_process_ubgl,
            )
        } else {
            let render_ubgl = bgls.get_bind_group_layout(RENDER_UBGL_ID).unwrap();
            let model_ubgl = bgls.get_bind_group_layout(MODEL_UBGL_ID).unwrap();
            let material_ubgl = bgls.get_bind_group_layout(MATERIAL_UBGL_ID).unwrap();
            let lighting_ubgl = bgls.get_bind_group_layout(LIGHT_UBGL_ID).unwrap();

            raw_shader.initialize_default_runtime(
                self.device.clone().unwrap().as_ref(),
                render_ubgl,
                model_ubgl,
                material_ubgl,
                lighting_ubgl,
            )
        };

        let id = as_run_id.unwrap_or(raw_id);
        if let Some(bounced) = self.runtime_shaders.insert(id, runtime_shader) {
            warn!("Bounced shader {:?} from its slot at id#{id}", bounced.name);
        }

        trace!(
            "Initialized runtime shader {:?} with id #{raw_id} as runtime shader with id #{id}",
            raw_shader.name
        );

        self.runtime_shaders.get(&id)
    }

    pub fn find_shader_by_name(&self, name: &str) -> Option<ShaderId> {
        self.raw_shaders
            .iter()
            .find(|(_, v)| v.name == name)
            .map(|(k, _)| k)
            .cloned()
    }
}

pub struct RenderPipelineBuilder<'a> {
    label: String,
    layout: &'a PipelineLayout,
    shader: &'a ShaderModule,
    is_post_process: bool,
    polygon_mode: PolygonMode,
}

impl<'a> RenderPipelineBuilder<'a> {
    pub fn build(&'a self, device: &Device) -> RenderPipeline {
        device.create_render_pipeline(&self.desc())
    }
    pub fn desc(&'a self) -> RenderPipelineDescriptor<'a> {
        const DEFAULT_BUFFERS: [VertexBufferLayout; 1] = [Vertex3D::continuous_descriptor()];

        const DEFAULT_COLOR_TARGET_STATE: [Option<ColorTargetState>; 1] =
            [Some(ColorTargetState {
                format: TextureFormat::Bgra8UnormSrgb,
                blend: None,
                write_mask: ColorWrites::all(),
            })];

        const DEFAULT_DEPTH_STENCIL: DepthStencilState = DepthStencilState {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: CompareFunction::Less,
            stencil: StencilState {
                front: StencilFaceState::IGNORE,
                back: StencilFaceState::IGNORE,
                read_mask: 0,
                write_mask: 0,
            },
            bias: DepthBiasState {
                constant: 0,
                slope_scale: 0.0,
                clamp: 0.0,
            },
        };

        let buffers = (!self.is_post_process)
            .then_some(DEFAULT_BUFFERS.as_ref())
            .unwrap_or(&[]);
        let depth_stencil = (!self.is_post_process).then_some(DEFAULT_DEPTH_STENCIL);
        let cull_mode = (!self.is_post_process).then_some(Face::Back);

        RenderPipelineDescriptor {
            label: Some(&self.label),
            layout: Some(self.layout),

            vertex: VertexState {
                module: self.shader,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers,
            },
            primitive: PrimitiveState {
                cull_mode,
                polygon_mode: self.polygon_mode,
                ..PrimitiveState::default()
            },
            depth_stencil,
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                module: self.shader,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &DEFAULT_COLOR_TARGET_STATE,
            }),
            multiview: None,
            cache: None,
        }
    }

    pub fn post_process(
        name: &str,
        layout: &'a PipelineLayout,
        shader: &'a ShaderModule,
    ) -> RenderPipelineBuilder<'a> {
        RenderPipelineBuilder {
            label: format!("{name} PostProcess Pipeline"),
            layout,
            shader,
            is_post_process: true,
            polygon_mode: PolygonMode::Fill,
        }
    }

    pub fn dim3(
        name: &str,
        layout: &'a PipelineLayout,
        shader: &'a ShaderModule,
        polygon_mode: PolygonMode,
    ) -> RenderPipelineBuilder<'a> {
        RenderPipelineBuilder {
            label: format!("{name} Pipeline"),
            layout,
            shader,
            is_post_process: false,
            polygon_mode,
        }
    }
}
