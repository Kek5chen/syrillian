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
    pub push_constant_ranges: &'static [PushConstantRange],
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
            push_constant_ranges: self.push_constant_ranges(),
            pipeline,
        }
    }
}

fn make_layout(shader: &Shader, device: &Device, cache: &AssetCache) -> PipelineLayout {
    let layout_name = format!("{} Pipeline Layout", shader.name());

    // lifetime stuff-
    let pp_bgl;
    let mut bgl_arcs: Vec<Arc<BindGroupLayout>>;
    let bind_group_layouts: Vec<&BindGroupLayout>;

    let desc = match shader {
        Shader::Default { .. } | Shader::Custom { .. } => {
            bgl_arcs = Vec::new();

            bgl_arcs.push(cache.bgl_render());
            if shader.needs_bgl(HBGL::MODEL) {
                bgl_arcs.push(cache.bgl_model());
                bgl_arcs.push(cache.bgl_material());
            }
            if shader.needs_bgl(HBGL::LIGHT) {
                bgl_arcs.push(cache.bgl_light());
            }

            bind_group_layouts = bgl_arcs.iter().map(Arc::as_ref).collect();

            PipelineLayoutDescriptor {
                label: Some(&layout_name),
                bind_group_layouts: &bind_group_layouts,
                push_constant_ranges: &shader.push_constant_ranges(),
            }
        }
        Shader::PostProcess { .. } => {
            pp_bgl = cache.bgl_post_process();

            PipelineLayoutDescriptor {
                label: Some(&layout_name),
                bind_group_layouts: &[&pp_bgl],
                push_constant_ranges: &[],
            }
        }
    };

    device.create_pipeline_layout(&desc)
}
