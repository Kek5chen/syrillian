use crate::World;
use crate::components::CameraComponent;
use crate::core::GameObjectId;
use crate::prefabs::prefab::Prefab;

pub struct CameraPrefab;

impl Prefab for CameraPrefab {
    fn prefab_name(&self) -> &'static str {
        "Camera"
    }

    fn build(&self, world: &mut World) -> GameObjectId {
        let mut obj = world.new_object("Camera");

        obj.transform.set_compound_pos_first(true);
        obj.add_component::<CameraComponent>();

        obj
    }
}
