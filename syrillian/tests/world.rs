use syrillian::World;
use web_time::Duration;

#[test]
fn new_object_add_find_delete() {
    let (mut world, _rx1, _rx2) = World::fresh();
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
fn delta_time_advances() {
    let (mut world, _rx1, _rx2) = World::fresh();
    std::thread::sleep(Duration::from_millis(1));
    world.update();
    world.next_frame();
    assert!(world.delta_time() > Duration::ZERO);
}

#[test]
fn strong_refs_keep_objects_alive_until_drop() {
    let (mut world, _rx1, _rx2) = World::fresh();
    let id = world.new_object("KeepAlive");
    let handle = world.get_object_ref(id).expect("object should exist");

    world.delete_object(id);

    assert!(world.get_object(id).is_none(), "deleted objects are hidden");
    assert!(
        world.objects.contains_key(id),
        "object storage is kept alive while strong refs exist"
    );

    drop(handle);
    assert!(
        !world.objects.contains_key(id),
        "object storage is freed once the last reference drops"
    );
}

#[test]
fn weak_refs_upgrade_only_when_alive() {
    let (mut world, _rx1, _rx2) = World::fresh();
    let id = world.new_object("WeakSubject");
    let weak = id.downgrade();

    assert!(weak.upgrade().is_some());
    world.delete_object(id);
    assert!(
        weak.upgrade().is_none(),
        "weak refs should not revive deleted objects"
    );
}

#[test]
fn shutdown_cleans_world_state() {
    let (mut world, _rx1, _rx2) = World::fresh();
    let id = world.new_object("ToDelete");
    world.add_child(id);
    world.shutdown();

    assert!(world.objects.is_empty());
    assert!(world.children.is_empty());
}
