use scalia_core::groups::{GroupId, GroupSpec};
use scalia_core::position::{Pos, RegionId};
use scalia_core::stats::soldier_stats;
use scalia_core::units::{Composition, Health, Needs};
use scalia_core::world::World;

fn world_with(n: u32) -> (World, GroupId) {
    let mut w = World::new(11);
    let pos = Pos { region: RegionId(0), x: 0, y: 0, z: 0 };
    let spec = GroupSpec { pos, composition: Composition([n, 0, 0, 0]), needs: Needs::full() };
    let id = w.groups.spawn(spec, &mut w.roster);
    (w, id)
}

#[test]
fn stats_deterministic() {
    let (w, id) = world_with(10);
    assert_eq!(soldier_stats(&w, id, 3), soldier_stats(&w, id, 3));
}

#[test]
fn skills_in_range() {
    let (w, id) = world_with(150);
    for slot in 0..150 {
        let s = soldier_stats(&w, id, slot).unwrap();
        for v in [s.accuracy, s.speed, s.composure] {
            assert!((40..=80).contains(&v), "slot {slot} skill {v}");
        }
    }
}

#[test]
fn xp_adds_exactly_twenty() {
    let (mut w, id) = world_with(150);
    let before: Vec<_> = (0..150).map(|s| soldier_stats(&w, id, s).unwrap()).collect();
    let (start, _) = w.groups.roster_range(id);
    for slot in 0..150 {
        w.roster.rec_mut(start, slot).xp = 1000;
    }
    for slot in 0..150 {
        let (a, b) = (&before[slot as usize], soldier_stats(&w, id, slot).unwrap());
        assert_eq!(b.accuracy, a.accuracy + 20);
        assert_eq!(b.speed, a.speed + 20);
        assert_eq!(b.composure, a.composure + 20);
        assert_eq!(b.name_seed, a.name_seed);
    }
}

#[test]
fn out_of_range_is_none() {
    let (w, id) = world_with(10);
    assert!(soldier_stats(&w, id, 10).is_none());
    assert!(soldier_stats(&w, id, 9).is_some());
}

#[test]
fn dead_soldier_still_has_stats() {
    let (mut w, id) = world_with(10);
    let (start, _) = w.groups.roster_range(id);
    w.roster.rec_mut(start, 4).health = Health::Dead;
    let s = soldier_stats(&w, id, 4).unwrap();
    assert_eq!(s.health, Health::Dead);
}
