use crate::assets::{HMaterial, HMesh};
use crate::core::GameObjectId;
use crate::drawables::MeshRenderer;
use crate::prefabs::prefab::Prefab;
use crate::World;

pub struct SpherePrefab {
    pub material: HMaterial,
}

impl Default for SpherePrefab {
    fn default() -> Self {
        Self {
            material: HMaterial::DEFAULT,
        }
    }
}

impl SpherePrefab {
    pub const fn new(material: HMaterial) -> Self {
        Self { material }
    }
}

impl Prefab for SpherePrefab {
    #[inline]
    fn prefab_name(&self) -> &'static str {
        "Sphere"
    }

    fn build(&self, world: &mut World) -> GameObjectId {
        let mut sphere = world.new_object(self.prefab_name());
        sphere.drawable = Some(MeshRenderer::new(HMesh::SPHERE, Some(vec![self.material])));

        sphere
    }
}
