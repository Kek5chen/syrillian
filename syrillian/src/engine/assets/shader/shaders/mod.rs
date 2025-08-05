macro_rules! test_shader {
    ($fn_name:ident, $name:literal => $path:literal) => {
        #[test]
        fn $fn_name() {
            use crate::assets::Shader;
            use crate::utils::validate_wgsl_source;

            let shader = Shader::new_default($name, include_str!($path)).gen_code();
            validate_wgsl_source(&shader)
                .inspect_err(|e| e.emit_to_stderr_with_path(&shader, $path))
                .unwrap();
        }
    };
}

macro_rules! test_post_shader {
    ($fn_name:ident, $name:literal => $path:literal) => {
        #[test]
        fn $fn_name() {
            use crate::assets::Shader;
            use crate::utils::validate_wgsl_source;

            let shader = Shader::new_post_process($name, include_str!($path)).gen_code();

            validate_wgsl_source(&shader)
                .inspect_err(|e| e.emit_to_stderr_with_path(&shader, $path))
                .unwrap();
        }
    };
}

macro_rules! test_custom_shader {
    ($fn_name:ident, $name:literal => $path:literal) => {
        #[test]
        fn $fn_name() {
            use crate::assets::Shader;
            use crate::assets::shader::{PolygonMode, PrimitiveTopology, ShaderCode};
            use crate::utils::validate_wgsl_source;

            let shader = Shader::Custom {
                name: $name.to_owned(),
                code: ShaderCode::Full(include_str!($path).to_string()),
                topology: PrimitiveTopology::TriangleList,
                polygon_mode: PolygonMode::Line,
                vertex_buffers: &[],
            }
            .gen_code();

            validate_wgsl_source(&shader)
                .inspect_err(|e| e.emit_to_stderr_with_path(&shader, $path))
                .unwrap();
        }
    };
}

// Fundamental Shaders
test_shader!(shader_2d, "Shader 2D" => "shader2d.wgsl");
test_shader!(shader_3d, "Shader 3D" => "shader3d.wgsl");
test_shader!(fallback_shader3d, "Fallback Shader 3D" => "fallback_shader3d.wgsl");

// Debug shaders
test_shader!(debug_edges, "Debug Edges Shader" => "debug/edges.wgsl");
test_custom_shader!(debug_rays, "Debug Rays Shader" => "debug/rays.wgsl");
test_custom_shader!(debug_vertex_normals, "Debug Vertex Normals" => "debug/vertex_normals.wgsl");

// Post-Processing Shaders
test_post_shader!(fullscreen_passthrough, "Fullscreen Passthrough Shader" => "fullscreen_passhthrough.wgsl");
