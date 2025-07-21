use crate::assets::HBGL;
use crate::engine::assets::Shader;
use crate::engine::rendering::cache::generic_cache::CacheType;
use crate::engine::rendering::cache::AssetCache;
use crate::rendering::RenderPipelineBuilder;
use std::borrow::Cow;
use std::sync::Arc;
use wgpu::*;

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
            source: ShaderSource::Wgsl(Cow::Owned(self.gen_code())),
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
            let mut bgls: Vec<Arc<BindGroupLayout>> = Vec::new();

            bgls.push(cache.bgl_render());
            if shader.needs_bgl(HBGL::MODEL) {
                bgls.push(cache.bgl_model());
                bgls.push(cache.bgl_material());
            }
            if shader.needs_bgl(HBGL::LIGHT) {
                bgls.push(cache.bgl_light());
            }

            let bind_group_layouts: Vec<&BindGroupLayout> = bgls.iter().map(Arc::as_ref).collect();

            device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some(&layout_name),
                bind_group_layouts: &bind_group_layouts,
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
