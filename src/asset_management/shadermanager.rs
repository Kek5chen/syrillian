use std::borrow::Cow;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::rc::Rc;
use wgpu::*;

use crate::asset_management::bindgroup_layout_manager::{CAMERA_UBGL_ID, MATERIAL_UBGL_ID, MODEL_UBGL_ID, POST_PROCESS_BGL_ID};
use crate::asset_management::mesh::Vertex3D;
use crate::world::World;

#[derive(Debug)]
pub struct Shader {
    pub name: String,
    pub code: String,
    pub polygon_mode: PolygonMode,
    pub draw_over: bool,
}

#[derive(Debug)]
pub struct RuntimeShader {
    pub name: String,
    pub module: ShaderModule,
    pub pipeline_layout: PipelineLayout,
    pub pipeline: RenderPipeline,
    pub draw_over: bool
}

pub type ShaderId = usize;
// The fallback shader if a pipeline fails
pub const FALLBACK_SHADER_ID: ShaderId = 0;
// The default 3D shader.
pub const DIM3_SHADER_ID: ShaderId = 1;

pub const POST_PROCESS_SHADER_ID: ShaderId = 2;
pub const DEBUG_EDGES_SHADER_ID: ShaderId = 3;

#[derive(Debug)]
pub struct ShaderManager {
    next_id: ShaderId,
    raw_shaders: HashMap<ShaderId, Shader>,
    runtime_shaders: HashMap<ShaderId, RuntimeShader>,
    device: Option<Rc<Device>>,
}

impl Shader {
    pub fn initialize_combined_runtime(
        &self,
        device: &Device,
        camera_uniform_bind_group_layout: &BindGroupLayout,
        model_uniform_bind_group_layout: &BindGroupLayout,
        material_uniform_bind_group_layout: &BindGroupLayout,
    ) -> RuntimeShader {
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some(&self.name),
            source: ShaderSource::Wgsl(Cow::Borrowed(&self.code)),
        });
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(&format!("{} Pipeline Layout", self.name)),
            bind_group_layouts: &[
                camera_uniform_bind_group_layout,
                model_uniform_bind_group_layout,
                material_uniform_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(&format!("{} Pipeline", self.name)),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[Vertex3D::continuous_descriptor()],
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                unclipped_depth: false,
                polygon_mode: self.polygon_mode,
                conservative: false,
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format: TextureFormat::Bgra8UnormSrgb,
                    blend: None,
                    write_mask: ColorWrites::all(),
                })],
            }),
            multiview: None,
            cache: None,
        });
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
            source: ShaderSource::Wgsl(Cow::Borrowed(&self.code)),
        });
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(&format!("{} PostProcess Pipeline Layout", self.name)),
            bind_group_layouts: &[post_process_bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(&format!("{} PostProcess Pipeline", self.name)),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                ..PrimitiveState::default()
            },
            depth_stencil: None, // No depth for post-processing
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format: TextureFormat::Bgra8UnormSrgb,
                    blend: None,
                    write_mask: ColorWrites::all(),
                })],
            }),
            multiview: None,
            cache: None,
        });

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
        self.add_shader(
            "Fallback".to_string(),
            include_str!("../shaders/fallback_shader3d.wgsl").to_string(),
        );
        self.add_shader(
            "3D Default Pipeline".to_string(),
            include_str!("../shaders/shader3d.wgsl").to_string(),
        );
        self.add_shader(
            "PostProcess".to_string(),
            include_str!("../shaders/fullscreen_passhthrough.wgsl").to_string(),
        );
        self.add_shader(
            "3D Debug Edges Shader".to_string(),
            include_str!("../shaders/debug/edges.wgsl").to_string(),
        );

        let shader = self.raw_shaders.get_mut(&DEBUG_EDGES_SHADER_ID).unwrap();
        shader.draw_over = true;
        shader.polygon_mode = PolygonMode::Line;
    }

    pub fn invalidate_runtime(&mut self) {
        self.runtime_shaders.clear();
        self.device = None;
    }

    pub fn init_runtime(&mut self, device: Rc<Device>) {
        self.device = Some(device);
        self.init();
    }

    pub fn add_combined_shader_file<T>(
        &mut self,
        name: &str,
        path: T,
    ) -> Result<ShaderId, Box<dyn Error>>
    where
        T: AsRef<Path>,
    {
        let content = fs::read_to_string(path)?;
        Ok(self.add_combined_shader(name, &content))
    }

    pub fn add_combined_shader(&mut self, name: &str, shader: &str) -> ShaderId {
        self.add_shader(name.to_string(), shader.to_string())
    }

    pub fn add_shader(&mut self, name: String, code: String) -> ShaderId {
        let id = self.next_id;

        self.raw_shaders.insert(
            self.next_id,
            Shader { name, code, polygon_mode: PolygonMode::Fill, draw_over: false },
        );
        self.next_id += 1;

        id
    }

    pub(crate) fn get_shader(&mut self, id: ShaderId) -> Option<&RuntimeShader> {
        // ugly but the borrow checker sucks a bit here
        if self.runtime_shaders.contains_key(&id) {
            self.runtime_shaders.get(&id)
        } else {
            self.init_single_runtime(id)
        }
    }

    fn init_single_runtime(&mut self, id: ShaderId) -> Option<&RuntimeShader> {
        let raw_shader = self.raw_shaders.get(&id)?;
        let world = World::instance();
        let bgls = &world.assets.bind_group_layouts;

        let runtime_shader = match id {
            FALLBACK_SHADER_ID | DIM3_SHADER_ID | DEBUG_EDGES_SHADER_ID => {
                let camera_ubgl = bgls.get_bind_group_layout(CAMERA_UBGL_ID).unwrap();
                let model_ubgl = bgls.get_bind_group_layout(MODEL_UBGL_ID).unwrap();
                let material_ubgl = bgls.get_bind_group_layout(MATERIAL_UBGL_ID).unwrap();
                raw_shader.initialize_combined_runtime(
                    self.device.clone().unwrap().as_ref(),
                    camera_ubgl,
                    model_ubgl,
                    material_ubgl,
                )
            },
            POST_PROCESS_SHADER_ID => {
                let post_process_ubgl = bgls.get_bind_group_layout(POST_PROCESS_BGL_ID).unwrap();
                raw_shader.initialize_post_process_runtime(
                    self.device.clone().unwrap().as_ref(),
                    post_process_ubgl,
                )
            },
            _ => panic!("Shader ID not recognized"),
        };

        self.runtime_shaders.insert(id, runtime_shader);

        self.runtime_shaders.get(&id)
    }

    pub(crate) fn find_shader_by_name(&self, name: &str) -> Option<ShaderId> {
        self.raw_shaders
            .iter()
            .find(|(_, v)| v.name == name)
            .map(|(k, _)| k)
            .cloned()
    }
}
