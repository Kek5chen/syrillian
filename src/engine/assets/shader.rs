use crate::engine::assets::generic_store::{HandleName, Store, StoreDefaults, StoreType};
use crate::engine::assets::{H, HShader, StoreTypeFallback, StoreTypeName};
use crate::store_add_checked;
use crate::utils::sizes::VEC3_SIZE;
use std::borrow::Cow;
use std::error::Error;
use std::fs;
use std::path::Path;
use wgpu::{
    PolygonMode, PrimitiveTopology, VertexAttribute, VertexBufferLayout,
    VertexFormat, VertexStepMode,
};

#[derive(Debug, Clone)]
pub enum Shader {
    Default {
        name: String,
        code: String,
        polygon_mode: PolygonMode,
    },

    PostProcess {
        name: String,
        code: String,
    },

    Custom {
        name: String,
        code: String,
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

    // An addon shader ID that is used for drawing debug edges on meshes
    #[cfg(debug_assertions)]
    pub const DEBUG_VERTEX_NORMALS: H<Shader> = H::new(Self::DEBUG_VERTEX_NORMALS_ID);
}

impl StoreDefaults for Shader {
    fn populate(store: &mut Store<Self>) {
        let shader = store.add_default_shader(
            "Fallback".to_string(),
            include_str!("../rendering/shaders/fallback_shader3d.wgsl").to_string(),
        );
        assert_eq!(shader, HShader::FALLBACK);

        let shader = store.add_default_shader(
            "3D Default Pipeline".to_string(),
            include_str!("../rendering/shaders/shader3d.wgsl").to_string(),
        );

        assert_eq!(shader, HShader::DIM3);

        let shader = store.add_default_shader(
            "2D Default Pipeline".to_string(),
            include_str!("../rendering/shaders/shader2d.wgsl").to_string(),
        );
        assert_eq!(shader, HShader::DIM2);

        let shader = store.add_post_process_shader(
            "PostProcess".to_string(),
            include_str!("../rendering/shaders/fullscreen_passhthrough.wgsl").to_string(),
        );
        assert_eq!(shader, HShader::POST_PROCESS);

        #[cfg(debug_assertions)]
        {
            store_add_checked!(
                store,
                HShader::DEBUG_EDGES_ID,
                Shader::Default {
                    name: "Mesh Debug Edges Shader".to_owned(),
                    code: include_str!("../rendering/shaders/debug/edges.wgsl").to_string(),
                    polygon_mode: PolygonMode::Line,
                }
            );

            const DEBUG_VERTEX_NORMALS_VBL: &[VertexBufferLayout] = &[
                VertexBufferLayout {
                    array_stride: 0,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &[
                        VertexAttribute {
                            format: VertexFormat::Uint32,
                            offset: 0,
                            shader_location: 0,
                        }
                    ],
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
                    code: include_str!("../rendering/shaders/debug/vertex_normals.wgsl")
                        .to_string(),
                    topology: PrimitiveTopology::LineList,
                    polygon_mode: PolygonMode::Line,
                    vertex_buffers: DEBUG_VERTEX_NORMALS_VBL,
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

const POST_PROCESS_SHADER_PRE_CONTEXT: &str =
    include_str!("../rendering/shaders/engine_reserved_groups/post_process.wgsl");
const SHADER_PRE_CONTEXT: &str =
    include_str!("../rendering/shaders/engine_reserved_groups/basic.wgsl");

impl Shader {
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
            | Shader::Custom { code, .. } => *code = source,
        }
    }

    pub fn gen_code(&self) -> Cow<'static, str> {
        let code = match self {
            Shader::Default { code, .. } | Shader::Custom { code, .. } => {
                format!("{}\n{}", code, SHADER_PRE_CONTEXT)
            }
            Shader::PostProcess { code, .. } => {
                format!("{}\n{}", code, POST_PROCESS_SHADER_PRE_CONTEXT)
            }
        };

        Cow::Owned(code)
    }
}

impl Store<Shader> {
    pub fn add_default_shader_from_file<T>(
        &self,
        name: &str,
        path: T,
    ) -> Result<H<Shader>, Box<dyn Error>>
    where
        T: AsRef<Path>,
    {
        let content = fs::read_to_string(path)?;
        Ok(self.add_default_shader(name.to_owned(), content.to_owned()))
    }

    pub fn add_post_process_shader(&self, name: String, code: String) -> H<Shader> {
        self.add(Shader::PostProcess { name, code })
    }

    pub fn add_default_shader(&self, name: String, code: String) -> H<Shader> {
        self.add(Shader::Default {
            name,
            code,
            polygon_mode: PolygonMode::Fill,
        })
    }
}
