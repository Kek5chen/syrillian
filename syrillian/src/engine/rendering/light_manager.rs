use crate::assets::{HTexture, Ref, Store, StoreType, Texture};
use crate::components::TypedComponentId;
use crate::rendering::AssetCache;
use crate::rendering::lights::{LightProxy, LightUniformIndex, ShadowUniformIndex};
use crate::rendering::message::LightProxyCommand;
use crate::rendering::uniform::ShaderUniform;
use itertools::Itertools;
use log::warn;
use syrillian_utils::debug_panic;
use wgpu::{
    AddressMode, Device, FilterMode, Queue, Sampler, SamplerBorderColor, SamplerDescriptor,
    TextureUsages, TextureView, TextureViewDescriptor,
};

#[cfg(debug_assertions)]
use crate::rendering::Renderer;
#[cfg(debug_assertions)]
use crate::rendering::lights::LightType;

const DUMMY_POINT_LIGHT: LightProxy = LightProxy::dummy();

pub struct LightManager {
    proxy_owners: Vec<TypedComponentId>,
    proxies: Vec<LightProxy>,

    uniform: ShaderUniform<LightUniformIndex>,
    shadow_uniform: ShaderUniform<ShadowUniformIndex>,
    pub(crate) shadow_texture: HTexture,
    pub(crate) _shadow_sampler: Sampler,
}

impl LightManager {
    pub fn update_shadow_map_ids(&mut self, layers: u32) -> u32 {
        if self.proxies.len() > layers as usize {
            debug_panic!("Too many lights for shadow map array");
            return 0;
        }

        let mut id = 0;
        for light in &mut self.proxies {
            if id >= layers {
                light.shadow_map_id = u32::MAX;
            } else {
                light.shadow_map_id = id;
                id += 1;
            }
        }

        id
    }

    pub fn add_proxy(&mut self, owner: TypedComponentId, proxy: LightProxy) {
        self.proxies.push(proxy);
        self.proxy_owners.push(owner);
    }

    pub fn remove_proxy(&mut self, owner: TypedComponentId) {
        let Some((pos, _)) = self
            .proxy_owners
            .iter()
            .find_position(|tcid| **tcid == owner)
        else {
            return;
        };

        self.proxy_owners.remove(pos);
        self.proxies.remove(pos);
    }

    pub fn execute_light_command(&mut self, owner: TypedComponentId, cmd: LightProxyCommand) {
        let Some((pos, _)) = self
            .proxy_owners
            .iter()
            .find_position(|tcid| **tcid == owner)
        else {
            warn!("Requested Light Proxy not found");
            return;
        };

        let Some(proxy) = self.proxies.get_mut(pos) else {
            debug_panic!("Light Proxy and Light Owners desynchronized");
            return;
        };

        cmd(proxy);
    }

