//! Pack Engine acoustics: how a sound reaches the listener through the world. It reads the map's solid geometry
//! (the sight grid) and works out, for the listener, the space he is in (open field, street, room, corridor),
//! and for each sound, what the walls between, the distance and the air do to it. The browser's audio graph
//! applies the result (gain, filters, reverb sends, 3D position); nothing here plays sound.
//!
//! Not part of the deterministic simulation: plain floating point.

use scalia_ai::sight::SightGrid;

/// Ear height (mm) the rays are cast at.
const EAR_MM: i64 = 1_600;
/// Height a sound travels at between source and listener (chest).
const PATH_MM: i64 = 1_400;
/// Step along a ray (mm): the sight grid's cell size.
const STEP_MM: f32 = 250.0;
/// How far the listener's rays look for walls (m): beyond this it is open.
const ENV_REACH_M: f32 = 30.0;
/// Rays around the listener.
const RAYS: usize = 24;

/// The space the listener is in.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Env {
    /// 0 open field .. 1 closed room: the share of rays that meet a wall within reach (nearer counts more).
    pub enclosure: f32,
    /// Typical distance to the walls that were met (m); the reach when none were.
    pub size_m: f32,
    /// Reverberation time (s): short and dry in the open, long in a big hard room.
    pub rt60_s: f32,
    /// 0 .. 1: long and narrow (a corridor: walls close on two sides, open along the other).
    pub corridor: f32,
}

/// How one sound reaches the listener.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Path {
    /// Linear gain from distance and walls (1 at the source; the sound's own loudness is applied on top).
    pub gain: f32,
    /// Low-pass cut-off (Hz): air over distance and walls take the top off.
    pub lowpass_hz: f32,
    /// 0 .. 1: how much of it is heard as reverb rather than direct sound.
    pub wet: f32,
    /// Travel time (s), speed of sound over the real distance.
    pub delay_s: f32,
    /// Walls between source and listener.
    pub walls: u32,
}

/// Where the first wall is along a ray from `from` (m) in direction `dir` (unit), within `max_m`.
pub fn ray_hit(sight: &SightGrid, from: (f32, f32), dir: (f32, f32), max_m: f32, z_mm: i64) -> Option<f32> {
    let n = (max_m * 1000.0 / STEP_MM) as usize;
    for k in 1..=n {
        let d = k as f32 * STEP_MM / 1000.0;
        let (x, y) = (from.0 + dir.0 * d, from.1 + dir.1 * d);
        if sight.solid_at((x * 1000.0) as i64, (y * 1000.0) as i64, z_mm) {
            return Some(d);
        }
    }
    None
}

/// The listener's space at `at` (m): rays all around at ear height.
pub fn listener_env(sight: &SightGrid, at: (f32, f32)) -> Env {
    let mut hits = [f32::INFINITY; RAYS];
    for (k, h) in hits.iter_mut().enumerate() {
        let a = k as f32 * std::f32::consts::TAU / RAYS as f32;
        if let Some(d) = ray_hit(sight, at, (a.cos(), a.sin()), ENV_REACH_M, EAR_MM) {
            *h = d;
        }
    }
    let met: Vec<f32> = hits.iter().copied().filter(|d| d.is_finite()).collect();
    // A wall 2 m away encloses more than one 25 m away.
    let enclosure = hits.iter().map(|&d| if d.is_finite() { 1.0 - (d / ENV_REACH_M).powf(1.5) } else { 0.0 }).sum::<f32>() / RAYS as f32;
    let size_m = if met.is_empty() { ENV_REACH_M } else { met.iter().sum::<f32>() / met.len() as f32 };
    // Corridor: close walls on opposite sides with long open runs between them.
    let mut corridor = 0.0f32;
    for k in 0..RAYS / 2 {
        let (a, b) = (hits[k], hits[k + RAYS / 2]);
        let (c, d) = (hits[(k + RAYS / 4) % RAYS], hits[(k + 3 * RAYS / 4) % RAYS]);
        if a + b < 3.5 && (c > 8.0 || c.is_infinite()) && (d > 8.0 || d.is_infinite()) {
            corridor = corridor.max(1.0);
        }
    }
    // Sabine-like: bigger and more enclosed rings longer. Outdoors only a short slap off whatever is near.
    let rt60_s = (0.25 + enclosure * (0.25 + size_m * 0.12)).clamp(0.2, 2.5);
    Env { enclosure, size_m, rt60_s, corridor }
}

/// Solid walls crossed on the straight line from `a` to `b` (m), at chest height.
pub fn walls_between(sight: &SightGrid, a: (f32, f32), b: (f32, f32)) -> u32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len = (dx * dx + dy * dy).sqrt();
    let n = (len * 1000.0 / STEP_MM).ceil() as usize;
    let (mut walls, mut inside) = (0, false);
    for k in 1..n {
        let t = k as f32 / n as f32;
        let (x, y) = (a.0 + dx * t, a.1 + dy * t);
        let solid = sight.solid_at((x * 1000.0) as i64, (y * 1000.0) as i64, PATH_MM);
        if solid && !inside {
            walls += 1;
        }
        inside = solid;
    }
    walls
}

