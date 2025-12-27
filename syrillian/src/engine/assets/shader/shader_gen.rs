use crate::assets::{BindGroupMap, Shader, ShaderCode, ShaderType};
use tracing::warn;

const POST_PROCESS_HEADER: &str = include_str!("shaders/groups/post_process.wgsl");
const DEFAULT_HEADER: &str = include_str!("shaders/groups/basic.wgsl");
const LIGHT_GROUP: &str = include_str!("shaders/groups/light.wgsl");
const BASE_GROUP: &str = include_str!("shaders/groups/render.wgsl");
const MODEL_GROUP: &str = include_str!("shaders/groups/model.wgsl");
const MATERIAL_GROUP: &str = include_str!("shaders/groups/material.wgsl");
const DEFAULT_VERTEX_3D: &str = include_str!("shaders/default_vertex3d.wgsl");
const POST_PROCESS_VERTEX: &str = include_str!("shaders/default_vertex_post.wgsl");

pub struct ShaderGen<'a> {
    shader: &'a Shader,
    map: &'a BindGroupMap,
}

impl<'a> ShaderGen<'a> {
    pub fn new(shader: &'a Shader, map: &'a BindGroupMap) -> Self {
        Self { shader, map }
    }

    pub fn generate(self) -> String {
        let shader = self.shader;
        let code = shader.code();

        match shader.stage() {
            ShaderType::Default | ShaderType::Custom => generate_default(
                code,
                shader.is_custom(),
                shader.is_depth_enabled(),
                self.map,
            ),
            ShaderType::PostProcessing => generate_post_process(code, self.map),
        }
    }
}

fn generate_default(
    code: &ShaderCode,
    custom: bool,
    has_depth: bool,
    map: &BindGroupMap,
) -> String {
    let mut generated = format!("{BASE_GROUP}\n");
    let fragment_only = code.is_only_fragment_shader();

    // if it's a fragment only, it doesn't matter if it's custom, because I can't have a clue what
    // the custom vertex shader should look like...
    if !custom {
        generated.push_str(DEFAULT_HEADER);
        generated.push('\n');
        generated.push_str(MODEL_GROUP);
        generated.push('\n');
        generated.push_str(MATERIAL_GROUP);
        generated.push('\n');

        if has_depth {
            generated.push_str(LIGHT_GROUP);
            generated.push('\n');
        }

        if fragment_only {
            generated.push_str(DEFAULT_VERTEX_3D);
            generated.push('\n');
        }

        generated.push_str(code.code());
        return rewrite_bind_groups(generated, map);
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
            "material" => generated.push_str(MATERIAL_GROUP),
            "light" => generated.push_str(LIGHT_GROUP),
            "default_vertex" => generated.push_str(DEFAULT_HEADER),

            _ => warn!("Shader use group {group} is invalid."),
        }
        generated.push('\n');
    }

    if fragment_only {
        generated.push_str(DEFAULT_VERTEX_3D);
        generated.push('\n');
    }

    rewrite_bind_groups(generated, map)
}

fn generate_post_process(code: &ShaderCode, map: &BindGroupMap) -> String {
    let mut generated = format!("{BASE_GROUP}\n{POST_PROCESS_HEADER}\n");
    let fragment_only = code.is_only_fragment_shader();

    if fragment_only {
        generated.push_str(POST_PROCESS_VERTEX);
        generated.push('\n');
    }

    generated.push_str(code.code());
    rewrite_bind_groups(generated, map)
}

fn rewrite_bind_groups(source: String, map: &BindGroupMap) -> String {
    let mut out = source;

    let mut replace = |orig: u32, new_idx: u32| {
        let needle = format!("@group({orig})");
        let repl = format!("@group({new_idx})");
        out = out.replace(&needle, &repl);
    };

    replace(0, map.render);
    if let Some(idx) = map.model {
        replace(1, idx);
    }
    if let Some(idx) = map.material {
        replace(2, idx);
    }
    if let Some(idx) = map.light {
        replace(3, idx);
    }
    if let Some(idx) = map.shadow {
        replace(4, idx);
    }
    if let Some(idx) = map.post_process {
        replace(1, idx);
    }

    out
}
