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
    /// 0 .. 1: how much rings in the listener's own space (a room around him); fades as he zooms out.
    pub wet_room: f32,
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

/// Solid walls crossed on the straight line from `a` to `b` (x, y, height; m): a camera high above hears
/// over the walls, one at ground level through them.
pub fn walls_between(sight: &SightGrid, a: (f32, f32, f32), b: (f32, f32, f32)) -> u32 {
    let (dx, dy, dz) = (b.0 - a.0, b.1 - a.1, b.2 - a.2);
    let len = (dx * dx + dy * dy).sqrt();
    let n = (len * 1000.0 / STEP_MM).ceil().max(1.0) as usize;
    let (mut walls, mut inside) = (0, false);
    for k in 1..n {
        let t = k as f32 / n as f32;
        let (x, y, z) = (a.0 + dx * t, a.1 + dy * t, a.2 + dz * t);
        // Above anything that stands on the testing ground: nothing more to cross.
        let solid = z < 8.0 && sight.solid_at((x * 1000.0) as i64, (y * 1000.0) as i64, (z * 1000.0) as i64);
        if solid && !inside {
            walls += 1;
        }
        inside = solid;
    }
    walls
}

/// How high the ears sit above the middle of the view (m) for a view this many metres tall: the camera's
/// distance, so zooming out takes you further from the fight (closest view about as heard, furthest fully
/// distant).
pub fn camera_height_m(view_h_m: f32) -> f32 {
    (view_h_m * 1.6).clamp(3.0, 400.0)
}

/// How a sound at `source` (m, on the ground; chest height) reaches ears at `listener` (x, y, height; m).
/// `at_source` is the space the sound is made in: its reverb rings there and is heard as such.
pub fn path(sight: &SightGrid, listener: (f32, f32, f32), source: (f32, f32), at_source: &Env) -> Path {
    let src = (source.0, source.1, PATH_MM as f32 / 1000.0);
    let (dx, dy, dz) = (src.0 - listener.0, src.1 - listener.1, src.2 - listener.2);
    let d = (dx * dx + dy * dy + dz * dz).sqrt();
    let walls = if d > 0.5 { walls_between(sight, listener, src) } else { 0 };
    // Distance: about -6 dB per doubling past 10 m; each wall takes most of what remains.
    let gain = 1.0 / (1.0 + d / 10.0) * 0.32f32.powi(walls as i32);
    // Air takes the highs over distance; a wall leaves a thump.
    let lowpass_hz = (18_000.0 / (1.0 + d / 70.0) * 0.22f32.powi(walls as i32)).clamp(250.0, 20_000.0);
    // The space it is made in rings (a room, a corridor); through walls you hear more of that than of it.
    let wet_room = (at_source.enclosure * 0.6 + walls as f32 * 0.15).clamp(0.0, 0.85);
    Path { gain, lowpass_hz, wet_room, delay_s: d / 343.0, walls }
}

/// One echo of a sound off a surface: how much later than the direct sound it arrives (s), how loud it is
/// against the direct sound (linear, < 1), and where it comes from (unit vector on the ground, from the ears
/// towards the surface).
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Echo {
    pub delay_s: f32,
    pub gain: f32,
    pub dir: (f32, f32),
}

/// Rays from a sound looking for something to bounce off.
const ECHO_RAYS: usize = 16;
/// How far a surface can be from the sound and still send an echo back (m).
const ECHO_REACH_M: f32 = 80.0;

/// The surfaces around a spot that a sound made there bounces off (points on walls, buildings, containers
/// within reach; m). Open ground has none.
pub fn reflectors(sight: &SightGrid, at: (f32, f32)) -> Vec<(f32, f32)> {
    let mut out = Vec::new();
    for k in 0..ECHO_RAYS {
        let a = (k as f32 + 0.5) * std::f32::consts::TAU / ECHO_RAYS as f32;
        let dir = (a.cos(), a.sin());
        if let Some(d) = ray_hit(sight, at, dir, ECHO_REACH_M, PATH_MM) {
            // Hugging a wall (under 1.5 m) it is part of the direct sound and the room, not an echo.
            if d > 1.5 {
                out.push((at.0 + dir.0 * d, at.1 + dir.1 * d));
            }
        }
    }
    out
}

