use scalia_core::boarding::{seated_count, ShipLayout};
use scalia_core::derive::*;
use scalia_core::groups::{GroupId, GroupSpec};
use scalia_core::position::{Pos, RegionId};
use scalia_core::roster::FLAG_PERSISTENT;
use scalia_core::time::WorldMs;
use scalia_core::units::{Composition, Health, Needs};
use scalia_core::world::World;
use std::collections::HashSet;

fn pos(x: i64, y: i64) -> Pos {
    Pos { region: RegionId(0), x, y, z: 0 }
}

fn spawn_at(w: &mut World, at: Pos, c: [u32; 4]) -> GroupId {
    let spec = GroupSpec { pos: at, composition: Composition(c), needs: Needs::full() };
    w.groups.spawn(spec, &mut w.roster)
}

fn big_view() -> ViewBox {
    ViewBox { region: RegionId(0), min_x: -1_000_000, max_x: 1_000_000, min_y: -1_000_000, max_y: 1_000_000 }
}

#[test]
fn formation_unique_and_deterministic() {
    let mut seen = HashSet::new();
    for slot in 0..150 {
        let o = formation_offset(9, GroupId(3), slot, 150);
        assert_eq!(o, formation_offset(9, GroupId(3), slot, 150));
        seen.insert(o);
    }
    assert_eq!(seen.len(), 150);
}

#[test]
fn formation_negative_coordinates() {
    let mut w = World::new(5);
    let id = spawn_at(&mut w, pos(-5000, -5000), [150, 0, 0, 0]);
    for slot in [0, 1, 77, 149] {
        let d = derive_soldier(&w, id, slot, WorldMs(0)).unwrap();
        let (dx, dy) = formation_offset(5, id, slot, 150);
        assert_eq!(d.pos, pos(-5000 + dx, -5000 + dy));
        assert_eq!(d.phase, Phase::Standing);
    }
}

#[test]
fn dead_wounded_persistent_are_none() {
    let mut w = World::new(5);
    let id = spawn_at(&mut w, pos(0, 0), [10, 0, 0, 0]);
    let (s, _) = w.groups.roster_range(id);
    w.roster.rec_mut(s, 0).health = Health::Dead;
    w.roster.rec_mut(s, 1).health = Health::Wounded;
    w.roster.rec_mut(s, 2).flags = FLAG_PERSISTENT;
    for slot in 0..3 {
        assert!(derive_soldier(&w, id, slot, WorldMs(0)).is_none(), "slot {slot}");
    }
    for slot in 3..10 {
        assert!(derive_soldier(&w, id, slot, WorldMs(0)).is_some(), "slot {slot}");
    }
    assert!(derive_soldier(&w, id, 10, WorldMs(0)).is_none());
}

#[test]
fn all_dead_group_derives_nobody() {
    let mut w = World::new(5);
    let id = spawn_at(&mut w, pos(0, 0), [20, 0, 0, 0]);
    let (s, n) = w.groups.roster_range(id);
    for slot in 0..n {
        w.roster.rec_mut(s, slot).health = Health::Dead;
    }
    let mut out = vec![];
    derive_visible(&w, WorldMs(0), &big_view(), 1000, &mut out);
    assert!(out.is_empty());
}

#[test]
fn marching_phase() {
    let mut w = World::new(5);
    let id = spawn_at(&mut w, pos(0, 0), [10, 0, 0, 0]);
    w.start_march(id, pos(10_000, 0), 1000, WorldMs(0)).unwrap();
    let mid = derive_soldier(&w, id, 0, WorldMs(5000)).unwrap();
    assert_eq!(mid.phase, Phase::Marching);
    let (dx, dy) = formation_offset(5, id, 0, 10);
    assert_eq!(mid.pos, pos(5000 + dx, dy));
    let end = derive_soldier(&w, id, 0, WorldMs(10_000)).unwrap();
    assert_eq!(end.phase, Phase::Standing);
    assert_eq!(end.pos, pos(10_000 + dx, dy));
}

#[test]
fn boarding_phases_and_seats() {
    let mut w = World::new(5);
    let id = spawn_at(&mut w, pos(0, 0), [150, 0, 0, 0]);
    let ship = pos(100_000, 0);
    let layout = ShipLayout { ramps: 2, doors: 3, corridor_width_cm: 200, seat_cols: 20 };
    let start = WorldMs(10_000);
    w.start_boarding(id, ship, layout, start).unwrap();
    let dur = w.groups.process(id).duration_ms;
    let at = |t: u64| -> Vec<Derived> {
        (0..150).filter_map(|s| derive_soldier(&w, id, s, WorldMs(t))).collect()
    };
    assert!(at(5000).iter().all(|d| d.phase == Phase::Queued));
    let early = derive_soldier(&w, id, 0, WorldMs(11_000)).unwrap();
    assert_eq!(early.phase, Phase::Walking);
    for d in at(start.0 + dur) {
        assert_eq!(d.phase, Phase::Seated);
        let (col, row) = ((d.slot % 20) as i64, (d.slot / 20) as i64);
        assert_eq!(d.pos, Pos { region: RegionId(0), x: 100_000 + col * 600, y: row * 600, z: 2500 });
    }
    for k in 0..40u64 {
        let t = start.0 + k * dur / 39;
        let seated = at(t).iter().filter(|d| d.phase == Phase::Seated).count() as u32;
        let expect = seated_count(5, id, 0..150u32, 625, start, WorldMs(t));
        assert_eq!(seated, expect, "t={t}");
    }
}

#[test]
fn visible_respects_cap_and_region() {
    let mut w = World::new(5);
    for _ in 0..150 {
        spawn_at(&mut w, pos(0, 0), [150, 0, 0, 0]);
    }
    let mut out = vec![];
    let other = ViewBox { region: RegionId(1), ..big_view() };
    derive_visible(&w, WorldMs(0), &other, 20_000, &mut out);
    assert!(out.is_empty());
    derive_visible(&w, WorldMs(0), &big_view(), 20_000, &mut out);
    assert_eq!(out.len(), 20_000);
}

#[test]
fn seated_soldiers_visible_near_ship() {
    let mut w = World::new(5);
    let id = spawn_at(&mut w, pos(0, 0), [30, 0, 0, 0]);
    let layout = ShipLayout { ramps: 2, doors: 3, corridor_width_cm: 200, seat_cols: 10 };
    w.start_boarding(id, pos(500_000, 0), layout, WorldMs(0)).unwrap();
    let t = WorldMs(w.groups.process(id).duration_ms);
    let near_ship = ViewBox { region: RegionId(0), min_x: 450_000, max_x: 550_000, min_y: -50_000, max_y: 50_000 };
    let mut out = vec![];
    derive_visible(&w, t, &near_ship, 1000, &mut out);
    assert_eq!(out.len(), 30);
}

#[test]
fn tiny_groups() {
    let mut w = World::new(5);
    let none = spawn_at(&mut w, pos(0, 0), [0, 0, 4, 1]);
    let one = spawn_at(&mut w, pos(2000, 2000), [1, 0, 0, 0]);
    assert!(derive_soldier(&w, none, 0, WorldMs(0)).is_none());
    let mut out = vec![];
    derive_visible(&w, WorldMs(0), &big_view(), 100, &mut out);
    assert_eq!(out.len(), 1);
    let (dx, dy) = formation_offset(5, one, 0, 1);
    assert_eq!(out[0].pos, pos(2000 + dx, 2000 + dy));
}