    pub fn shadow_array<'a>(&self, assets: &'a Store<Texture>) -> Option<Ref<'a, Texture>> {
        Some(assets.get(self.shadow_texture))
    }

    pub fn shadow_layer(&self, cache: &AssetCache, layer: u32) -> TextureView {
        let texture = &cache.texture(self.shadow_texture).texture;
        texture.create_view(&TextureViewDescriptor {
            label: Some("Shadow Map Layer"),
            format: Some(wgpu::TextureFormat::Depth32Float),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: layer,
            array_layer_count: Some(1),
            usage: Some(TextureUsages::RENDER_ATTACHMENT),
        })
    }

    pub fn uniform(&self) -> &ShaderUniform<LightUniformIndex> {
        &self.uniform
    }

    pub fn shadow_uniform(&self) -> &ShaderUniform<ShadowUniformIndex> {
        &self.shadow_uniform
    }

    pub fn light_for_layer(&self, layer: u32) -> Option<&LightProxy> {
        self.proxies.iter().find(|l| l.shadow_map_id == layer)
    }

    pub fn new(cache: &AssetCache, device: &Device) -> Self {
        const DUMMY_POINT_LIGHT: LightProxy = LightProxy::dummy();

        let shadow_texture =
            Texture::new_2d_shadow_map_array(8, 1024, 1024).store(&cache.textures.store());
        let texture = cache
            .textures
            .try_get(shadow_texture, cache)
            .unwrap_or_else(|| {
                log::error!("Failed to get a Texture in light_for_layer in the LightManager");
                std::process::exit(1);
            });

        let bgl = cache.bgl_light();
        let count: u32 = 0;
        let uniform = ShaderUniform::builder(&bgl)
            .with_buffer_data(&count)
            .with_storage_buffer_data(&[DUMMY_POINT_LIGHT])
            .build(device);

        let shadow_sampler = device.create_sampler(&SamplerDescriptor {
            label: None,
            address_mode_u: AddressMode::ClampToBorder,
            address_mode_v: AddressMode::ClampToBorder,
            address_mode_w: AddressMode::ClampToBorder,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: 0.0,
            compare: Some(wgpu::CompareFunction::LessEqual),
            anisotropy_clamp: 1,
            border_color: Some(SamplerBorderColor::OpaqueWhite),
        });

        let bgl = cache.bgl_shadow();
        let shadow_uniform = ShaderUniform::builder(&bgl)
            .with_texture(&texture.view)
            .with_sampler(&shadow_sampler)
            .build(device);

        Self {
            proxy_owners: vec![],
            proxies: vec![],
            uniform,
            shadow_uniform,
            shadow_texture,
            _shadow_sampler: shadow_sampler,
        }
    }

    pub fn update(&mut self, cache: &AssetCache, queue: &Queue, device: &Device) {
        let queue = &queue;
        let proxies = proxy_buffer_slice(&self.proxies);
        let size = proxies.len();

        let count = self.uniform.buffer(LightUniformIndex::Count);
        queue.write_buffer(count, 0, bytemuck::bytes_of(&(size as u32)));

        let data = self.uniform.buffer(LightUniformIndex::Lights);
        if size_of_val(proxies) > data.size() as usize {
            let bgl = cache.bgl_light();
            self.uniform = ShaderUniform::builder(&bgl)
                .with_buffer(count.clone())
                .with_storage_buffer_data(proxies)
                .build(device);
        } else {
            queue.write_buffer(data, 0, bytemuck::cast_slice(proxies));
        }
    }

    #[cfg(debug_assertions)]
    pub fn render_debug_lights(&self, renderer: &Renderer, ctx: &crate::rendering::GPUDrawCtx) {
        use crate::assets::HShader;
        use wgpu::ShaderStages;

        let mut pass = ctx.pass.write().unwrap_or_else(|_| {
            log::error!("Failed to obtain a writable pass in render_debug_lights in LightManager");
            std::process::exit(1);
        });

        let shader = renderer.cache.shader(HShader::DEBUG_LIGHT);
        crate::must_pipeline!(pipeline = shader, ctx.pass_type => return);

        pass.set_pipeline(pipeline);

        let lights = self.proxies.as_slice();
        for (i, proxy) in lights.iter().enumerate().take(self.proxies.len()) {
            let type_id: LightType = match proxy.type_id.try_into() {
                Ok(ty) => ty,
                Err(e) => {
                    debug_panic!("{}", e);
                    continue;
                }
            };

            pass.set_push_constants(ShaderStages::VERTEX, 0, bytemuck::bytes_of(&(i as u32)));
            match type_id {
                LightType::Point => pass.draw(0..2, 0..6),
                LightType::Sun => pass.draw(0..2, 0..9),
                LightType::Spot => pass.draw(0..2, 0..9),
            }
        }
    }
}

pub fn proxy_buffer_slice(proxies: &[LightProxy]) -> &[LightProxy] {
    if proxies.is_empty() {
        &[DUMMY_POINT_LIGHT]
    } else {
        proxies
    }
}
