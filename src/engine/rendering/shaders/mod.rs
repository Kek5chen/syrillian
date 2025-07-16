macro_rules! test_shader {
    ($fn_name:ident, $name:literal => $path:literal) => {
        #[test]
        fn $fn_name() {
            use crate::assets::Shader;

            let shader = Shader::builder()
                .name($name.to_string())
                .code(include_str!($path).to_string())
                .build()
                .gen_code();

            wgpu::naga::front::wgsl::parse_str(&shader).unwrap();
        }
    };
}

macro_rules! test_post_shader {
    ($fn_name:ident, $name:literal => $path:literal) => {
        #[test]
        fn $fn_name() {
            use crate::assets::{Shader, ShaderStage};

            let shader = Shader::builder()
                .name($name.to_string())
                .code(include_str!($path).to_string())
                .stage(ShaderStage::PostProcess)
                .draw_over(true)
                .build()
                .gen_code();

            wgpu::naga::front::wgsl::parse_str(&shader).unwrap();
        }
    };
}

test_shader!(shader_2d, "Shader 2D" => "shader2d.wgsl");
test_shader!(shader_3d, "Shader 3D" => "shader3d.wgsl");
test_shader!(fallback_shader3d, "Fallback Shader 3D" => "fallback_shader3d.wgsl");

test_post_shader!(fullscreen_passthrough, "Fullscreen Passthrough Shader" => "fullscreen_passhthrough.wgsl");
test_post_shader!(debug_edges, "Debug Edges Shader" => "fullscreen_passhthrough.wgsl");
