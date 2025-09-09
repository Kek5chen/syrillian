mod shader_gen;

// this module only has tests for the built-in shaders and can be safely ignored
mod shaders;

use crate::assets::HBGL;
use crate::assets::shader::shader_gen::ShaderGen;
use crate::engine::assets::generic_store::{HandleName, Store, StoreDefaults, StoreType};
use crate::engine::assets::{H, HShader, StoreTypeFallback, StoreTypeName};
use crate::rendering::AssetCache;
use crate::rendering::proxies::text_proxy::TextPushConstants;
use crate::utils::sizes::VEC2_SIZE;
use crate::{store_add_checked, store_add_checked_many};
use std::error::Error;
use std::fs;
use std::path::Path;
use wgpu::{
    BindGroupLayout, Device, PipelineLayout, PipelineLayoutDescriptor, PolygonMode,
    PrimitiveTopology, PushConstantRange, ShaderStages, VertexAttribute, VertexBufferLayout,
    VertexFormat, VertexStepMode,
};

#[cfg(debug_assertions)]
use crate::rendering::DEFAULT_VBL_STEP_INSTANCE;

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

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
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
        push_constant_ranges: &'static [PushConstantRange],
        shadow_transparency: bool,
    },
}

impl H<Shader> {
    pub const FALLBACK_ID: u32 = 0;
    pub const DIM2_ID: u32 = 1;
    pub const DIM3_ID: u32 = 2;
    pub const POST_PROCESS_ID: u32 = 3;
    pub const TEXT_2D_ID: u32 = 4;
    pub const TEXT_3D_ID: u32 = 5;
    pub const SKYBOX_CUBEMAP_ID: u32 = 6;
    #[cfg(not(debug_assertions))]
    pub const MAX_BUILTIN_ID: u32 = 6;

    #[cfg(debug_assertions)]
    pub const DEBUG_EDGES_ID: u32 = 7;
    #[cfg(debug_assertions)]
    pub const DEBUG_VERTEX_NORMALS_ID: u32 = 8;
    #[cfg(debug_assertions)]
    pub const DEBUG_LINES_ID: u32 = 9;
    #[cfg(debug_assertions)]
    pub const DEBUG_TEXT2D_GEOMETRY_ID: u32 = 10;
    #[cfg(debug_assertions)]
    pub const DEBUG_TEXT3D_GEOMETRY_ID: u32 = 11;
    #[cfg(debug_assertions)]
    pub const DEBUG_LIGHT_ID: u32 = 12;
    #[cfg(debug_assertions)]
    pub const MAX_BUILTIN_ID: u32 = 12;

    // The fallback shader if a pipeline fails
    pub const FALLBACK: H<Shader> = H::new(Self::FALLBACK_ID);

    // The default 2D shader.
    pub const DIM2: H<Shader> = H::new(Self::DIM2_ID);

    // The default 3D shader.
    pub const DIM3: H<Shader> = H::new(Self::DIM3_ID);

    // Default post-processing shader
    pub const POST_PROCESS: H<Shader> = H::new(Self::POST_PROCESS_ID);

    // Default 2D Text shader.
    pub const TEXT_2D: H<Shader> = H::new(Self::TEXT_2D_ID);

    // Default 3D Text shader.
    pub const TEXT_3D: H<Shader> = H::new(Self::TEXT_3D_ID);

    // An addon shader ID that is used for drawing debug edges on meshes
    #[cfg(debug_assertions)]
    pub const DEBUG_EDGES: H<Shader> = H::new(Self::DEBUG_EDGES_ID);

    // An addon shader ID that is used for drawing debug vertex normals on meshes
    #[cfg(debug_assertions)]
    pub const DEBUG_VERTEX_NORMALS: H<Shader> = H::new(Self::DEBUG_VERTEX_NORMALS_ID);

