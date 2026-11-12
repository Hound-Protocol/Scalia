use scalia_core::derive::{derive_vehicles, DerivedVehicle};
use scalia_core::groups::{GroupId, GroupSpec};
use scalia_core::position::{Pos, RegionId};
use scalia_core::time::WorldMs;
use scalia_core::units::{Composition, Needs, UnitType};
use scalia_core::world::World;

fn at(x: i64, y: i64) -> Pos {
    Pos { region: RegionId(1), x, y, z: 0 }
}

fn group(c: [u32; 4]) -> (World, GroupId) {
    let mut w = World::new(5);
    let spec = GroupSpec { pos: at(0, 0), composition: Composition(c), needs: Needs::full() };
    let id = w.groups.spawn(spec, &mut w.roster);
    (w, id)
}

fn vehicles(w: &World, id: GroupId, t: u64) -> Vec<DerivedVehicle> {
    let mut out = Vec::new();
    derive_vehicles(w, id, WorldMs(t), &mut out);
    out
}

#[test]
fn one_per_vehicle_tanks_first() {
    let (w, id) = group([100, 0, 3, 2]);
    let v = vehicles(&w, id, 0);
    assert_eq!(v.len(), 5);
    assert_eq!(v.iter().filter(|d| d.kind == UnitType::Tank).count(), 3);
    assert_eq!(v.iter().map(|d| d.kind).collect::<Vec<_>>(), [UnitType::Tank, UnitType::Tank, UnitType::Tank, UnitType::Truck, UnitType::Truck]);
    assert!(vehicles(&group([100, 5, 0, 0]).0, id, 0).is_empty());
}

#[test]
fn idle_vehicles_park_behind_the_formation_in_two_columns() {
    let (w, id) = group([100, 0, 2, 2]);
    let v = vehicles(&w, id, 0);
    // 100 soldiers: 10 rows at 0.9 m, so the formation's back edge is about 4.5 m behind the centre.
    assert!(v.iter().all(|d| d.pos.y < -4_500), "{v:?}");
    let tank_x = v.iter().filter(|d| d.kind == UnitType::Tank).map(|d| d.pos.x).collect::<Vec<_>>();
    let truck_x = v.iter().filter(|d| d.kind == UnitType::Truck).map(|d| d.pos.x).collect::<Vec<_>>();
    assert!(tank_x.iter().all(|&x| x < 0) && truck_x.iter().all(|&x| x > 0), "{tank_x:?} {truck_x:?}");
    assert!(v.iter().all(|d| d.facing == (0, 1)));
}

#[test]
fn marching_vehicles_follow_behind_and_face_the_march() {
    let (mut w, id) = group([100, 0, 1, 1]);
    w.order_march(id, at(100_000, 0), 1_000, WorldMs(0)).unwrap();
    let a = vehicles(&w, id, 10_000);
    let b = vehicles(&w, id, 20_000);
    for (va, vb) in a.iter().zip(&b) {
        assert!(va.pos.x < 10_000 - 4_500, "behind (west of) the formation centre: {va:?}");
        assert_eq!(vb.pos.x - va.pos.x, 10_000, "moves with the group");
        assert!(va.facing.0 > 0 && va.facing.1 == 0);
    }
}

#[test]
fn deterministic() {
    let (w, id) = group([300, 10, 4, 4]);
    assert_eq!(vehicles(&w, id, 1234), vehicles(&w, id, 1234));
}
