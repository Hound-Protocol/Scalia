use scalia_core::fixed::Fix;
use scalia_core::hash::mix64;
use scalia_core::trig::sin_cos_turns;

fn f(raw: i64) -> f64 {
    raw as f64 / 4294967296.0
}
fn t(num: i64, den: i64) -> Fix {
    Fix::from_ratio(num, den).unwrap()
}

#[test]
fn exact_at_quarter_turns() {
    let cases = [(0, 1, 0i64, 1i64), (1, 4, 1, 0), (1, 2, 0, -1), (3, 4, -1, 0)];
    for (n, d, s, c) in cases {
        let (sin, cos) = sin_cos_turns(t(n, d));
        assert!((sin.0 - s * Fix::ONE.0).abs() <= 1, "sin {n}/{d}: {}", sin.0);
        assert!((cos.0 - c * Fix::ONE.0).abs() <= 1, "cos {n}/{d}: {}", cos.0);
    }
}

#[test]
fn eighth_turn_is_root_half() {
    let (s, c) = sin_cos_turns(t(1, 8));
    let r = std::f64::consts::FRAC_1_SQRT_2;
    assert!((f(s.0) - r).abs() < 2e-6 && (f(c.0) - r).abs() < 2e-6);
}

#[test]
fn identity_holds() {
    for i in 0..5000u64 {
        let raw = mix64(i) as i64 >> 28; // spans negatives and values above one turn
        let (s, c) = sin_cos_turns(Fix(raw));
        let e = f(s.0).powi(2) + f(c.0).powi(2) - 1.0;
        assert!(e.abs() < 4e-6, "turns raw {raw}: {e}");
    }
}

#[test]
fn periodic() {
    for i in 0..1000u64 {
        let x = Fix(mix64(i) as i64 >> 30);
        let a = sin_cos_turns(x);
        assert_eq!(a, sin_cos_turns(Fix(x.0 + Fix::ONE.0)));
        assert_eq!(a, sin_cos_turns(Fix(x.0 - Fix::ONE.0)));
    }
}

#[test]
fn matches_reference_values() {
    for k in 0..8 {
        let turns = [0.1f64, 0.3, 0.6, 0.9, 0.017, 0.44, 0.71, 0.999][k];
        let raw = (turns * 4294967296.0).round() as i64;
        let (s, c) = sin_cos_turns(Fix(raw));
        let a = 2.0 * std::f64::consts::PI * (raw as f64 / 4294967296.0);
        assert!((f(s.0) - a.sin()).abs() < 2e-6, "sin {turns}");
        assert!((f(c.0) - a.cos()).abs() < 2e-6, "cos {turns}");
    }
}
