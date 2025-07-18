use crate::assets::{HMaterial, HMesh, Mesh};
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
        // TODO: Move materials out of mesh and into the MeshRenderer
        let sphere = world.assets.meshes.try_get(HMesh::SPHERE).expect("Sphere should exist").data.clone();
        let mesh = world.assets.meshes.add(
            Mesh::builder(sphere.vertices)
                .with_one_texture(self.material)
                .build(),
        );

        let mut cube = world.new_object(self.prefab_name());
        cube.drawable = Some(MeshRenderer::new(mesh));

        cube
    }
}
