use wgpu::*;

use crate::engine::assets::Shader;
use crate::engine::rendering::cache::AssetCache;
use crate::engine::rendering::cache::generic_cache::CacheType;
use crate::rendering::RenderPipelineBuilder;

pub mod builder;

#[derive(Debug, Clone)]
pub struct RuntimeShader {
    pub module: ShaderModule,
    pub pipeline_layout: PipelineLayout,
    pub pipeline: RenderPipeline,
}

impl CacheType for Shader {
    type Hot = RuntimeShader;

    fn upload(&self, device: &Device, _queue: &Queue, cache: &AssetCache) -> Self::Hot {
        let module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some(self.name()),
            source: ShaderSource::Wgsl(self.gen_code()),
        });

        let pipeline_layout = make_layout(self, device, cache);

        let pipeline =
            RenderPipelineBuilder::builder(self, &pipeline_layout, &module).build(&device);

        RuntimeShader {
            module,
            pipeline_layout,
            pipeline,
        }
    }
}

fn make_layout(shader: &Shader, device: &Device, cache: &AssetCache) -> PipelineLayout {
    let layout_name = format!("{} Pipeline Layout", shader.name());

    match shader {
        Shader::Default { .. } | Shader::Custom { .. } => {
            let render_bgl = cache.bgl_render();
            let model_bgl = cache.bgl_model();
            let mat_bgl = cache.bgl_material();
            let light_bgl = cache.bgl_light();

            device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some(&layout_name),
                bind_group_layouts: &[&render_bgl, &model_bgl, &mat_bgl, &light_bgl],
                push_constant_ranges: &[],
            })
        }
        Shader::PostProcess { .. } => {
            let pp_bgl = cache.bgl_post_process();

            device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some(&layout_name),
                bind_group_layouts: &[&pp_bgl],
                push_constant_ranges: &[],
            })
        }
    }
}
