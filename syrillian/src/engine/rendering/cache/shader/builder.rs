use crate::assets::Shader;
use crate::core::Vertex3D;
use wgpu::{BlendState, ColorTargetState, ColorWrites, CompareFunction, DepthBiasState, DepthStencilState, Device, Face, FragmentState, MultisampleState, PipelineCompilationOptions, PipelineLayout, PolygonMode, PrimitiveState, PrimitiveTopology, RenderPipeline, RenderPipelineDescriptor, ShaderModule, StencilFaceState, StencilState, TextureFormat, VertexBufferLayout, VertexState};

const DEFAULT_BUFFERS: [VertexBufferLayout; 1] = [Vertex3D::continuous_descriptor()];

const DEFAULT_COLOR_TARGET_STATE: [Option<ColorTargetState>; 1] = [Some(ColorTargetState {
    format: TextureFormat::Bgra8UnormSrgb,
    blend: Some(BlendState::ALPHA_BLENDING),
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

pub struct RenderPipelineBuilder<'a> {
    label: String,
    layout: &'a PipelineLayout,
    module: &'a ShaderModule,
    is_post_process: bool,
    polygon_mode: PolygonMode,
    topology: PrimitiveTopology,
    vertex_buffers: &'a [VertexBufferLayout<'a>],
}

impl<'a> RenderPipelineBuilder<'a> {
    pub fn build(&'a self, device: &Device) -> RenderPipeline {
        device.create_render_pipeline(&self.desc())
    }

    pub fn desc(&'a self) -> RenderPipelineDescriptor<'a> {
        let depth_stencil = (!self.is_post_process).then_some(DEFAULT_DEPTH_STENCIL);
        let cull_mode = (!self.is_post_process).then_some(Face::Back);

        RenderPipelineDescriptor {
            label: Some(&self.label),
            layout: Some(self.layout),

            vertex: VertexState {
                module: self.module,
                entry_point: None,
                compilation_options: PipelineCompilationOptions::default(),
                buffers: self.vertex_buffers,
            },
            primitive: PrimitiveState {
                topology: self.topology,
                cull_mode,
                polygon_mode: self.polygon_mode,
                ..PrimitiveState::default()
            },
            depth_stencil,
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                module: self.module,
                entry_point: None,
                compilation_options: PipelineCompilationOptions::default(),
                targets: &DEFAULT_COLOR_TARGET_STATE,
            }),
            multiview: None,
            cache: None,
        }
    }

    pub fn builder(
        shader: &Shader,
        layout: &'a PipelineLayout,
        module: &'a ShaderModule,
    ) -> RenderPipelineBuilder<'a> {
        let name = shader.name();
        let polygon_mode = shader.polygon_mode();
        let topology = shader.topology();
        let is_post_process = matches!(shader, Shader::PostProcess { .. });

        let label = match shader {
            Shader::Default { .. } => format!("{name} Pipeline"),
            Shader::PostProcess { .. } => format!("{name} Post Process Pipeline"),
            Shader::Custom { .. } => format!("{name} Custom Pipeline"),
        };

        let vertex_buffers = match shader {
            Shader::Custom { vertex_buffers, .. } => *vertex_buffers,
            Shader::Default { .. } => &DEFAULT_BUFFERS,
            Shader::PostProcess { .. } => &[],
        };

        RenderPipelineBuilder {
            label,
            layout,
            module,
            is_post_process,
            polygon_mode,
            topology,
            vertex_buffers,
        }
    }
}
