mod shader_gen;

// this module only has tests for the built-in shaders and can be safely ignored
mod shaders;

use crate::assets::shader::shader_gen::ShaderGen;
use crate::assets::HBGL;
use crate::engine::assets::generic_store::{HandleName, Store, StoreDefaults, StoreType};
use crate::engine::assets::{HShader, StoreTypeFallback, StoreTypeName, H};
use crate::{store_add_checked, store_add_checked_many};
use std::error::Error;
use std::fs;
use std::path::Path;
use wgpu::{PolygonMode, PrimitiveTopology, VertexBufferLayout};

#[derive(Debug, Clone)]
pub enum ShaderCode {
    Full(String),
    Fragment(String),
}

impl ShaderCode {
    pub fn is_only_fragment_shader(&self) -> bool {
        matches!(self, ShaderCode::Fragment(_))
    }

    pub fn code(&self) -> &str {
        match self {
            ShaderCode::Full(code) => code,
            ShaderCode::Fragment(code) => code,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum PipelineStage {
    Default,
    PostProcess,
}

#[derive(Debug, Clone)]
pub enum Shader {
    Default {
        name: String,
        code: ShaderCode,
        polygon_mode: PolygonMode,
    },

    PostProcess {
        name: String,
        code: ShaderCode,
    },

    Custom {
        name: String,
        code: ShaderCode,
        polygon_mode: PolygonMode,
        topology: PrimitiveTopology,
        vertex_buffers: &'static [VertexBufferLayout<'static>],
    },
}

impl H<Shader> {
    pub const FALLBACK_ID: u32 = 0;
    pub const DIM3_ID: u32 = 1;
    pub const DIM2_ID: u32 = 2;
    pub const POST_PROCESS_ID: u32 = 3;
    #[cfg(debug_assertions)]
    pub const DEBUG_EDGES_ID: u32 = 4;
    #[cfg(debug_assertions)]
    pub const DEBUG_VERTEX_NORMALS_ID: u32 = 5;
    #[cfg(debug_assertions)]
    pub const DEBUG_RAYS_ID: u32 = 6;

    // The fallback shader if a pipeline fails
    pub const FALLBACK: H<Shader> = H::new(Self::FALLBACK_ID);

    // The default 3D shader.
    pub const DIM3: H<Shader> = H::new(Self::DIM3_ID);

    // The default 2D shader.
    pub const DIM2: H<Shader> = H::new(Self::DIM2_ID);

    // Default post-processing shader
    pub const POST_PROCESS: H<Shader> = H::new(Self::POST_PROCESS_ID);

    // An addon shader ID that is used for drawing debug edges on meshes
    #[cfg(debug_assertions)]
    pub const DEBUG_EDGES: H<Shader> = H::new(Self::DEBUG_EDGES_ID);

    // An addon shader ID that is used for drawing debug vertex normals on meshes
    #[cfg(debug_assertions)]
    pub const DEBUG_VERTEX_NORMALS: H<Shader> = H::new(Self::DEBUG_VERTEX_NORMALS_ID);

    // An addon shader ID that is used for drawing debug rays
    #[cfg(debug_assertions)]
    pub const DEBUG_RAYS: H<Shader> = H::new(Self::DEBUG_RAYS_ID);
}

const SHADER_FALLBACK3D: &str = include_str!("shaders/fallback_shader3d.wgsl");
const SHADER_DIM3: &str = include_str!("shaders/shader3d.wgsl");
const SHADER_DIM2: &str = include_str!("shaders/shader2d.wgsl");
const SHADER_FS_COPY: &str = include_str!("shaders/fullscreen_passhthrough.wgsl");

#[cfg(debug_assertions)]
const DEBUG_EDGES_SHADER: &str = include_str!("shaders/debug/edges.wgsl");
#[cfg(debug_assertions)]
const DEBUG_VERTEX_NORMAL_SHADER: &str = include_str!("shaders/debug/vertex_normals.wgsl");
#[cfg(debug_assertions)]
const DEBUG_RAY_SHADER: &str = include_str!("shaders/debug/rays.wgsl");

impl StoreDefaults for Shader {
    fn populate(store: &mut Store<Self>) {
        store_add_checked_many!(store,
            HShader::FALLBACK_ID => Shader::new_default("Fallback", SHADER_FALLBACK3D),
            HShader::DIM3_ID => Shader::new_fragment("3D Default Pipeline", SHADER_DIM3),
            HShader::DIM2_ID => Shader::new_default("2D Default Pipeline", SHADER_DIM2),
            HShader::POST_PROCESS_ID => Shader::new_post_process("Post Process", SHADER_FS_COPY),
        );

        #[cfg(debug_assertions)]
        {
            use crate::utils::sizes::VEC3_SIZE;
            use wgpu::{VertexAttribute, VertexFormat, VertexStepMode};

            store_add_checked!(
                store,
                HShader::DEBUG_EDGES_ID,
                Shader::Default {
                    name: "Mesh Debug Edges Shader".to_owned(),
                    code: ShaderCode::Full(DEBUG_EDGES_SHADER.to_string()),
                    polygon_mode: PolygonMode::Line,
                }
            );

            const DEBUG_VERTEX_NORMALS_VBL: &[VertexBufferLayout] = &[
                VertexBufferLayout {
                    array_stride: 0,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &[VertexAttribute {
                        format: VertexFormat::Uint32,
                        offset: 0,
                        shader_location: 0,
                    }],
                },
                VertexBufferLayout {
                    array_stride: 0,
                    step_mode: VertexStepMode::Instance,
                    attributes: &[
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 1,
                        },
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: VEC3_SIZE,
                            shader_location: 2,
                        },
                    ],
                },
            ];

            store_add_checked!(
                store,
                HShader::DEBUG_VERTEX_NORMALS_ID,
                Shader::Custom {
                    name: "Mesh Debug Vertices Shader".to_owned(),
                    code: ShaderCode::Full(DEBUG_VERTEX_NORMAL_SHADER.to_string()),
                    topology: PrimitiveTopology::LineList,
                    polygon_mode: PolygonMode::Line,
                    vertex_buffers: DEBUG_VERTEX_NORMALS_VBL,
                }
            );

            const DEBUG_RAYS_VBL: &[VertexBufferLayout] = &[
                VertexBufferLayout {
                    array_stride: 0,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &[VertexAttribute {
                        format: VertexFormat::Uint32,
                        offset: 0,
                        shader_location: 0,
                    }],
                },
                VertexBufferLayout {
                    array_stride: 0,
                    step_mode: VertexStepMode::Instance,
                    attributes: &[
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 1,
                        },
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: VEC3_SIZE,
                            shader_location: 2,
                        },
                        VertexAttribute {
                            format: VertexFormat::Float32,
                            offset: VEC3_SIZE * 2,
                            shader_location: 3,
                        },
                    ],
                },
            ];

