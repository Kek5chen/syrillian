use crate::assets::{AssetStore, HTexture, Ref, StoreType, Texture};
use crate::components::Component;
use crate::rendering::uniform::{ResourceDesc, ShaderUniform};
use crate::rendering::{AssetCache, Renderer};
use crate::utils::hacks::DenseSlotMapDirectAccess;
use crate::utils::MATRIX4_ID;
use crate::{ensure_aligned, must_pipeline, World};
use delegate::delegate;
use nalgebra::{Matrix4, SimdPartialOrd, Vector3};
use num_enum::TryFromPrimitive;
use slotmap::{new_key_type, DenseSlotMap};
use syrillian_macros::UniformIndex;
use syrillian_utils::debug_panic;
use wgpu::{
    AddressMode, FilterMode, Sampler, SamplerBorderColor, SamplerDescriptor, TextureUsages,
    TextureView, TextureViewDescriptor,
};

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniform {
    pub position: Vector3<f32>,
    pub _p0: u32,
    pub direction: Vector3<f32>,
    pub range: f32,
    pub color: Vector3<f32>,
    pub intensity: f32,
    pub inner_angle: f32,
    pub outer_angle: f32,
    pub type_id: u32, // LightType
    pub shadow_map_id: u32,
    pub view_mat: Matrix4<f32>,
}

impl LightUniform {
    pub const fn dummy() -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            _p0: 0,
            direction: Vector3::new(0.0, -1.0, 0.0),
            range: 10.0,
            color: Vector3::new(1.0, 1.0, 1.0),
            intensity: 1.0,
            inner_angle: 0.0,
            outer_angle: 0.0,
            type_id: LightType::Point as u32,
            shadow_map_id: 0,
            view_mat: MATRIX4_ID,
        }
    }
}

ensure_aligned!(LightUniform { position, color, color, view_mat }, align <= 16 * 8 => size);

new_key_type! { pub struct LightHandle; }

pub trait Light: Component {
    fn light_handle(&self) -> LightHandle;
    fn light_type(&self) -> LightType;

    fn data<'a>(&self, world: &'a World) -> &'a LightUniform {
        let Some(light) = world.lights.get(self.light_handle()) else {
            debug_panic!("Light desynced");
            const DUMMY_LIGHT: LightUniform = LightUniform::dummy();
            return &DUMMY_LIGHT;
        };
        light
    }

    fn data_mut<'a>(&self, world: &'a mut World) -> &'a mut LightUniform {
        let Some(light) = world.lights.get_mut(self.light_handle()) else {
            panic!("Light desynced");
        };
        light
    }

    fn set_range(&mut self, world: &mut World, range: f32) {
        self.data_mut(world).range = range.max(0.);
    }

    fn set_intensity(&mut self, world: &mut World, intensity: f32) {
        self.data_mut(world).intensity = intensity.max(0.);
    }

    fn set_color(&mut self, world: &mut World, r: f32, g: f32, b: f32) {
        let light = self.data_mut(world);

        light.color.x = r.clamp(0., 1.);
        light.color.y = g.clamp(0., 1.);
        light.color.z = b.clamp(0., 1.);
    }

    fn set_color_vec(&mut self, world: &mut World, color: &Vector3<f32>) {
        let light = self.data_mut(world);
        light.color = color.simd_clamp(Vector3::new(0., 0., 0.), Vector3::new(1., 1., 1.));
    }
}

#[repr(u32)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, TryFromPrimitive)]
pub enum LightType {
    Point = 0,
    Sun = 1,
    Spot = 2,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, UniformIndex)]
pub enum LightUniformIndex {
    Count = 0,
    Lights = 1,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, UniformIndex)]
pub enum ShadowUniformIndex {
    ShadowMaps = 0,
    ShadowSampler = 1,
}

pub struct LightManager {
    uniform: Option<ShaderUniform<LightUniformIndex>>,
    shadow_uniform: Option<ShaderUniform<ShadowUniformIndex>>,
    inner: DenseSlotMap<LightHandle, LightUniform>,
    pub(crate) shadow_texture: Option<HTexture>,
    pub(crate) shadow_sampler: Option<Sampler>,
}

impl Default for LightManager {
    fn default() -> Self {
        Self {
            uniform: None,
            shadow_uniform: None,
            inner: DenseSlotMap::with_key(),
            shadow_texture: None,
            shadow_sampler: None,
        }
    }
}

