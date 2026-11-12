use scalia_core::groups::{GroupId, GroupSpec};
use scalia_core::position::{dist_mm, Pos, RegionId};
use scalia_core::process::{process_pos, ProcessError, ProcessKind};
use scalia_core::time::WorldMs;
use scalia_core::units::{Composition, Needs};
use scalia_core::world::World;

fn at(x: i64, y: i64) -> Pos {
    Pos { region: RegionId(1), x, y, z: 0 }
}

fn one_group() -> (World, GroupId) {
    let mut w = World::new(1);
    let spec = GroupSpec { pos: at(0, 0), composition: Composition([100, 0, 0, 0]), needs: Needs::full() };
    let id = w.groups.spawn(spec, &mut w.roster);
    (w, id)
}

#[test]
fn order_from_rest_is_an_ordinary_march() {
    let (mut w, id) = one_group();
    w.order_march(id, at(10_000, 0), 1_000, WorldMs(0)).unwrap();
    assert!(matches!(w.groups.process(id).kind, ProcessKind::March { .. }));
    assert_eq!(w.groups.process(id).duration_ms, 10_000);
}

#[test]
fn redirect_mid_march_starts_where_the_group_is_now() {
    let (mut w, id) = one_group();
    w.order_march(id, at(10_000, 0), 1_000, WorldMs(0)).unwrap();
    let mid = process_pos(w.groups.process(id), WorldMs(4_000)).unwrap();
    w.order_march(id, at(4_000, 6_000), 1_000, WorldMs(4_000)).unwrap();
    // No teleport: the new march starts exactly at the old position at that moment.
    let p = w.groups.process(id);
    match p.kind {
        ProcessKind::March { from, to, .. } => {
            assert_eq!(from, mid);
            assert_eq!(to, at(4_000, 6_000));
            assert_eq!(p.start, WorldMs(4_000));
            assert_eq!(p.duration_ms, dist_mm(mid, to) * 1000 / 1000);
        }
        other => panic!("expected a march, got {other:?}"),
    }
    assert_eq!(w.groups.pos(id), mid);
}

#[test]
fn orders_obey_the_usual_rules() {
    let (mut w, id) = one_group();
    assert_eq!(w.order_march(id, at(5, 5), 0, WorldMs(0)), Err(ProcessError::ZeroSpeed));
    let elsewhere = Pos { region: RegionId(2), ..at(5, 5) };
    assert_eq!(w.order_march(id, elsewhere, 1_000, WorldMs(0)), Err(ProcessError::RegionMismatch));
    assert_eq!(w.groups.pos(id), at(0, 0));
}

#[test]
fn a_group_in_flight_cannot_be_ordered_to_march() {
    use scalia_core::system::{BodyId, System, PARK_OFFSET, SYSTEM_REGION};
    let s = System::demo();
    let mut w = World::new(1);
    let start = s.body_pos(BodyId(0), WorldMs(0)).unwrap().offset(PARK_OFFSET.0, PARK_OFFSET.1, PARK_OFFSET.2);
    let spec = GroupSpec { pos: start, composition: Composition([10, 0, 0, 0]), needs: Needs::full() };
    let id = w.groups.spawn(spec, &mut w.roster);
    w.start_flight(id, &s, BodyId(1), PARK_OFFSET, 2_000_000_000, WorldMs(0)).unwrap();
    let to = Pos { region: SYSTEM_REGION, x: 0, y: 0, z: 0 };
    assert_eq!(w.order_march(id, to, 1_000, WorldMs(10)), Err(ProcessError::Busy));
}