    // An addon shader ID that is used for drawing debug lines
    #[cfg(debug_assertions)]
    pub const DEBUG_LINES: H<Shader> = H::new(Self::DEBUG_LINES_ID);
    #[cfg(debug_assertions)]
    pub const DEBUG_TEXT2D_GEOMETRY: H<Shader> = H::new(Self::DEBUG_TEXT2D_GEOMETRY_ID);
    #[cfg(debug_assertions)]
    pub const DEBUG_TEXT3D_GEOMETRY: H<Shader> = H::new(Self::DEBUG_TEXT3D_GEOMETRY_ID);
    #[cfg(debug_assertions)]
    pub const DEBUG_LIGHT: H<Shader> = H::new(Self::DEBUG_LIGHT_ID);

    // Skybox Cubemap shader
    pub const SKYBOX_CUBEMAP: H<Shader> = H::new(Self::SKYBOX_CUBEMAP_ID);
}

const SHADER_FALLBACK3D: &str = include_str!("shaders/fallback_shader3d.wgsl");
const SHADER_DIM2: &str = include_str!("shaders/shader2d.wgsl");
const SHADER_DIM3: &str = include_str!("shaders/shader3d.wgsl");
const SHADER_TEXT2D: &str = include_str!("shaders/text2d.wgsl");
const SHADER_TEXT3D: &str = include_str!("shaders/text3d.wgsl");
const SHADER_FS_COPY: &str = include_str!("shaders/fullscreen_passhthrough.wgsl");

#[cfg(debug_assertions)]
const DEBUG_EDGES_SHADER: &str = include_str!("shaders/debug/edges.wgsl");
#[cfg(debug_assertions)]
const DEBUG_VERTEX_NORMAL_SHADER: &str = include_str!("shaders/debug/vertex_normals.wgsl");
#[cfg(debug_assertions)]
const DEBUG_LINES_SHADER: &str = include_str!("shaders/debug/lines.wgsl");
#[cfg(debug_assertions)]
const DEBUG_TEXT2D_GEOMETRY: &str = include_str!("shaders/debug/text2d_geometry.wgsl");
#[cfg(debug_assertions)]
const DEBUG_TEXT3D_GEOMETRY: &str = include_str!("shaders/debug/text3d_geometry.wgsl");
#[cfg(debug_assertions)]
const DEBUG_LIGHT_SHADER: &str = include_str!("shaders/debug/light.wgsl");

const SHADER_SKYBOX_CUBEMAP: &str = include_str!("shaders/skybox/skybox_cubemap.wgsl");

