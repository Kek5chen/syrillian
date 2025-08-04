use crate::assets::{HMaterial, Mesh};
use crate::core::GameObjectId;
use crate::drawables::MeshRenderer;
use crate::prefabs::prefab::Prefab;
use crate::utils::{CUBE_IDX, CUBE_VERT};
use crate::World;

pub struct CubePrefab {
    pub material: HMaterial,
}

impl Default for CubePrefab {
    fn default() -> Self {
        CubePrefab {
            material: HMaterial::DEFAULT,
        }
    }
}

impl CubePrefab {
    pub const fn new(material: HMaterial) -> Self {
        CubePrefab { material }
    }
}

impl Prefab for CubePrefab {
    #[inline]
    fn prefab_name(&self) -> &'static str {
        "Cube"
    }

    fn build(&self, world: &mut World) -> GameObjectId {
        // TODO: Move materials out of mesh and into the MeshRenderer
        let mesh = world.assets.meshes.add(
            Mesh::builder(CUBE_VERT.to_vec())
                .with_indices(CUBE_IDX.to_vec())
                .with_one_texture(self.material)
                .build(),
        );

        let mut cube = world.new_object("Cube");
        cube.drawable = Some(MeshRenderer::new(mesh));

        cube
    }
}
