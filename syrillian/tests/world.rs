use serial_test::serial;
use std::time::Duration;
use syrillian::World;

#[test]
#[serial]
fn new_object_add_find_delete() {
    let mut world = unsafe { World::new() };
    let id = world.new_object("TestObject");
    world.add_child(id);
    assert!(world.find_object_by_name("TestObject").is_some());
    assert_eq!(world.children.len(), 1);
    assert!(world.get_object(&id).is_some());
    world.delete_object(id);
    assert!(world.get_object(&id).is_none());
}

#[test]
#[serial]
fn delta_time_advances() {
    let mut world = unsafe { World::new() };
    std::thread::sleep(Duration::from_millis(1));
    world.update();
    assert!(world.delta_time() > Duration::ZERO);
}
