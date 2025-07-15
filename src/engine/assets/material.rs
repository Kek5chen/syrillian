use crate::engine::assets::generic_store::{HandleName, Store, StoreDefaults, StoreType};
use crate::engine::assets::*;
use crate::store_add_checked;
use nalgebra::Vector3;

#[derive(Debug, Clone)]
pub struct Material {
    pub name: String,
    pub diffuse: Vector3<f32>,
    pub diffuse_texture: Option<HTexture>,
    pub normal_texture: Option<HTexture>,
    pub shininess: f32,
    pub shininess_texture: Option<HTexture>,
    pub opacity: f32,
    pub shader: Option<H<Shader>>,
}

impl StoreDefaults for Material {
    fn populate(store: &mut Store<Self>) {
        let fallback = Material {
            name: "Fallback Material".to_string(),
            diffuse: Vector3::new(1.0, 1.0, 1.0),
            diffuse_texture: None,
            normal_texture: None,
            shininess: 0.0,
            shader: Some(HShader::DIM3),
            opacity: 1.0,
            shininess_texture: None,
        };

        store_add_checked!(store, HMaterial::FALLBACK_ID, fallback);
    }
}

impl HMaterial {
    const FALLBACK_ID: u32 = 0;

    pub const FALLBACK: HMaterial = HMaterial::new(Self::FALLBACK_ID);
}

impl StoreType for Material {
    fn name() -> &'static str {
        "Material"
    }

    fn ident_fmt(handle: H<Self>) -> HandleName<Self> {
        match handle.id() {
            HMaterial::FALLBACK_ID => HandleName::Static("Fallback Material"),
            _ => HandleName::Id(handle),
        }
    }
}

impl StoreTypeFallback for Material {
    fn fallback() -> H<Self> {
        HMaterial::FALLBACK
    }
}