            store_add_checked!(
                store,
                HShader::DEBUG_RAYS_ID,
                Shader::Custom {
                    name: "Ray Debug".to_owned(),
                    code: ShaderCode::Full(DEBUG_RAY_SHADER.to_string()),
                    topology: PrimitiveTopology::LineList,
                    polygon_mode: PolygonMode::Line,
                    vertex_buffers: DEBUG_RAYS_VBL,
                }
            );
        }
    }
}

impl StoreTypeFallback for Shader {
    #[inline]
    fn fallback() -> H<Self> {
        HShader::FALLBACK
    }
}

impl StoreTypeName for Shader {
    #[inline]
    fn name(&self) -> &str {
        &self.name()
    }
}

impl StoreType for Shader {
    #[inline]
    fn name() -> &'static str {
        "Shader"
    }

    fn ident_fmt(handle: H<Self>) -> HandleName<Self> {
        match handle.id() {
            HShader::FALLBACK_ID => HandleName::Static("Diffuse Fallback"),
            HShader::DIM3_ID => HandleName::Static("3 Dimensional Shader"),
            HShader::DIM2_ID => HandleName::Static("2 Dimensional Shader"),
            HShader::POST_PROCESS_ID => HandleName::Static("Post Process Shader"),

            #[cfg(debug_assertions)]
            HShader::DEBUG_EDGES_ID => HandleName::Static("Debug Edges Shader"),

            _ => HandleName::Id(handle),
        }
    }
}

