use scalia_render::sky::{sky, Sky};

fn brightness(c: [f32; 3]) -> f32 {
    c.iter().sum::<f32>() / 3.0
}

#[test]
fn light_direction_is_unit_and_above_the_ground() {
    for k in 0..48 {
        let s: Sky = sky(k as f32 / 48.0);
        assert!((s.light_dir.length() - 1.0).abs() < 1e-4, "{k}");
        assert!(s.light_dir.z > 0.05, "light comes from above at {k}");
    }
}

#[test]
fn noon_is_bright_and_high_midnight_is_dark_and_blue() {
    let noon = sky(0.5);
    let night = sky(0.0);
    assert!(noon.light_dir.z > 0.7);
    assert!(brightness(noon.light_col) > 3.0 * brightness(night.light_col));
    assert!(brightness(noon.ambient) > brightness(night.ambient));
    assert!(night.ambient[2] > night.ambient[0], "night ambient is blue");
    assert!(noon.is_day && !night.is_day);
}

#[test]
fn dawn_light_is_warm_and_low() {
    let dawn = sky(0.27);
    assert!(dawn.is_day);
    assert!(dawn.light_dir.z < 0.5);
    assert!(dawn.light_col[0] > dawn.light_col[2], "warm: more red than blue");
}

#[test]
fn time_wraps() {
    let (a, b) = (sky(0.3), sky(1.3));
    assert!((a.light_dir - b.light_dir).length() < 1e-4);
    let c = sky(-0.7);
    assert!((a.light_dir - c.light_dir).length() < 1e-4);
}

#[test]
fn clock_text() {
    assert_eq!(scalia_render::sky::clock(0.5), "12:00");
    assert_eq!(scalia_render::sky::clock(0.25), "06:00");
    assert_eq!(scalia_render::sky::clock(1.0 + 23.5 / 24.0), "23:30");
}
