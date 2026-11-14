use scalia_world::{turned, Class, State, Use, World};

#[test]
fn the_testing_ground_loads_and_every_object_kind_is_in_the_catalog() {
    let w = World::testing_ground();
    assert!(w.objects.len() > 150, "{}", w.objects.len());
    assert!(w.area("range").is_some() && w.area("barracks_bay_1").is_some());
    assert_eq!(w.killhouses().len(), 1);
}

#[test]
fn doors_are_objects_with_a_state_and_hold_no_solid_prop() {
    let w = World::testing_ground();
    let doors = w.doors();
    // Seven in the training house, the range door, four in the barracks.
    assert_eq!(doors.len(), 7 + 1 + 4);
    assert!(doors.iter().all(|d| d.lockable));
    let range = doors.iter().find(|d| d.at == (64_000, 27_000)).expect("the range door");
    assert_eq!(range.state, State::Closed);
    assert!(!range.along_x, "in the range's east wall (running along y)");
    assert_eq!(range.width_mm, 1_600);
    for d in &doors {
        assert!(w.prop_of(&w.objects[d.object]).is_none());
        assert_eq!(w.kind(&w.objects[d.object]).class, Class::Door);
    }
}

#[test]
fn a_bad_map_or_catalog_is_refused_with_a_reason() {
    let cat = include_str!("../data/catalog.ron");
    let map = |objects: &str| format!("MapFile(name: \"t\", bounds: (0, 0, 1, 1), objects: [{objects}])");
    let e = World::load(cat, &map("Placed(kind: \"tank\", at: (0, 0))")).err().expect("unknown kind");
    assert!(e.contains("no kind 'tank'"), "{e}");
    let e = World::load(cat, &map("Placed(kind: \"hatch\", at: (0, 0), state: Some(Breached))")).err().expect("a hatch cannot be breached");
    assert!(e.contains("cannot be Breached"), "{e}");
    assert!(World::load("Catalog(kinds: [Kind(id: \"d\", name: \"d\", class: Door, shape: Door(height_mm: 2000))])", &map("")).is_err(), "a door with no states");
}

#[test]
fn spots_turn_with_their_object() {
    assert_eq!(turned((0, 700), 2), (0, -700));
    assert_eq!(turned((1, 0), 1), (0, 1));
    let w = World::testing_ground();
    // A north-row locker (turned half round) is used from the south of it, facing north.
    let (i, north) = w.of_kind("locker").max_by_key(|(_, o)| o.at.1).expect("lockers");
    let s = w.spots().into_iter().find(|s| s.object == i).expect("its spot");
    assert_eq!(s.what, Use::Work);
    assert_eq!((s.at.0, s.at.1, s.face), (north.at.0, north.at.1 - 700, (0, 1)));
    // Every bunk sleeps two (lower and upper).
    for (i, _) in w.of_kind("bunk") {
        let lie: Vec<i64> = w.spots().iter().filter(|s| s.object == i && s.what == Use::Lie).map(|s| s.height_mm).collect();
        assert_eq!(lie, vec![450, 1_450]);
    }
}
