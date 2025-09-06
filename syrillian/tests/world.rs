use serial_test::serial;
use syrillian::World;
use web_time::Duration;

#[test]
#[serial]
fn new_object_add_find_delete() {
    let (mut world, _rx1, _rx2) = unsafe { World::fresh() };
    let id = world.new_object("TestObject");
    world.add_child(id);
    assert!(world.find_object_by_name("TestObject").is_some());
    assert_eq!(world.children.len(), 1);
    assert!(world.get_object(id).is_some());
    world.delete_object(id);
    assert!(world.get_object(id).is_none());
    assert_eq!(world.children.len(), 0);
}

#[test]
#[serial]
fn delta_time_advances() {
    let (mut world, _rx1, _rx2) = unsafe { World::fresh() };
    std::thread::sleep(Duration::from_millis(1));
    world.update();
    world.next_frame();
    assert!(world.delta_time() > Duration::ZERO);
}
