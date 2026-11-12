use scalia_core::fixed::Fix;
use scalia_core::flight::{ease, flight_pos, plan_flight, DEMO_PEAK_SPEED_MM_S};
use scalia_core::hash::mix64;
use scalia_core::position::{dist_mm, Pos, RegionId};
use scalia_core::process::ProcessError;
use scalia_core::system::{BodyId, System, PARK_OFFSET};
use scalia_core::time::WorldMs;

const A: BodyId = BodyId(0);
const B: BodyId = BodyId(1);

fn parked(s: &System, id: BodyId, t: u64) -> Pos {
    s.body_pos(id, WorldMs(t))
        .unwrap()
        .offset(PARK_OFFSET.0, PARK_OFFSET.1, PARK_OFFSET.2)
}

#[test]
fn ease_endpoints_and_midpoint() {
    assert_eq!(ease(0, 100), Fix::ZERO);
    assert_eq!(ease(100, 100), Fix::ONE);
    assert!((ease(50, 100).0 - Fix::ONE.0 / 2).abs() <= 1);
    assert_eq!(ease(500, 100), Fix::ONE);
    assert_eq!(ease(5, 0), Fix::ONE);
}

#[test]
fn ease_is_monotone() {
    let mut last = Fix::ZERO;
    for i in 0..=1000 {
        let e = ease(i, 1000);
        assert!(e >= last);
        last = e;
    }
}

#[test]
fn arrival_is_where_the_body_will_be() {
    let s = System::demo();
    for start in [0u64, 1_000, 77_777, 400_000, 1_234_567] {
        let from = parked(&s, A, start);
        let p = plan_flight(
            &s,
            from,
            B,
            PARK_OFFSET,
            DEMO_PEAK_SPEED_MM_S,
            WorldMs(start),
        )
        .unwrap();
        assert_eq!(p.to, parked(&s, B, start + p.duration_ms));
        assert_eq!(p.from, from);
    }
}

#[test]
fn peak_speed_is_close_to_requested() {
    let s = System::demo();
    let from = parked(&s, A, 0);
    let p = plan_flight(&s, from, B, PARK_OFFSET, DEMO_PEAK_SPEED_MM_S, WorldMs(0)).unwrap();
    let v = dist_mm(p.from, p.to) as f64 * 1875.0 / p.duration_ms as f64;
    let want = DEMO_PEAK_SPEED_MM_S as f64;
    assert!((v - want).abs() / want < 0.01, "peak {v}");
}

#[test]
fn starts_and_ends_exactly() {
    let s = System::demo();
    let p = plan_flight(
        &s,
        parked(&s, A, 5000),
        B,
        PARK_OFFSET,
        DEMO_PEAK_SPEED_MM_S,
        WorldMs(5000),
    )
    .unwrap();
    assert_eq!(flight_pos(&p, WorldMs(0)), p.from);
    assert_eq!(flight_pos(&p, WorldMs(5000)), p.from);
    assert_eq!(flight_pos(&p, WorldMs(5000 + p.duration_ms)), p.to);
    assert_eq!(flight_pos(&p, WorldMs(u64::MAX)), p.to);
}

#[test]
fn speed_is_zero_at_both_ends() {
    let s = System::demo();
    let p = plan_flight(
        &s,
        parked(&s, A, 0),
        B,
        PARK_OFFSET,
        DEMO_PEAK_SPEED_MM_S,
        WorldMs(0),
    )
    .unwrap();
    let d = dist_mm(p.from, p.to);
    assert!(dist_mm(p.from, flight_pos(&p, WorldMs(1))) <= d / 100_000);
    assert!(dist_mm(p.to, flight_pos(&p, WorldMs(p.duration_ms - 1))) <= d / 100_000);
}

#[test]
fn progress_is_monotone() {
    let s = System::demo();
    let p = plan_flight(
        &s,
        parked(&s, A, 0),
        B,
        PARK_OFFSET,
        DEMO_PEAK_SPEED_MM_S,
        WorldMs(0),
    )
    .unwrap();
    let mut last = 0;
    for i in 0..=200u64 {
        let d = dist_mm(p.from, flight_pos(&p, WorldMs(p.duration_ms * i / 200)));
        assert!(d >= last);
        last = d;
    }
}

#[test]
fn demo_flight_takes_a_sensible_time() {
    let s = System::demo();
    for i in 0..20u64 {
        let t = mix64(i) % 2_000_000;
        let p = plan_flight(
            &s,
            parked(&s, A, t),
            B,
            PARK_OFFSET,
            DEMO_PEAK_SPEED_MM_S,
            WorldMs(t),
        )
        .unwrap();
        assert!(
            (10_000..=600_000).contains(&p.duration_ms),
            "{}",
            p.duration_ms
        );
    }
}

#[test]
fn bad_inputs_are_rejected() {
    let s = System::demo();
    let from = parked(&s, A, 0);
    assert_eq!(
        plan_flight(&s, from, B, PARK_OFFSET, 0, WorldMs(0)),
        Err(ProcessError::ZeroSpeed)
    );
    assert_eq!(
        plan_flight(&s, from, BodyId(9), PARK_OFFSET, 1, WorldMs(0)),
        Err(ProcessError::UnknownBody)
    );
    let other = Pos {
        region: RegionId(1),
        ..from
    };
    assert_eq!(
        plan_flight(&s, other, B, PARK_OFFSET, 1, WorldMs(0)),
        Err(ProcessError::RegionMismatch)
    );
}

#[test]
fn zero_distance_has_zero_duration() {
    let s = System::demo();
    let from = s.body_pos(A, WorldMs(0)).unwrap();
    let p = plan_flight(&s, from, A, (0, 0, 0), DEMO_PEAK_SPEED_MM_S, WorldMs(0)).unwrap();
    assert_eq!(p.duration_ms, 0);
    assert_eq!(p.to, from);
    assert_eq!(flight_pos(&p, WorldMs(0)), from);
}

#[test]
fn huge_start_does_not_panic() {
    let s = System::demo();
    let t = u64::MAX - 5;
    let p = plan_flight(
        &s,
        parked(&s, A, t),
        B,
        PARK_OFFSET,
        DEMO_PEAK_SPEED_MM_S,
        WorldMs(t),
    )
    .unwrap();
    let _ = flight_pos(&p, WorldMs(u64::MAX));
}

#[test]
fn golden_flight_plan() {
    let s = System::demo();
    let p = plan_flight(
        &s,
        parked(&s, A, 0),
        B,
        PARK_OFFSET,
        DEMO_PEAK_SPEED_MM_S,
        WorldMs(0),
    )
    .unwrap();
    assert_eq!(
        (p.to.x, p.to.y, p.duration_ms),
        (-31891477111, 84210154052, 103768)
    );
}
