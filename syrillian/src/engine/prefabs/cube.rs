use crate::assets::{HMaterial, HMesh};
use crate::components::MeshRenderer;
use crate::core::GameObjectId;
use crate::prefabs::prefab::Prefab;
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
        let mut cube = world.new_object("Cube");
        cube.add_component::<MeshRenderer>().change_mesh(HMesh::UNIT_CUBE, Some(vec![self.material]));

        cube
    }
}