/// How a sound at `source` reaches a listener at `listener` (m) in `env`. `extra_m` is distance added on top
/// of the real one (zoomed out, the commander hears the fight as if further off): it dulls and wets the sound
/// but does not delay it.
pub fn path(sight: &SightGrid, listener: (f32, f32), source: (f32, f32), extra_m: f32, env: &Env) -> Path {
    let (dx, dy) = (source.0 - listener.0, source.1 - listener.1);
    let real = (dx * dx + dy * dy).sqrt();
    let walls = if real > 0.5 { walls_between(sight, listener, source) } else { 0 };
    let d = real + extra_m;
    // Distance: about -6 dB per doubling past 10 m; each wall takes most of what remains.
    let gain = 1.0 / (1.0 + d / 10.0) * 0.32f32.powi(walls as i32);
    // Air takes the highs over distance; a wall leaves a thump.
    let lowpass_hz = (18_000.0 / (1.0 + d / 70.0) * 0.22f32.powi(walls as i32)).clamp(250.0, 20_000.0);
    // Far off or behind walls you hear mostly the space; in a closed room everything rings.
    let wet = (0.08 + d / 140.0 + walls as f32 * 0.22 + env.enclosure * 0.35).clamp(0.05, 0.95);
    Path { gain, lowpass_hz, wet, delay_s: real / 343.0, walls }
}

/// Distance the commander's zoom adds (m), by on-foot zoom step: as heard, closest; slightly muffled; fully wet.
pub fn zoom_extra_m(step: usize) -> f32 {
    match step {
        0 => 0.0,
        1 => 40.0,
        _ => 150.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scalia_core::nav::{Prop, PropKind};

    fn wall(x: i64, y: i64, len: i64, turns: u8) -> Prop {
        Prop { kind: PropKind::Wall { length_mm: len, thickness_mm: 300 }, x_mm: x, y_mm: y, quarter_turns: turns }
    }
    fn grid(props: &[Prop]) -> SightGrid {
        SightGrid::build(0, 0, 400, 400, props)
    }

    #[test]
    fn the_open_field_is_open_and_a_room_is_closed() {
        let open = listener_env(&grid(&[]), (50.0, 50.0));
        assert!(open.enclosure < 0.01 && open.rt60_s < 0.35, "{open:?}");
        // A 6 x 6 m room around the listener.
        let room = [wall(50_000, 47_000, 6_300, 0), wall(50_000, 53_000, 6_300, 0), wall(47_000, 50_000, 6_300, 1), wall(53_000, 50_000, 6_300, 1)];
        let r = listener_env(&grid(&room), (50.0, 50.0));
        assert!(r.enclosure > 0.7 && r.size_m < 5.0 && r.rt60_s > open.rt60_s, "{r:?}");
    }

    #[test]
    fn a_corridor_is_recognised() {
        let c = [wall(50_000, 48_800, 40_000, 0), wall(50_000, 51_200, 40_000, 0)];
        assert!(listener_env(&grid(&c), (50.0, 50.0)).corridor > 0.5);
    }

    #[test]
    fn a_wall_between_muffles_and_quietens() {
        let g = grid(&[wall(50_000, 50_000, 20_000, 1)]);
        let env = listener_env(&g, (40.0, 50.0));
        let through = path(&g, (40.0, 50.0), (60.0, 50.0), 0.0, &env);
        let clear = path(&grid(&[]), (40.0, 50.0), (60.0, 50.0), 0.0, &env);
        assert_eq!(through.walls, 1);
        assert!(through.gain < clear.gain * 0.5 && through.lowpass_hz < clear.lowpass_hz * 0.3, "{through:?} vs {clear:?}");
        assert!(through.wet > clear.wet);
    }

    #[test]
    fn distance_dulls_delays_and_wets() {
        let g = grid(&[]);
        let env = listener_env(&g, (10.0, 10.0));
        let near = path(&g, (10.0, 10.0), (15.0, 10.0), 0.0, &env);
        let far = path(&g, (10.0, 10.0), (95.0, 10.0), 0.0, &env);
        assert!(far.gain < near.gain && far.lowpass_hz < near.lowpass_hz && far.wet > near.wet && far.delay_s > near.delay_s);
        // Zoomed out: duller and wetter, but not later.
        let zoomed = path(&g, (10.0, 10.0), (15.0, 10.0), zoom_extra_m(2), &env);
        assert!(zoomed.lowpass_hz < near.lowpass_hz && zoomed.wet > near.wet && zoomed.delay_s == near.delay_s);
    }
}
