use crate::components::{
    Collider3D, FirstPersonCameraController, FirstPersonMovementController, RigidBodyComponent,
};
use crate::core::GameObjectId;
use crate::engine::prefabs::prefab::Prefab;
use crate::World;
use log::warn;
use rapier3d::geometry::SharedShape;

pub struct FirstPersonPlayerPrefab;

impl Prefab for FirstPersonPlayerPrefab {
    fn prefab_name(&self) -> &'static str {
        "First Person Player"
    }

    fn build(&self, world: &mut World) -> GameObjectId {
        // Prepare camera
        let mut camera = world.new_camera();
        camera.transform.set_position(0.0, 1.0, 0.0);
        camera.add_component::<FirstPersonCameraController>();

        // Prepare character controller
        let mut char_controller = world.new_object(self.prefab_name());
        char_controller.transform.set_position(0.0, 0.0, 0.0);

        char_controller
            .add_component::<Collider3D>()
            .get_collider_mut()
            .unwrap()
            .set_shape(SharedShape::capsule_y(1.0, 0.25));

        if let Some(rigid_body) = char_controller
            .add_component::<RigidBodyComponent>()
            .get_body_mut()
        {
            rigid_body.set_additional_mass(5., false);
        } else {
            warn!("Not able to set rigid body properties for First Person Player Prefab");
        }

        char_controller.add_child(camera);
        char_controller.add_component::<FirstPersonMovementController>();

        world.active_camera = Some(camera);

        char_controller
    }
}
