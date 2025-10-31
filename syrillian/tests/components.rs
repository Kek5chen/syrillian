use nalgebra::Vector3;
use serial_test::serial;
use std::any::TypeId;
use syrillian::World;
use syrillian::components::{Component, NewComponent};
use syrillian::core::GameObjectId;

struct MyComponent {
    parent: GameObjectId,
}

impl NewComponent for MyComponent {
    fn new(parent: GameObjectId) -> Self {
        Self { parent }
    }
}

impl Component for MyComponent {
    fn init(&mut self, _world: &mut World) {
        self.parent.transform.translate(Vector3::new(1.0, 0.0, 0.0));
    }
}

#[test]
#[serial]
fn component() {
    let (mut world, _rx1, _rx2) = unsafe { World::fresh() };
    let mut obj = world.new_object("Test");

    let comp = obj.add_component::<MyComponent>();
    assert_eq!(obj.transform.position(), Vector3::new(1.0, 0.0, 0.0));

    let comp2 = obj.add_component::<MyComponent>();
    assert_eq!(obj.transform.position(), Vector3::new(2.0, 0.0, 0.0));

    assert_eq!(comp.parent(), obj);
    assert_eq!(comp2.parent(), obj);

    assert_eq!(world.components.values().count(), 2);
    assert_eq!(
        world
            .components
            .values_of_type::<MyComponent>()
            .unwrap()
            .count(),
        2
    );

    obj.remove_component(&comp2, &mut world);
    assert_eq!(obj.get_components::<MyComponent>().count(), 1);
    assert_eq!(world.components.values().count(), 1);
    assert_eq!(
        world
            .components
            .values_of_type::<MyComponent>()
            .unwrap()
            .count(),
        1
    );

    let comp2 = comp2.downgrade();
    assert_eq!(comp2.upgrade(&world), None);

    obj.delete();
    let comp = comp.downgrade();
    assert_eq!(comp.upgrade(&world), None);
}

#[test]
#[serial]
fn check_typed() {
    let (mut world, _rx1, _rx2) = unsafe { World::fresh() };
    let mut obj = world.new_object("Test");

    let comp = obj.add_component::<MyComponent>();
    let typed = comp.typed_id();

    assert_eq!(typed.type_id(), TypeId::of::<MyComponent>());

    obj.remove_component(comp, &mut world);

    assert_eq!(world.components.values().count(), 0);
}