/// The strongest echoes (up to four) a listener hears of a sound at `source`, off its reflectors. Each comes
/// later than the direct sound by its longer way round, weaker by distance and by what the surface swallows.
pub fn echoes(listener: (f32, f32, f32), source: (f32, f32), reflectors: &[(f32, f32)]) -> Vec<Echo> {
    let d3 = |a: (f32, f32, f32), b: (f32, f32, f32)| ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2) + (a.2 - b.2).powi(2)).sqrt();
    let z = PATH_MM as f32 / 1000.0;
    let s = (source.0, source.1, z);
    let direct = d3(listener, s);
    let direct_gain = 1.0 / (1.0 + direct / 10.0);
    let mut out: Vec<Echo> = reflectors
        .iter()
        .filter_map(|&w| {
            let w3 = (w.0, w.1, z);
            let path = d3(s, w3) + d3(w3, listener);
            let delay_s = (path - direct) / 343.0;
            // Closer than about 25 ms it fuses with the shot itself.
            if delay_s < 0.025 {
                return None;
            }
            let gain = (1.0 / (1.0 + path / 10.0)) * 0.45 / direct_gain;
            let (dx, dy) = (w.0 - listener.0, w.1 - listener.1);
            let len = (dx * dx + dy * dy).sqrt().max(0.001);
            Some(Echo { delay_s, gain: gain.min(0.6), dir: (dx / len, dy / len) })
        })
        .collect();
    out.sort_by(|a, b| b.gain.total_cmp(&a.gain));
    // Several rays on the same wall make one echo: keep those at least 15 ms apart.
    let mut kept: Vec<Echo> = Vec::new();
    for e in out {
        if kept.iter().all(|k| (k.delay_s - e.delay_s).abs() > 0.015) {
            kept.push(e);
        }
        if kept.len() == 4 {
            break;
        }
    }
    kept
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
    fn at_ground_level_a_wall_muffles_but_from_above_you_hear_over_it() {
        let g = grid(&[wall(50_000, 50_000, 20_000, 1)]);
        let open = listener_env(&grid(&[]), (60.0, 50.0));
        let low = path(&g, (40.0, 50.0, 1.6), (60.0, 50.0), &open);
        let clear = path(&grid(&[]), (40.0, 50.0, 1.6), (60.0, 50.0), &open);
        assert_eq!(low.walls, 1);
        assert!(low.gain < clear.gain * 0.5 && low.lowpass_hz < clear.lowpass_hz * 0.3, "{low:?} vs {clear:?}");
        let above = path(&g, (45.0, 50.0, 30.0), (60.0, 50.0), &open);
        assert_eq!(above.walls, 0, "{above:?}");
    }

    #[test]
    fn zooming_out_takes_you_further_from_the_fight() {
        let g = grid(&[]);
        let env = listener_env(&g, (50.0, 50.0));
        let close = path(&g, (50.0, 50.0, camera_height_m(12.0)), (52.0, 50.0), &env);
        let far = path(&g, (50.0, 50.0, camera_height_m(50.0)), (52.0, 50.0), &env);
        assert!(far.lowpass_hz < close.lowpass_hz && far.gain < close.gain && far.delay_s > close.delay_s);
    }

    #[test]
    fn a_shot_in_the_open_has_no_echo_and_one_by_a_wall_does() {
        let open = grid(&[]);
        assert!(reflectors(&open, (50.0, 50.0)).is_empty());
        assert!(echoes((20.0, 50.0, 20.0), (50.0, 50.0), &reflectors(&open, (50.0, 50.0))).is_empty(), "nothing to bounce off");
        // A long wall 20 m behind the shooter: one echo, about 40 m of extra way round, from that side.
        let g = grid(&[wall(70_000, 50_000, 60_000, 1)]);
        let e = echoes((20.0, 50.0, 20.0), (50.0, 50.0), &reflectors(&g, (50.0, 50.0)));
        assert!(!e.is_empty(), "the wall sends it back");
        assert!(e[0].delay_s > 0.08 && e[0].delay_s < 0.16 && e[0].gain < 0.6 && e[0].dir.0 > 0.5, "{e:?}");
    }

    #[test]
    fn a_shot_in_a_room_rings_with_the_room_wherever_you_hear_it_from() {
        let room = [wall(50_000, 47_000, 6_300, 0), wall(50_000, 53_000, 6_300, 0), wall(47_000, 50_000, 6_300, 1), wall(53_000, 50_000, 6_300, 1)];
        let g = grid(&room);
        let inside = listener_env(&g, (50.0, 50.0));
        let open = listener_env(&g, (20.0, 20.0));
        let a = path(&g, (50.0, 40.0, 30.0), (50.0, 50.0), &inside);
        let b = path(&g, (20.0, 10.0, 30.0), (20.0, 20.0), &open);
        assert!(a.wet_room > 0.4 && b.wet_room < 0.05, "{a:?} {b:?}");
    }
}