impl StoreDefaults for Shader {
    fn populate(store: &mut Store<Self>) {
        store_add_checked_many!(store,
            HShader::FALLBACK_ID => Shader::new_default("Fallback", SHADER_FALLBACK3D),
            HShader::DIM2_ID => Shader::new_default("2D Default Pipeline", SHADER_DIM2),
            HShader::DIM3_ID => Shader::new_fragment("3D Default Pipeline", SHADER_DIM3),
            HShader::POST_PROCESS_ID => Shader::new_post_process("Post Process", SHADER_FS_COPY),
        );

        const TEXT_VBL: &[VertexBufferLayout] = &[VertexBufferLayout {
            array_stride: VEC2_SIZE * 2,
            step_mode: VertexStepMode::Vertex,
            attributes: &[
                VertexAttribute {
                    format: VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x2,
                    offset: VEC2_SIZE,
                    shader_location: 1,
                },
            ],
        }];

        const TEXT_PC: &[PushConstantRange] = &[PushConstantRange {
            stages: ShaderStages::VERTEX_FRAGMENT,
            range: 0..size_of::<TextPushConstants>() as u32,
        }];

        store_add_checked!(
            store,
            HShader::TEXT_2D_ID,
            Shader::Custom {
                name: "Text 2D Shader".to_string(),
                code: ShaderCode::Full(SHADER_TEXT2D.to_string()),
                polygon_mode: PolygonMode::Fill,
                topology: PrimitiveTopology::TriangleList,
                vertex_buffers: TEXT_VBL,
                push_constant_ranges: TEXT_PC,
                shadow_transparency: false,
            }
        );

        store_add_checked!(
            store,
            HShader::TEXT_3D_ID,
            Shader::Custom {
                name: "Text 3D Shader".to_string(),
                code: ShaderCode::Full(SHADER_TEXT3D.to_string()),
                polygon_mode: PolygonMode::Fill,
                topology: PrimitiveTopology::TriangleList,
                vertex_buffers: TEXT_VBL,
                push_constant_ranges: TEXT_PC,
                shadow_transparency: true,
            }
        );
        store_add_checked!(
            store,
            HShader::SKYBOX_CUBEMAP_ID,
            Shader::new_default("Skybox Cubemap", SHADER_SKYBOX_CUBEMAP)
        );
        #[cfg(debug_assertions)]
        {
            use crate::rendering::DEFAULT_VBL;
            use crate::utils::sizes::{VEC3_SIZE, VEC4_SIZE, WGPU_VEC4_ALIGN};
            use wgpu::{VertexAttribute, VertexFormat, VertexStepMode};

            store_add_checked!(
                store,
                HShader::DEBUG_EDGES_ID,
                Shader::Custom {
                    name: "Mesh Debug Edges Shader".to_owned(),
                    code: ShaderCode::Full(DEBUG_EDGES_SHADER.to_string()),
                    polygon_mode: PolygonMode::Line,
                    topology: PrimitiveTopology::TriangleList,
                    vertex_buffers: &DEFAULT_VBL,
                    push_constant_ranges: &[PushConstantRange {
                        stages: ShaderStages::FRAGMENT,
                        range: 0..WGPU_VEC4_ALIGN as u32,
                    }],
                    shadow_transparency: false,
                }
            );

            store_add_checked!(
                store,
                HShader::DEBUG_VERTEX_NORMALS_ID,
                Shader::Custom {
                    name: "Mesh Debug Vertices Shader".to_owned(),
                    code: ShaderCode::Full(DEBUG_VERTEX_NORMAL_SHADER.to_string()),
                    topology: PrimitiveTopology::LineList,
                    polygon_mode: PolygonMode::Line,
                    vertex_buffers: &DEFAULT_VBL_STEP_INSTANCE,
                    push_constant_ranges: &[],
                    shadow_transparency: false,
                }
            );

            const DEBUG_LINE_VBL: &[VertexBufferLayout] = &[VertexBufferLayout {
                array_stride: 0,
                step_mode: VertexStepMode::Instance,
                attributes: &[
                    VertexAttribute {
                        format: VertexFormat::Float32x3, // start position
                        offset: 0,
                        shader_location: 0,
                    },
                    VertexAttribute {
                        format: VertexFormat::Float32x3, // end position
                        offset: VEC3_SIZE,
                        shader_location: 1,
                    },
                    VertexAttribute {
                        format: VertexFormat::Float32x4, // start color
                        offset: VEC3_SIZE * 2,
                        shader_location: 2,
                    },
                    VertexAttribute {
                        format: VertexFormat::Float32x4, // end color
                        offset: VEC3_SIZE * 2 + VEC4_SIZE,
                        shader_location: 3,
                    },
                ],
            }];

            store_add_checked!(
                store,
                HShader::DEBUG_LINES_ID,
                Shader::Custom {
                    name: "Line Debug".to_owned(),
                    code: ShaderCode::Full(DEBUG_LINES_SHADER.to_string()),
                    topology: PrimitiveTopology::LineList,
                    polygon_mode: PolygonMode::Line,
                    vertex_buffers: DEBUG_LINE_VBL,
                    push_constant_ranges: &[],
                    shadow_transparency: false,
                }
            );

            const DEBUG_TEXT: &[VertexBufferLayout] = &[VertexBufferLayout {
                array_stride: VEC2_SIZE * 2,
                step_mode: VertexStepMode::Vertex,
                attributes: &[VertexAttribute {
                    format: VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                }], // dont need atlas uv
            }];

            store_add_checked!(
                store,
                HShader::DEBUG_TEXT2D_GEOMETRY_ID,
                Shader::Custom {
                    name: "Debug 2D Text Geometry Shader".to_owned(),
                    code: ShaderCode::Full(DEBUG_TEXT2D_GEOMETRY.to_string()),
                    polygon_mode: PolygonMode::Line,
                    topology: PrimitiveTopology::TriangleList,
                    vertex_buffers: DEBUG_TEXT,
                    push_constant_ranges: TEXT_PC,
                    shadow_transparency: false,
                }
            );

            store_add_checked!(
                store,
                HShader::DEBUG_TEXT3D_GEOMETRY_ID,
                Shader::Custom {
                    name: "Debug 3D Text Geometry Shader".to_owned(),
                    code: ShaderCode::Full(DEBUG_TEXT3D_GEOMETRY.to_string()),
                    polygon_mode: PolygonMode::Line,
                    topology: PrimitiveTopology::TriangleList,
                    vertex_buffers: DEBUG_TEXT,
                    push_constant_ranges: TEXT_PC,
                    shadow_transparency: false,
                }
            );

            store_add_checked!(
                store,
                HShader::DEBUG_LIGHT_ID,
                Shader::Custom {
                    name: "Light Debug".to_owned(),
                    code: ShaderCode::Full(DEBUG_LIGHT_SHADER.to_string()),
                    topology: PrimitiveTopology::LineList,
                    polygon_mode: PolygonMode::Line,
                    vertex_buffers: &[],
                    push_constant_ranges: &[PushConstantRange {
                        stages: ShaderStages::VERTEX,
                        range: 0..4,
                    }],
                    shadow_transparency: false,
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
        self.name()
    }
}

impl StoreType for Shader {
    #[inline]
    fn name() -> &'static str {
        "Shader"
    }

    fn ident_fmt(handle: H<Self>) -> HandleName<Self> {
        let name = match handle.id() {
            HShader::FALLBACK_ID => "Diffuse Fallback",
            HShader::DIM2_ID => "2 Dimensional Shader",
            HShader::DIM3_ID => "3 Dimensional Shader",
            HShader::TEXT_2D_ID => "2D Text Shader",
            HShader::TEXT_3D_ID => "3D Text Shader",
            HShader::POST_PROCESS_ID => "Post Process Shader",

            #[cfg(debug_assertions)]
            HShader::DEBUG_EDGES_ID => "Debug Edges Shader",
            #[cfg(debug_assertions)]
            HShader::DEBUG_VERTEX_NORMALS_ID => "Debug Vertex Normals Shader",
            #[cfg(debug_assertions)]
            HShader::DEBUG_LINES_ID => "Debug Rays Shader",
            #[cfg(debug_assertions)]
            HShader::DEBUG_TEXT2D_GEOMETRY_ID => "Debug Text 2D Geometry Shader",
            #[cfg(debug_assertions)]
            HShader::DEBUG_TEXT3D_GEOMETRY_ID => "Debug Text 3D Geometry Shader",
            #[cfg(debug_assertions)]
            HShader::DEBUG_LIGHT_ID => "Debug Lights Shader",
            HShader::SKYBOX_CUBEMAP_ID => "Skybox Cubemap Shader",

            _ => return HandleName::Id(handle),
        };

        HandleName::Static(name)
    }

    fn is_builtin(handle: H<Self>) -> bool {
        handle.id() <= H::MAX_BUILTIN_ID
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
            Shader::Default { polygon_mode, .. } | Shader::Custom { polygon_mode, .. } => {
                *polygon_mode
            }
            Shader::PostProcess { .. } => PolygonMode::Fill,
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

    pub fn push_constant_ranges(&self) -> &'static [PushConstantRange] {
        match self {
            Shader::Default { .. } => &[],
            Shader::PostProcess { .. } => &[],
            Shader::Custom {
                push_constant_ranges,
                ..
            } => push_constant_ranges,
        }
    }

    pub fn is_custom(&self) -> bool {
        matches!(self, Shader::Custom { .. })
    }

    pub fn is_post_process(&self) -> bool {
        matches!(self, Shader::PostProcess { .. })
    }

    pub fn has_shadow_transparency(&self) -> bool {
        matches!(
            self,
            Shader::Custom {
                shadow_transparency: true,
                ..
            }
        )
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
            HBGL::MATERIAL_ID => "material",
            HBGL::LIGHT_ID => "light",
            HBGL::SHADOW_ID => "shadow",

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

    pub(crate) fn solid_layout(&self, device: &Device, cache: &AssetCache) -> PipelineLayout {
        let layout_name = format!("{} Pipeline Layout", self.name());

        let cam_bgl = cache.bgl_render();
        let mdl_bgl = cache.bgl_model();
        let mat_bgl = cache.bgl_material();
        let lgt_bgl = cache.bgl_light();
        let sdw_bgl = cache.bgl_shadow();
        let pp_bgl = cache.bgl_post_process();
        let empty_bgl = cache.bgl_empty();

        let mut slots: [Option<&BindGroupLayout>; 6] = [None; 6];
        slots[0] = Some(&cam_bgl);

        if self.is_post_process() {
            slots[1] = Some(&pp_bgl);
        } else {
            if self.needs_bgl(HBGL::MODEL) {
                slots[1] = Some(&mdl_bgl);
            }
            if self.needs_bgl(HBGL::MATERIAL) {
                slots[2] = Some(&mat_bgl);
            }
            if self.needs_bgl(HBGL::LIGHT) {
                slots[3] = Some(&lgt_bgl);
            }
            if self.needs_bgl(HBGL::SHADOW) {
                slots[4] = Some(&sdw_bgl);
            }
        }

        let last = slots.iter().rposition(|s| s.is_some()).unwrap_or(0);
        let fixed: Vec<&BindGroupLayout> =
            (0..=last).map(|i| slots[i].unwrap_or(&empty_bgl)).collect();

        self.layout_with(device, &layout_name, &fixed)
    }

    pub(crate) fn shadow_layout(
        &self,
        device: &Device,
        cache: &AssetCache,
    ) -> Option<PipelineLayout> {
        if self.is_post_process() {
            return None;
        }

        let layout_name = format!("{} Shadow Pipeline Layout", self.name());

        let cam_bgl = cache.bgl_render();
        let mdl_bgl = cache.bgl_model();
        let mat_bgl = cache.bgl_material();
        let lgt_bgl = cache.bgl_light();
        let empty_bgl = cache.bgl_empty();

        let mut slots: [Option<&BindGroupLayout>; 6] = [None; 6];
        slots[0] = Some(&cam_bgl);

        if self.needs_bgl(HBGL::MODEL) {
            slots[1] = Some(&mdl_bgl);
        }
        if self.needs_bgl(HBGL::MATERIAL) {
            slots[2] = Some(&mat_bgl);
        }
        if self.needs_bgl(HBGL::LIGHT) {
            slots[3] = Some(&lgt_bgl);
        }

        let last = slots.iter().rposition(|s| s.is_some()).unwrap_or(0);
        let fixed: Vec<&BindGroupLayout> =
            (0..=last).map(|i| slots[i].unwrap_or(&empty_bgl)).collect();

        Some(self.layout_with(device, &layout_name, &fixed))
    }

    fn layout_with(
        &self,
        device: &Device,
        layout_name: &str,
        fixed_bgls: &[&BindGroupLayout],
    ) -> PipelineLayout {
        let desc = PipelineLayoutDescriptor {
            label: Some(layout_name),
            bind_group_layouts: fixed_bgls,
            push_constant_ranges: self.push_constant_ranges(),
        };
        device.create_pipeline_layout(&desc)
    }
}