impl LightManager {
    delegate! {
        to self.inner {
            pub fn get(&self, handle: LightHandle) -> Option<&LightUniform>;
            pub fn get_mut(&mut self, handle: LightHandle) -> Option<&mut LightUniform>;
            pub fn insert(&mut self, value: LightUniform) -> LightHandle;
            pub fn remove(&mut self, handle: LightHandle) -> Option<LightUniform>;
            pub fn len(&self) -> usize;
        }
    }

    pub fn update_shadow_map_ids(&mut self, layers: u32) -> u32 {
        if self.len() > layers as usize {
            debug_panic!("Too many lights for shadow map array");
            return 0;
        }

        let mut id = 0;
        for light in self.inner.values_mut() {
            if id >= layers {
                light.shadow_map_id = u32::MAX;
            } else {
                light.shadow_map_id = id;
                id += 1;
            }
        }

        id
    }

    pub fn shadow_array<'a>(&self, assets: &'a AssetStore) -> Option<Ref<'a, Texture>> {
        Some(assets.textures.get(self.shadow_texture.unwrap()))
    }

    pub fn shadow_layer(&self, cache: &AssetCache, layer: u32) -> TextureView {
        let texture = &cache.texture(self.shadow_texture.unwrap()).texture;
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

    pub fn register(&mut self) -> LightHandle {
        self.insert(LightUniform::dummy())
    }

    pub fn uniform(&self) -> &ShaderUniform<LightUniformIndex> {
        self.uniform
            .as_ref()
            .expect("Light data should be initialized")
    }

    pub fn shadow_uniform(&self) -> &ShaderUniform<ShadowUniformIndex> {
        self.shadow_uniform
            .as_ref()
            .expect("Shadow data should be initialized")
    }

    pub fn light_for_layer(&self, layer: u32) -> Option<&LightUniform> {
        self.inner.values().find(|l| l.shadow_map_id == layer)
    }

    pub fn init(&mut self, renderer: &Renderer, assets: &AssetStore) {
        let shadow_map = Texture::new_2d_shadow_map_array(8, 1024, 1024).store(&assets.textures);
        self.shadow_texture = Some(shadow_map);
        let texture = renderer
            .cache
            .textures
            .try_get(shadow_map, &renderer.cache)
            .unwrap();

        let bgl = renderer.cache.bgl_light();
        let count = self.len() as u32;
        let mut builder = ShaderUniform::builder(&bgl).with_buffer_data(&count);

        const DUMMY_POINT_LIGHT: LightUniform = LightUniform::dummy();

        if count == 0 {
            builder = builder.with_buffer_storage(&[DUMMY_POINT_LIGHT]);
        } else {
            builder = builder.with_buffer_storage(self.inner.as_slice());
        }
        self.uniform = Some(builder.build(&renderer.state.device));

        let sampler = renderer.state.device.create_sampler(&SamplerDescriptor {
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

        let bgl = renderer.cache.bgl_shadow();
        let shadow_uniform = ShaderUniform::builder(&bgl)
            .with_texture(&texture.view)
            .with_sampler(&sampler)
            .build(&renderer.state.device);
        self.shadow_uniform = Some(shadow_uniform);

        self.shadow_sampler = Some(sampler);
    }

    pub fn update(&mut self, renderer: &Renderer) {
        let queue = &renderer.state.queue;
        let size = self.len();

        let uniform = self.uniform.as_mut().unwrap();
        {
            let data = uniform.buffer(LightUniformIndex::Lights);

            if size * size_of::<LightUniform>() > data.size() as usize {
                uniform.set_buffer(
                    ResourceDesc::StorageBuffer {
                        data: bytemuck::cast_slice(self.inner.as_slice()),
                        name: LightUniformIndex::Lights,
                    },
                    &renderer.state.device,
                );
            }
        }

        let count = uniform.buffer(LightUniformIndex::Count);
        let data = uniform.buffer(LightUniformIndex::Lights);
        queue.write_buffer(count, 0, bytemuck::bytes_of(&(size as u32)));
        queue.write_buffer(data, 0, bytemuck::cast_slice(self.inner.as_slice()));
    }

    #[cfg(debug_assertions)]
    pub fn draw_debug_lights(&mut self, ctx: &crate::rendering::DrawCtx) {
        use crate::assets::HShader;
        use wgpu::ShaderStages;

        let mut pass = ctx.pass.write().unwrap();

        let shader = ctx.frame.cache.shader(HShader::DEBUG_LIGHT);
        must_pipeline!(pipeline = shader, ctx.pass_type => return);

        pass.set_pipeline(pipeline);

        let lights = self.inner.as_slice();
        for i in 0..self.len() {
            let type_id: LightType = match lights[i].type_id.try_into() {
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
