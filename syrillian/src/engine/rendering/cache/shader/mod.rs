use crate::engine::assets::Shader;
use crate::engine::rendering::cache::generic_cache::CacheType;
use crate::engine::rendering::cache::AssetCache;
use crate::rendering::{RenderPassType, RenderPipelineBuilder};
use std::borrow::Cow;
use wgpu::*;

pub mod builder;

#[derive(Debug, Clone)]
pub struct RuntimeShader {
    pub module: ShaderModule,
    pipeline: RenderPipeline,
    shadow_pipeline: Option<RenderPipeline>,
    pub push_constant_ranges: &'static [PushConstantRange],
}

impl CacheType for Shader {
    type Hot = RuntimeShader;

    fn upload(&self, device: &Device, _queue: &Queue, cache: &AssetCache) -> Self::Hot {
        let module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some(self.name()),
            source: ShaderSource::Wgsl(Cow::Owned(self.gen_code())),
        });

        let solid_layout = self.solid_layout(device, cache);
        let solid_builder = RenderPipelineBuilder::builder(self, &solid_layout, &module);
        let pipeline = solid_builder.build(device);
        let shadow_pipeline = self.shadow_layout(device, cache).and_then(|layout| {
            let shadow_builder = RenderPipelineBuilder::builder(self, &layout, &module);
            shadow_builder.build_shadow(&device)
        });

        RuntimeShader {
            module,
            pipeline,
            shadow_pipeline,
            push_constant_ranges: self.push_constant_ranges(),
        }
    }
}

impl RuntimeShader {
    pub fn solid_pipeline(&self) -> &RenderPipeline {
        &self.pipeline
    }

    pub fn shadow_pipeline(&self) -> Option<&RenderPipeline> {
        self.shadow_pipeline.as_ref()
    }

    pub fn pipeline(&self, stage: RenderPassType) -> Option<&RenderPipeline> {
        match stage {
            RenderPassType::Color => Some(&self.pipeline),
            RenderPassType::Shadow => self.shadow_pipeline.as_ref(),
        }
    }
}

#[macro_export]
macro_rules! must_pipeline {
    ($name:ident = $shader:expr, $pass_type:expr => $exit_strat:tt) => {
        let Some($name) = $shader.pipeline($pass_type) else {
            ::syrillian_utils::debug_panic!(
                "A 3D Shader was instantiated without a Shadow Pipeline Variant"
            );
            ::log::error!("A 3D Shader was instantiated without a Shadow Pipeline Variant");
            $exit_strat;
        };
    };
}
