use scalia_blueprint::{
    layout::ship_layout,
    mesh::{mesh, mesh_hash},
    shipped::shipped,
    triangle_budget, triangle_count,
};
use scalia_core::boarding::{interval_ms, seated_count};
use scalia_core::groups::GroupSpec;
use scalia_core::position::{Pos, RegionId};
use scalia_core::time::WorldMs;
use scalia_core::units::{Composition, Needs};
use scalia_core::world::World;

#[test]
fn six_blueprints_load_and_validate() {
    assert_eq!(shipped().expect("all shipped blueprints are valid").len(), 6);
}

#[test]
fn ids_are_unique() {
    let mut ids: Vec<_> = shipped().unwrap().into_iter().map(|b| b.id).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 6);
}

#[test]
fn each_blueprint_under_its_triangle_budget() {
    for b in shipped().unwrap() {
        assert!(triangle_count(&b) <= triangle_budget(b.class), "{}", b.id);
    }
}

#[test]
fn golden_mesh_hashes() {
    let want: [(&str, u64); 6] = [
        ("trooper_light", 175910638632774405),
        ("trooper_medium", 1794808734767731072),
        ("trooper_heavy", 14658804970292668338),
        ("tank_bulwark", 9438481043952797810),
        ("truck_hauler", 8688044065886655461),
        ("transport_ox", 10854958835410261074),
    ];
    let got: Vec<(String, u64)> = shipped().unwrap().iter().map(|b| (b.id.clone(), mesh_hash(&mesh(b)))).collect();
    for (id, h) in &got {
        println!("GOLDEN {id} {h}");
    }
    for (i, (id, h)) in got.iter().enumerate() {
        assert_eq!(id, want[i].0);
        assert_eq!(*h, want[i].1, "mesh hash of {id} changed");
    }
}

#[test]
fn transport_boards_a_group_through_core() {
    let ox = shipped().unwrap().into_iter().find(|b| b.id == "transport_ox").unwrap();
    let layout = ship_layout(&ox).expect("the Ox has a layout");
    assert!(layout.throughput_milli_per_s().unwrap() > 0);
    let mut w = World::new(7);
    let pos = Pos { region: RegionId(1), x: 0, y: 0, z: 0 };
    let spec = GroupSpec { pos, composition: Composition([500, 0, 0, 0]), needs: Needs::full() };
    let id = w.groups.spawn(spec, &mut w.roster);
    w.start_boarding(id, pos, layout, WorldMs(0)).unwrap();
    let interval = interval_ms(layout.throughput_milli_per_s().unwrap());
    let all = seated_count(w.seed, id, 0..500, interval, WorldMs(0), WorldMs(10_000_000));
    assert_eq!(all, 500);
    let none = seated_count(w.seed, id, 0..500, interval, WorldMs(0), WorldMs(0));
    assert_eq!(none, 0);
}
