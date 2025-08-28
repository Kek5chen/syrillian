use crate::World;
use crate::core::GameObjectId;

pub trait Prefab {
    fn prefab_name(&self) -> &'static str;
    fn build(&self, world: &mut World) -> GameObjectId;
    fn spawn(&self, world: &mut World) -> GameObjectId {
        let obj = self.build(world);
        world.add_child(obj);
        obj
    }
}
