use scalia_core::boarding::ShipLayout;
use scalia_core::groups::{GroupId, GroupSpec, Groups, SpawnError, MAX_GROUP_PEOPLE};
use scalia_core::position::{Pos, RegionId};
use scalia_core::process::ProcessError;
use scalia_core::roster::Roster;
use scalia_core::time::WorldMs;
use scalia_core::units::{Composition, Needs};
use scalia_core::world::World;

fn pos(x: i64) -> Pos {
    Pos { region: RegionId(0), x, y: 0, z: 0 }
}

fn spawn(w: &mut World, c: [u32; 4]) -> GroupId {
    let spec = GroupSpec { pos: pos(0), composition: Composition(c), needs: Needs::full() };
    w.groups.spawn(spec, &mut w.roster)
}

const LAYOUT: ShipLayout = ShipLayout { ramps: 1, doors: 1, corridor_width_cm: 100, seat_cols: 10 };

#[test]
fn second_march_while_marching_is_busy() {
    let mut w = World::new(1);
    let id = spawn(&mut w, [10, 0, 0, 0]);
    w.start_march(id, pos(5000), 1000, WorldMs(0)).unwrap();
    let before = *w.groups.process(id);
    assert_eq!(w.start_march(id, pos(9000), 1000, WorldMs(1000)), Err(ProcessError::Busy));
    assert_eq!(*w.groups.process(id), before);
}

#[test]
fn boarding_while_marching_is_busy_and_group_unchanged() {
    let mut w = World::new(1);
    let id = spawn(&mut w, [10, 0, 0, 0]);
    w.start_march(id, pos(5000), 1000, WorldMs(0)).unwrap();
    let before = *w.groups.process(id);
    assert_eq!(w.start_boarding(id, pos(0), LAYOUT, WorldMs(1000)), Err(ProcessError::Busy));
    assert_eq!(*w.groups.process(id), before);
}

#[test]
fn idle_group_can_start_again_after_march_completes() {
    let mut w = World::new(1);
    let id = spawn(&mut w, [10, 0, 0, 0]);
    w.start_march(id, pos(5000), 1000, WorldMs(0)).unwrap(); // 5 s
    w.tick(WorldMs(6000)).unwrap();
    assert!(w.start_march(id, pos(9000), 1000, WorldMs(6000)).is_ok());
}

#[test]
fn restart_at_or_after_the_scheduled_end_is_allowed() {
    let mut w = World::new(1);
    let id = spawn(&mut w, [10, 0, 0, 0]);
    w.start_boarding(id, pos(0), LAYOUT, WorldMs(0)).unwrap();
    let end = w.groups.process(id).duration_ms;
    assert_eq!(w.start_march(id, pos(100), 1000, WorldMs(end - 1)), Err(ProcessError::Busy));
    assert!(w.start_march(id, pos(100), 1000, WorldMs(end)).is_ok());
}

#[test]
fn try_spawn_rejects_absurd_group() {
    let mut groups = Groups::new();
    let mut roster = Roster::new();
    let spec = GroupSpec { pos: pos(0), composition: Composition([u32::MAX, 0, 0, 0]), needs: Needs::full() };
    assert_eq!(groups.try_spawn(spec, &mut roster).unwrap_err(), SpawnError::TooLarge);
    assert_eq!(groups.len(), 0);
    assert_eq!(roster.len(), 0);
}

#[test]
fn try_spawn_accepts_one_million() {
    let mut groups = Groups::new();
    let mut roster = Roster::new();
    let spec = GroupSpec { pos: pos(0), composition: Composition([1_000_000, 0, 0, 0]), needs: Needs::full() };
    assert!(groups.try_spawn(spec, &mut roster).is_ok());
    assert_eq!(roster.len(), 1_000_000);
    assert!(MAX_GROUP_PEOPLE >= 1_000_000);
}
