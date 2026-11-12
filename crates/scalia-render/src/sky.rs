//! Time of day: where the light comes from and what colour the world is.
//! `t` is the fraction of a day: 0 midnight, 0.25 sunrise, 0.5 noon, 0.75 sunset.

use glam::Vec3;

#[derive(Copy, Clone, Debug)]
pub struct Sky {
    /// Direction towards the main light (sun by day, moon by night); unit length, always above the ground.
    pub light_dir: Vec3,
    pub light_col: [f32; 3],
    pub ambient: [f32; 3],
    /// Haze and background colour.
    pub fog: [f32; 3],
    pub is_day: bool,
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

fn scale(a: [f32; 3], k: f32) -> [f32; 3] {
    [a[0] * k, a[1] * k, a[2] * k]
}

/// Latitude of the demo battlefield: the sun's arc leans south by this much.
const LATITUDE: f32 = 0.6;

pub fn sky(t: f32) -> Sky {
    let a = std::f32::consts::TAU * (t.rem_euclid(1.0) - 0.25);
    let (sa, ca) = a.sin_cos();
    let sun = Vec3::new(ca, -sa * LATITUDE.sin(), sa * LATITUDE.cos());
    let is_day = sa > 0.0;
    // Keep the light a little above the horizon so shadows stay finite.
    let lift = |v: Vec3, min_z: f32| Vec3::new(v.x, v.y, v.z.max(min_z)).normalize();
    let light_dir = if is_day { lift(sun, 0.12) } else { lift(-sun, 0.3) };
    let e = sa.max(0.0);
    let d = smoothstep(-0.1, 0.1, sa);
    let sun_col = scale(mix([1.0, 0.55, 0.3], [1.0, 0.97, 0.9], smoothstep(0.0, 0.5, e)), 0.45 + 0.55 * smoothstep(0.0, 0.3, e));
    let moon_col = [0.22, 0.27, 0.42];
    let ambient = mix([0.10, 0.12, 0.22], scale([0.38, 0.42, 0.5], 0.6 + 0.4 * e), d);
    let fog = mix([0.04, 0.05, 0.10], mix([0.75, 0.5, 0.4], [0.62, 0.70, 0.78], smoothstep(0.0, 0.4, e)), d);
    Sky { light_dir, light_col: mix(moon_col, sun_col, d), ambient, fog, is_day }
}

/// "HH:MM" for a time of day.
pub fn clock(t: f32) -> String {
    let m = (t.rem_euclid(1.0) * 1440.0).round() as u32 % 1440;
    format!("{:02}:{:02}", m / 60, m % 60)
}
