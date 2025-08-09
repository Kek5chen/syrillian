use crate::assets::HBGL;
use crate::engine::assets::Shader;
use crate::engine::rendering::cache::generic_cache::CacheType;
use crate::engine::rendering::cache::AssetCache;
use crate::rendering::RenderPipelineBuilder;
use std::borrow::Cow;
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

    let cam_bgl = cache.bgl_render();
    let mdl_bgl = cache.bgl_model();
    let mat_bgl = cache.bgl_material();
    let lgt_bgl = cache.bgl_light();
    let pp_bgl = cache.bgl_post_process();
    let empty_bgl = cache.bgl_empty();

    let mut slots: [Option<&BindGroupLayout>; 5] = [None; 5];
    slots[0] = Some(&cam_bgl);

    if matches!(shader, Shader::PostProcess { .. }) {
        slots[1] = Some(&pp_bgl);
    } else {
        if shader.needs_bgl(HBGL::MODEL) {
            slots[1] = Some(&mdl_bgl);
            slots[2] = Some(&mat_bgl);
        }
        if shader.needs_bgl(HBGL::LIGHT) {
            slots[3] = Some(&lgt_bgl);
        }
    }

    let last = slots.iter().rposition(|s| s.is_some()).unwrap_or(0);
    let fixed: Vec<&BindGroupLayout> = (0..=last).map(|i| slots[i].unwrap_or(&empty_bgl)).collect();

    let desc = PipelineLayoutDescriptor {
        label: Some(&layout_name),
        bind_group_layouts: &fixed,
        push_constant_ranges: &shader.push_constant_ranges(),
    };
    device.create_pipeline_layout(&desc)
}
