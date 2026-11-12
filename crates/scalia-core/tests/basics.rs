use scalia_core::fixed::Fix;
use scalia_core::position::{dist_mm, isqrt, Pos, RegionId};
use scalia_core::time::WorldMs;

#[test]
fn fix_floor_rounding() {
    assert_eq!(Fix::from_ratio(-1, 3).unwrap().floor_to_i64(), -1);
    assert_eq!(Fix::from_ratio(1, 3).unwrap().floor_to_i64(), 0);
    // Floor, not truncation: -2^-32 * 0.5 rounds to -2^-32, not 0.
    assert_eq!(Fix(-1).mul(Fix(1 << 31)), Fix(-1));
    // Exact case: -0.5 * 0.5 = -0.25.
    let half = Fix::from_ratio(1, 2).unwrap();
    let neg_half = Fix::from_ratio(-1, 2).unwrap();
    assert_eq!(neg_half.mul(half), Fix(-(1i64 << 30)));
}

#[test]
fn fix_div_zero_is_none() {
    assert_eq!(Fix::ONE.div(Fix::ZERO), None);
    assert_eq!(Fix::from_ratio(1, 0), None);
}

#[test]
fn fix_mul_saturates() {
    assert_eq!(
        Fix::from_int(1_000_000_000)
            .mul(Fix::from_int(2))
            .floor_to_i64(),
        2_000_000_000
    );
    assert_eq!(
        Fix::from_int(2_000_000_000).mul(Fix::from_int(2)),
        Fix(i64::MAX)
    );
}

#[test]
fn time_game_ms() {
    assert_eq!(WorldMs(1000).game_ms(), 24_000);
    assert_eq!(WorldMs(u64::MAX).game_ms(), u64::MAX);
    assert_eq!(WorldMs(5).since(WorldMs(9)), 0);
}

#[test]
fn pos_lerp_floors() {
    let r = RegionId(1);
    let a = Pos {
        region: r,
        x: -1,
        y: 0,
        z: 0,
    };
    let b = Pos {
        region: r,
        x: 0,
        y: 0,
        z: 0,
    };
    assert_eq!(a.lerp(b, 1, 3).unwrap().x, -1);
    assert_eq!(b.lerp(a, 1, 3).unwrap().x, -1);
    let c = Pos {
        region: RegionId(2),
        ..b
    };
    assert!(a.lerp(c, 1, 3).is_none());
}

#[test]
fn pos_dist() {
    let r = RegionId(1);
    let a = Pos {
        region: r,
        x: 0,
        y: 0,
        z: 0,
    };
    let b = Pos {
        region: r,
        x: 3000,
        y: 4000,
        z: 0,
    };
    assert_eq!(dist_mm(a, b), 5000);
    assert_eq!(isqrt(0), 0);
    assert_eq!(isqrt(24), 4);
}
