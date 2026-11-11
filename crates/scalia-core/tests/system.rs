use scalia_core::fixed::Fix;
use scalia_core::hash::mix64;
use scalia_core::position::{dist_mm, Pos, RegionId};
use scalia_core::system::{Body, BodyId, System, SYSTEM_REGION};
use scalia_core::time::WorldMs;

const A: BodyId = BodyId(0);
const B: BodyId = BodyId(1);
const RA: i64 = 40_000_000_000;
const RB: i64 = 90_000_000_000;

fn near(p: Pos, x: i64, y: i64, tol: i64) -> bool {
    p.region == SYSTEM_REGION && (p.x - x).abs() <= tol && (p.y - y).abs() <= tol
}

#[test]
fn start_position_is_on_the_phase_axis() {
    let s = System::demo();
    assert!(near(s.body_pos(A, WorldMs(0)).unwrap(), RA, 0, 4));
    assert!(near(s.body_pos(B, WorldMs(0)).unwrap(), 0, RB, 4 + RB / 1_000_000_000));
}

#[test]
fn distance_from_star_equals_radius() {
    let s = System::demo();
    for (id, r) in [(A, RA), (B, RB)] {
        for i in 0..1000u64 {
            let p = s.body_pos(id, WorldMs(mix64(i) % 10_000_000)).unwrap();
            let d = dist_mm(p, s.star) as i64;
            assert!((d - r).abs() <= r / 1_000_000, "d {d} r {r}");
        }
    }
}

#[test]
fn returns_after_one_period() {
    let s = System::demo();
    for i in 0..200u64 {
        let t = mix64(i) % 5_000_000;
        let a = s.body_pos(A, WorldMs(t)).unwrap();
        let b = s.body_pos(A, WorldMs(t + 600_000)).unwrap();
        assert!(dist_mm(a, b) <= 4_000, "{}", dist_mm(a, b));
    }
}

#[test]
fn quarter_period_is_quarter_turn() {
    let s = System::demo();
    assert!(near(s.body_pos(A, WorldMs(150_000)).unwrap(), 0, RA, 20_000));
}

#[test]
fn unknown_body_is_none() {
    assert!(System::demo().body_pos(BodyId(9), WorldMs(0)).is_none());
}

#[test]
fn period_zero_stays_at_phase() {
    let s = System {
        star: Pos { region: SYSTEM_REGION, x: 0, y: 0, z: 0 },
        bodies: vec![Body { name: "Still", orbit_radius_mm: 1000, period_ms: 0, phase: Fix::ZERO }],
    };
    let p0 = s.body_pos(BodyId(0), WorldMs(0)).unwrap();
    assert_eq!(p0, s.body_pos(BodyId(0), WorldMs(123_456_789)).unwrap());
}

#[test]
fn huge_time_does_not_panic() {
    let s = System::demo();
    assert!(s.body_pos(A, WorldMs(u64::MAX)).is_some());
    assert!(s.body_pos(B, WorldMs(u64::MAX - 1)).is_some());
}

#[test]
fn golden_system_hash() {
    let s = System::demo();
    let mut h = 0u64;
    for i in 0..1000u64 {
        for id in [A, B] {
            let p = s.body_pos(id, WorldMs(i * 977)).unwrap();
            h = mix64(h ^ p.x as u64);
            h = mix64(h ^ p.y as u64);
        }
    }
    let _ = RegionId(0);
    assert_eq!(h, 3716267243279158290);
}
