use crate::engine::assets::generic_store::{HandleName, Store, StoreDefaults, StoreType};
use crate::engine::assets::*;
use crate::store_add_checked;
use bon::Builder;
use nalgebra::Vector3;

#[derive(Debug, Clone, Builder)]
pub struct Material {
    pub name: String,
    #[builder(default = Vector3::new(0.7, 0.7, 0.7))]
    pub color: Vector3<f32>,
    pub diffuse_texture: Option<HTexture>,
    pub normal_texture: Option<HTexture>,
    pub shininess_texture: Option<HTexture>,
    #[builder(default = 0.0)]
    pub shininess: f32,
    #[builder(default = 1.0)]
    pub opacity: f32,
    #[builder(default = HShader::DIM3)]
    pub shader: HShader,
}

impl<S: material_builder::State> MaterialBuilder<S>
where
    S: material_builder::IsComplete,
{
    pub fn store<A: AsRef<Store<Material>>>(self, store: &A) -> HMaterial {
        store.as_ref().add(self.build())
    }
}

impl StoreDefaults for Material {
    fn populate(store: &mut Store<Self>) {
        let fallback = Material {
            name: "Fallback Material".to_string(),
            color: Vector3::new(1.0, 1.0, 1.0),
            diffuse_texture: None,
            normal_texture: None,
            shininess_texture: None,
            shininess: 0.0,
            shader: HShader::FALLBACK,
            opacity: 1.0,
        };

        store_add_checked!(store, HMaterial::FALLBACK_ID, fallback);

        let default = Material {
            name: "Default Material".to_string(),
            color: Vector3::new(0.7, 0.7, 0.7),
            diffuse_texture: None,
            normal_texture: None,
            shininess_texture: None,
            shininess: 0.3,
            shader: HShader::DIM3,
            opacity: 1.0,
        };

        store_add_checked!(store, HMaterial::DEFAULT_ID, default);
    }
}

impl HMaterial {
    const FALLBACK_ID: u32 = 0;
    const DEFAULT_ID: u32 = 1;
    const MAX_BUILTIN_ID: u32 = 1;

    pub const FALLBACK: HMaterial = HMaterial::new(Self::FALLBACK_ID);
    pub const DEFAULT: HMaterial = HMaterial::new(Self::DEFAULT_ID);
}

impl StoreType for Material {
    fn name() -> &'static str {
        "Material"
    }

    fn ident_fmt(handle: H<Self>) -> HandleName<Self> {
        match handle.id() {
            HMaterial::FALLBACK_ID => HandleName::Static("Fallback Material"),
            HMaterial::DEFAULT_ID => HandleName::Static("Default Material"),
            _ => HandleName::Id(handle),
        }
    }

    fn is_builtin(handle: H<Self>) -> bool {
        handle.id() <= H::<Self>::MAX_BUILTIN_ID
    }
}

impl StoreTypeFallback for Material {
    fn fallback() -> H<Self> {
        HMaterial::FALLBACK
    }
}