impl Shader {
    pub fn load_default<S, T>(name: S, path: T) -> Result<Shader, Box<dyn Error>>
    where
        S: Into<String>,
        T: AsRef<Path>,
    {
        let content = fs::read_to_string(path)?;
        Ok(Self::new_default(name, content))
    }

    pub fn load_fragment<S, T>(name: S, path: T) -> Result<Shader, Box<dyn Error>>
    where
        S: Into<String>,
        T: AsRef<Path>,
    {
        let code = fs::read_to_string(path)?;
        Ok(Self::new_fragment(name, code))
    }

    pub fn new_post_process<S, S2>(name: S, code: S2) -> Shader
    where
        S: Into<String>,
        S2: Into<String>,
    {
        Shader::PostProcess {
            name: name.into(),
            code: ShaderCode::Full(code.into()),
        }
    }

    pub fn new_fragment<S, S2>(name: S, code: S2) -> Shader
    where
        S: Into<String>,
        S2: Into<String>,
    {
        Shader::Default {
            name: name.into(),
            code: ShaderCode::Fragment(code.into()),
            polygon_mode: PolygonMode::Fill,
        }
    }

    pub fn new_default<S, S2>(name: S, code: S2) -> Shader
    where
        S: Into<String>,
        S2: Into<String>,
    {
        Shader::Default {
            name: name.into(),
            code: ShaderCode::Full(code.into()),
            polygon_mode: PolygonMode::Fill,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Shader::Default { name, .. } => name,
            Shader::PostProcess { name, .. } => name,
            Shader::Custom { name, .. } => name,
        }
    }

    pub fn polygon_mode(&self) -> PolygonMode {
        match self {
            Shader::Default { polygon_mode, .. } => *polygon_mode,
            Shader::PostProcess { .. } => PolygonMode::Fill,
            Shader::Custom { .. } => PolygonMode::Fill,
        }
    }

    pub fn topology(&self) -> PrimitiveTopology {
        match self {
            Shader::Default { .. } | Shader::PostProcess { .. } => PrimitiveTopology::TriangleList,
            Shader::Custom { topology, .. } => *topology,
        }
    }

    pub fn set_code(&mut self, source: String) {
        match self {
            Shader::Default { code, .. }
            | Shader::PostProcess { code, .. }
            | Shader::Custom { code, .. } => *code = ShaderCode::Full(source),
        }
    }

    pub fn code(&self) -> &ShaderCode {
        match self {
            Shader::Default { code, .. }
            | Shader::PostProcess { code, .. }
            | Shader::Custom { code, .. } => code,
        }
    }

    pub fn set_fragment_code(&mut self, source: String) {
        match self {
            Shader::Default { code, .. }
            | Shader::PostProcess { code, .. }
            | Shader::Custom { code, .. } => *code = ShaderCode::Fragment(source),
        }
    }

    pub fn stage(&self) -> PipelineStage {
        match self {
            Shader::Default { .. } => PipelineStage::Default,
            Shader::PostProcess { .. } => PipelineStage::PostProcess,
            Shader::Custom { .. } => PipelineStage::Default,
        }
    }

    pub fn is_custom(&self) -> bool {
        matches!(self, Shader::Custom { .. })
    }

    pub fn gen_code(&self) -> String {
        ShaderGen::new(self).generate()
    }

    pub fn needs_bgl(&self, bgl: HBGL) -> bool {
        if !self.is_custom() {
            return true;
        }

        let use_name = match bgl.id() {
            HBGL::MODEL_ID => "model",
            HBGL::LIGHT_ID => "light",

            HBGL::RENDER_ID => return true,
            _ => return false,
        };
        let source = self.code().code();

        for line in source.lines() {
            let Some(i) = line.find("#use ") else {
                continue;
            };

            if line[i + 5..].trim() == use_name {
                return true;
            }
        }

        false
    }
}
