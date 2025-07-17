use crate::engine::assets::generic_store::{HandleName, Store, StoreDefaults, StoreType};
use crate::engine::assets::{H, HShader, StoreTypeFallback, StoreTypeName};
use std::borrow::Cow;
use std::error::Error;
use std::fs;
use std::path::Path;
use bon::Builder;
use wgpu::PolygonMode;

#[derive(Debug, Clone, Eq, PartialEq, Builder)]
pub struct Shader {
    pub name: String,
    pub code: String,
    #[builder(default = PolygonMode::Fill)]
    pub polygon_mode: PolygonMode,
    #[builder(default = false)]
    pub draw_over: bool,
    #[builder(default = ShaderStage::Default)]
    pub stage: ShaderStage,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ShaderStage {
    Default,
    PostProcess,
}

impl H<Shader> {
    pub const FALLBACK_ID: u32 = 0;
    pub const DIM3_ID: u32 = 1;
    pub const DIM2_ID: u32 = 2;
    pub const POST_PROCESS_ID: u32 = 3;
    #[cfg(debug_assertions)]
    pub const DEBUG_EDGES_ID: u32 = 4;

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
            let shader = store.add(Shader {
                name: "3D Debug Edges Shader".to_owned(),
                code: include_str!("../rendering/shaders/debug/edges.wgsl").to_string(),
                polygon_mode: PolygonMode::Line,
                draw_over: true,
                stage: ShaderStage::Default,
            });
            assert_eq!(shader, HShader::DEBUG_EDGES);
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
        &self.name
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
    pub fn gen_code(&self) -> Cow<'static, str> {
        let code = match self.stage {
            ShaderStage::Default => format!("{}\n{}", &self.code, SHADER_PRE_CONTEXT),
            ShaderStage::PostProcess => {
                format!("{}\n{}", &self.code, POST_PROCESS_SHADER_PRE_CONTEXT)
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
        self.add(Shader {
            name,
            code,
            polygon_mode: PolygonMode::Fill,
            draw_over: true,
            stage: ShaderStage::PostProcess,
        })
    }

    pub fn add_default_shader(&self, name: String, code: String) -> H<Shader> {
        self.add(Shader {
            name,
            code,
            polygon_mode: PolygonMode::Fill,
            draw_over: false,
            stage: ShaderStage::Default,
        })
    }
}
