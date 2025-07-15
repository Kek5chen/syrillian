use rapier3d::geometry::SharedShape;
use crate::components::{Collider3D, FPCameraController, FPPlayerController, RigidBodyComponent};
use crate::core::GameObjectId;
use crate::engine::prefabs::prefab::Prefab;
use crate::World;

pub struct FirstPersonPlayerPrefab;

impl Prefab for FirstPersonPlayerPrefab {
    fn prefab_name(&self) -> &'static str {
        "First Person Player"
    }

    fn build(&self, world: &mut World) -> GameObjectId {
        // Prepare camera
        let mut camera = world.new_camera();
        camera.add_component::<FPCameraController>();
        camera.transform.set_position(0.0, 1., 0.0);

        // Prepare character controller
        let mut char_controller = world.new_object(self.prefab_name());
        char_controller
            .transform
            .set_position(0.0, 0.0, 0.0);

        char_controller.add_component::<Collider3D>()
            .get_collider_mut()
            .unwrap()
            .set_shape(SharedShape::capsule_y(1.0, 0.25));

        char_controller.add_component::<RigidBodyComponent>();
        char_controller.add_component::<FPPlayerController>();

        char_controller.add_child(camera);

        world.active_camera = Some(camera);

        char_controller
    }
}
