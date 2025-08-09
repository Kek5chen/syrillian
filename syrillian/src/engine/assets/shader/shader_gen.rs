use crate::assets::{PipelineStage, Shader, ShaderCode};
use log::warn;

const POST_PROCESS_HEADER: &str = include_str!("shaders/groups/post_process.wgsl");
const DEFAULT_HEADER: &str = include_str!("shaders/groups/basic.wgsl");
const LIGHT_GROUP: &str = include_str!("shaders/groups/light.wgsl");
const BASE_GROUP: &str = include_str!("shaders/groups/render.wgsl");
const MODEL_GROUP: &str = include_str!("shaders/groups/model.wgsl");
const DEFAULT_VERTEX_3D: &str = include_str!("shaders/default_vertex3d.wgsl");
const POST_PROCESS_VERTEX: &str = include_str!("shaders/default_vertex_post.wgsl");

pub struct ShaderGen<'a> {
    shader: &'a Shader,
}

impl<'a> ShaderGen<'a> {
    pub fn new(shader: &'a Shader) -> Self {
        Self { shader }
    }

    pub fn generate(self) -> String {
        let shader = self.shader;
        let code = shader.code();

        match shader.stage() {
            PipelineStage::Default => generate_default(code, shader.is_custom()),
            PipelineStage::PostProcess => generate_post_process(code),
        }
    }
}

fn generate_default(code: &ShaderCode, custom: bool) -> String {
    let mut generated = String::new();
    let fragment_only = code.is_only_fragment_shader();

    generated.push_str(BASE_GROUP);
    generated.push('\n');

    // if it's a fragment only, it doesn't matter if it's custom, because i can't have a clue what
    // the custom vertex shader should look like...
    if !custom {
        generated.push_str(DEFAULT_HEADER);
        generated.push('\n');
        generated.push_str(MODEL_GROUP);
        generated.push('\n');
        generated.push_str(LIGHT_GROUP);
        generated.push('\n');

        if fragment_only {
            generated.push_str(DEFAULT_VERTEX_3D);
            generated.push('\n');
        }

        generated.push_str(code.code());
        return generated;
    }

    // for custom shaders, we add to the shader what it needs using the use statements at the top
    for line in code.code().lines() {
        let Some(import) = line.find("#use ") else {
            generated.push_str(line);
            generated.push('\n');
            continue;
        };

        let group = line[import + 5..].trim();
        match group {
            "model" => generated.push_str(MODEL_GROUP),
            "light" => generated.push_str(LIGHT_GROUP),
            "default_vertex" => generated.push_str(DEFAULT_HEADER),

            _ => warn!("Shader use group {group} is invalid."),
        }
        generated.push('\n');
    }

    generated
}

fn generate_post_process(code: &ShaderCode) -> String {
    let mut generated = String::new();
    let fragment_only = code.is_only_fragment_shader();

    generated.push_str(POST_PROCESS_HEADER);
    generated.push('\n');

    if fragment_only {
        generated.push_str(POST_PROCESS_VERTEX);
        generated.push('\n');
    }

    generated.push_str(code.code());
    generated
}
