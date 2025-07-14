use std::default::Default;
use wgpu::*;

use crate::core::Vertex3D;
use crate::engine::assets::{Shader, ShaderStage};
use crate::engine::rendering::cache::AssetCache;
use crate::engine::rendering::cache::generic_cache::CacheType;

#[derive(Debug, Clone)]
pub struct RuntimeShader {
    pub module: ShaderModule,
    pub pipeline_layout: PipelineLayout,
    pub pipeline: RenderPipeline,
}

impl CacheType for Shader {
    type Hot = RuntimeShader;

    fn upload(&self, device: &Device, _queue: &Queue, cache: &AssetCache) -> Self::Hot {
        match self.stage {
            ShaderStage::Default => upload_default(self, device, cache),
            ShaderStage::PostProcess => upload_post_process(self, device, cache),
        }
    }
}

fn upload_default(shader: &Shader, device: &Device, cache: &AssetCache) -> RuntimeShader {
    let module = device.create_shader_module(ShaderModuleDescriptor {
        label: Some(&shader.name),
        source: ShaderSource::Wgsl(shader.gen_code()),
    });

    let render_bgl = cache.bgl_render();
    let model_bgl = cache.bgl_model();
    let material_bgl = cache.bgl_material();
    let light_bgl = cache.bgl_light();

    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some(&format!("{} Pipeline Layout", shader.name)),
        bind_group_layouts: &[&render_bgl, &model_bgl, &material_bgl, &light_bgl],
        push_constant_ranges: &[],
    });

    let pipeline =
        RenderPipelineBuilder::dim3(&shader.name, &pipeline_layout, &module, shader.polygon_mode)
            .build(&device);

    RuntimeShader {
        module,
        pipeline_layout,
        pipeline,
    }
}

pub fn upload_post_process(shader: &Shader, device: &Device, cache: &AssetCache) -> RuntimeShader {
    let module = device.create_shader_module(ShaderModuleDescriptor {
        label: Some(&shader.name),
        source: ShaderSource::Wgsl(shader.gen_code()),
    });

    let pp_bgl = cache.bgl_post_process();

    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some(&format!("{} PostProcess Pipeline Layout", shader.name)),
        bind_group_layouts: &[&pp_bgl],
        push_constant_ranges: &[],
    });

    let pipeline =
        RenderPipelineBuilder::post_process(&shader.name, &pipeline_layout, &module).build(&device);

    RuntimeShader {
        module,
        pipeline_layout,
        pipeline,
    }
}

pub struct RenderPipelineBuilder<'a> {
    label: String,
    layout: &'a PipelineLayout,
    shader: &'a ShaderModule,
    is_post_process: bool,
    polygon_mode: PolygonMode,
}

impl<'a> RenderPipelineBuilder<'a> {
    pub fn build(&'a self, device: &Device) -> RenderPipeline {
        device.create_render_pipeline(&self.desc())
    }
    pub fn desc(&'a self) -> RenderPipelineDescriptor<'a> {
        const DEFAULT_BUFFERS: [VertexBufferLayout; 1] = [Vertex3D::continuous_descriptor()];

        const DEFAULT_COLOR_TARGET_STATE: [Option<ColorTargetState>; 1] =
            [Some(ColorTargetState {
                format: TextureFormat::Bgra8UnormSrgb,
                blend: None,
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

        let buffers = (!self.is_post_process)
            .then_some(DEFAULT_BUFFERS.as_ref())
            .unwrap_or(&[]);
        let depth_stencil = (!self.is_post_process).then_some(DEFAULT_DEPTH_STENCIL);
        let cull_mode = (!self.is_post_process).then_some(Face::Back);

        RenderPipelineDescriptor {
            label: Some(&self.label),
            layout: Some(self.layout),

            vertex: VertexState {
                module: self.shader,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers,
            },
            primitive: PrimitiveState {
                cull_mode,
                polygon_mode: self.polygon_mode,
                ..PrimitiveState::default()
            },
            depth_stencil,
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                module: self.shader,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &DEFAULT_COLOR_TARGET_STATE,
            }),
            multiview: None,
            cache: None,
        }
    }

    pub fn post_process(
        name: &str,
        layout: &'a PipelineLayout,
        shader: &'a ShaderModule,
    ) -> RenderPipelineBuilder<'a> {
        RenderPipelineBuilder {
            label: format!("{name} PostProcess Pipeline"),
            layout,
            shader,
            is_post_process: true,
            polygon_mode: PolygonMode::Fill,
        }
    }

    pub fn dim3(
        name: &str,
        layout: &'a PipelineLayout,
        shader: &'a ShaderModule,
        polygon_mode: PolygonMode,
    ) -> RenderPipelineBuilder<'a> {
        RenderPipelineBuilder {
            label: format!("{name} Pipeline"),
            layout,
            shader,
            is_post_process: false,
            polygon_mode,
        }
    }
}
