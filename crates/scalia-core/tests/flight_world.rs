use scalia_core::derive::{derive_visible, ViewBox};
use scalia_core::flight::{plan_flight, DEMO_PEAK_SPEED_MM_S};
use scalia_core::groups::{GroupId, GroupSpec};
use scalia_core::position::{Pos, RegionId};
use scalia_core::process::{process_pos, ProcessError};
use scalia_core::state_hash::state_hash;
use scalia_core::system::{BodyId, System, PARK_OFFSET};
use scalia_core::time::WorldMs;
use scalia_core::units::{Composition, Needs};
use scalia_core::world::World;

const A: BodyId = BodyId(0);
const B: BodyId = BodyId(1);

fn parked(s: &System, id: BodyId, t: u64) -> Pos {
    s.body_pos(id, WorldMs(t))
        .unwrap()
        .offset(PARK_OFFSET.0, PARK_OFFSET.1, PARK_OFFSET.2)
}

fn world_with_group(at: Pos) -> (World, GroupId) {
    let mut w = World::new(3);
    let spec = GroupSpec {
        pos: at,
        composition: Composition([150, 0, 0, 0]),
        needs: Needs::full(),
    };
    let id = w.groups.spawn(spec, &mut w.roster);
    (w, id)
}

fn view_around(p: Pos) -> ViewBox {
    ViewBox {
        region: p.region,
        min_x: p.x - 100_000,
        max_x: p.x + 100_000,
        min_y: p.y - 100_000,
        max_y: p.y + 100_000,
    }
}

#[test]
fn flight_moves_the_group_and_lands_it() {
    let s = System::demo();
    let start = parked(&s, A, 0);
    let (mut w, id) = world_with_group(start);
    w.start_flight(id, &s, B, PARK_OFFSET, DEMO_PEAK_SPEED_MM_S, WorldMs(0))
        .unwrap();
    let plan = plan_flight(&s, start, B, PARK_OFFSET, DEMO_PEAK_SPEED_MM_S, WorldMs(0)).unwrap();
    let end = plan.duration_ms;
    w.tick(WorldMs(end - 1)).unwrap();
    assert_ne!(
        process_pos(w.groups.process(id), WorldMs(end - 1)).unwrap(),
        plan.to
    );
    assert_eq!(w.groups.pos(id), start);
    w.tick(WorldMs(end)).unwrap();
    assert_eq!(w.groups.pos(id), plan.to);
    assert!(!w.groups.process(id).is_running_at(WorldMs(end)));
}

#[test]
fn second_flight_while_flying_is_busy() {
    let s = System::demo();
    let (mut w, id) = world_with_group(parked(&s, A, 0));
    w.start_flight(id, &s, B, PARK_OFFSET, DEMO_PEAK_SPEED_MM_S, WorldMs(0))
        .unwrap();
    let before = *w.groups.process(id);
    assert_eq!(
        w.start_flight(id, &s, A, PARK_OFFSET, DEMO_PEAK_SPEED_MM_S, WorldMs(10)),
        Err(ProcessError::Busy)
    );
    assert_eq!(*w.groups.process(id), before);
}

#[test]
fn errors_change_nothing() {
    let s = System::demo();
    let ground = Pos {
        region: RegionId(0),
        x: 0,
        y: 0,
        z: 0,
    };
    let (mut w, id) = world_with_group(ground);
    let before = *w.groups.process(id);
    assert_eq!(
        w.start_flight(id, &s, B, PARK_OFFSET, DEMO_PEAK_SPEED_MM_S, WorldMs(0)),
        Err(ProcessError::RegionMismatch)
    );
    assert_eq!(*w.groups.process(id), before);
    let (mut w, id) = world_with_group(parked(&s, A, 0));
    assert_eq!(
        w.start_flight(
            id,
            &s,
            BodyId(9),
            PARK_OFFSET,
            DEMO_PEAK_SPEED_MM_S,
            WorldMs(0)
        ),
        Err(ProcessError::UnknownBody)
    );
    assert_eq!(
        w.start_flight(id, &s, B, PARK_OFFSET, 0, WorldMs(0)),
        Err(ProcessError::ZeroSpeed)
    );
    assert_eq!(*w.groups.process(id), before);
}

#[test]
fn soldiers_aboard_a_flying_group_are_not_derived() {
    let s = System::demo();
    let start = parked(&s, A, 0);
    let (mut w, id) = world_with_group(start);
    let mut out = Vec::new();
    derive_visible(&w, WorldMs(0), &view_around(start), 1000, &mut out);
    assert!(out.len() > 0);
    w.start_flight(id, &s, B, PARK_OFFSET, DEMO_PEAK_SPEED_MM_S, WorldMs(0))
        .unwrap();
    derive_visible(&w, WorldMs(0), &view_around(start), 1000, &mut out);
    assert_eq!(out.len(), 0);
}

#[test]
fn state_hash_differs_in_flight_and_is_repeatable() {
    let s = System::demo();
    let make = || world_with_group(parked(&s, A, 0));
    let (mut a, ia) = make();
    let (b, _) = make();
    assert_eq!(state_hash(&a), state_hash(&b));
    a.start_flight(ia, &s, B, PARK_OFFSET, DEMO_PEAK_SPEED_MM_S, WorldMs(0))
        .unwrap();
    assert_ne!(state_hash(&a), state_hash(&b));
}
