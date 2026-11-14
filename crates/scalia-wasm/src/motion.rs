//! Footwork with momentum: a body on foot cannot change its velocity faster than its feet can push. Quick and
//! sharp at a jog; at a sprint the grip thins, so turns bend round in an arc, a reversal is a cut (plant, pivot,
//! push off), stopping dead is a short skid, and easing off from a sprint to a jog is a gradual slow-down.

/// How a body's feet work on the ground (m/s and m/s²).
#[derive(Copy, Clone, Debug)]
pub struct Footing {
    /// Speeding up to `cruise`.
    pub push: f32,
    /// Speeding up past `cruise` (towards a sprint): slower, a build-up.
    pub kick: f32,
    /// Stopping, or turning against the motion.
    pub brake: f32,
    /// Easing off while still going the same way (a sprint settling to a jog): gentle.
    pub coast: f32,
    /// The everyday pace (a jog) up to which `push` applies.
    pub cruise: f32,
    /// Top speed (a sprint): the grip thins towards it.
    pub top: f32,
}

/// One step of a runner's velocity (m/s, x and y) towards `want` (where he is trying to go, at the pace he
/// wants) over `dt` seconds.
pub fn step_velocity(vel: (f32, f32), want: (f32, f32), dt: f32, f: Footing) -> (f32, f32) {
    let speed = (vel.0 * vel.0 + vel.1 * vel.1).sqrt();
    let wanted = (want.0 * want.0 + want.1 * want.1).sqrt();
    let (dx, dy) = (want.0 - vel.0, want.1 - vel.1);
    let need = (dx * dx + dy * dy).sqrt();
    if need < 1e-5 {
        return want;
    }
    let aligned = speed > 1e-3 && wanted > 1e-3 && (vel.0 * want.0 + vel.1 * want.1) / (speed * wanted) > 0.9;
    let rate = if aligned && wanted < speed {
        f.coast
    } else if speed > 1e-3 && (dx * vel.0 + dy * vel.1) < 0.0 {
        f.brake
    } else if speed >= f.cruise {
        f.kick
    } else {
        f.push
    };
    // Grip thins with speed past the cruise: fast turns are wide.
    let over = ((speed - f.cruise) / (f.top - f.cruise).max(0.1)).clamp(0.0, 1.0);
    let grip = rate * (1.0 - (1.0 - GRIP_AT_TOP) * over);
    let k = (grip * dt / need).min(1.0);
    (vel.0 + dx * k, vel.1 + dy * k)
}

/// The share of grip left at top speed.
pub const GRIP_AT_TOP: f32 = 0.55;

/// What his feet are doing about a change of direction or a stop.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Skid {
    /// Running as told (or simply turning).
    None,
    /// Told to go back the way he came at pace: plant, pivot round, push off (a 180 cut).
    Cut,
    /// Let go of everything at a sprint: feet planted, a short slide to a stop.
    Stop,
}

/// Whether he is skidding, and how.
pub fn skidding(vel: (f32, f32), want: (f32, f32)) -> Skid {
    let speed = (vel.0 * vel.0 + vel.1 * vel.1).sqrt();
    let w = (want.0 * want.0 + want.1 * want.1).sqrt();
    if w < 0.05 {
        return if speed > SKID_STOP_FROM { Skid::Stop } else { Skid::None };
    }
    if speed >= CUT_FROM && (vel.0 * want.0 + vel.1 * want.1) / (speed * w) < -0.35 { Skid::Cut } else { Skid::None }
}

/// Below this pace (m/s) he simply turns; above it a reversal is a cut.
pub const CUT_FROM: f32 = 2.2;
/// Stopping dead from at least this pace (m/s, a sprint) is a skid.
pub const SKID_STOP_FROM: f32 = 3.6;

#[cfg(test)]
mod tests {
    use super::*;

    const FEET: Footing = Footing { push: 10.0, kick: 3.0, brake: 10.0, coast: 3.0, cruise: 3.0, top: 4.6 };

    fn run(mut v: (f32, f32), want: (f32, f32), secs: f32) -> Vec<(f32, f32)> {
        let mut out = Vec::new();
        for _ in 0..(secs / 0.016) as usize {
            v = step_velocity(v, want, 0.016, FEET);
            out.push(v);
        }
        out
    }
    fn secs_until(path: &[(f32, f32)], f: impl Fn(&(f32, f32)) -> bool) -> f32 {
        path.iter().position(f).map_or(f32::INFINITY, |k| k as f32 * 0.016)
    }

    #[test]
    fn a_jog_starts_and_stops_sharply_a_sprint_builds_up() {
        let start = run((0.0, 0.0), (0.0, 3.0), 2.0);
        assert!(secs_until(&start, |v| v.1 >= 2.9) < 0.4, "up to a jog quickly");
        let kick = run((0.0, 3.0), (0.0, 4.6), 2.0);
        let t = secs_until(&kick, |v| v.1 >= 4.5);
        assert!((0.4..1.5).contains(&t), "the sprint builds up ({t:.2} s)");
        let stop = run((0.0, 3.0), (0.0, 0.0), 1.0);
        assert!(secs_until(&stop, |v| v.1 <= 0.05) < 0.4, "a jog stops short");
    }

    #[test]
    fn easing_off_a_sprint_slows_to_a_jog_gradually_without_a_skid() {
        let ease = run((0.0, 4.6), (0.0, 3.0), 2.0);
        let t = secs_until(&ease, |v| v.1 <= 3.05);
        assert!(t > 0.35, "gradual ({t:.2} s)");
        assert_eq!(skidding((0.0, 4.6), (0.0, 3.0)), Skid::None);
        assert_eq!(skidding((0.0, 4.6), (0.0, 0.0)), Skid::Stop, "letting go of everything at a sprint skids");
        assert_eq!(skidding((0.0, 3.0), (0.0, 0.0)), Skid::None, "a jog just stops");
    }

    #[test]
    fn a_reversal_at_pace_is_a_cut_back_along_the_line() {
        let path = run((-4.6, 0.0), (4.6, 0.0), 3.0);
        let t = secs_until(&path, |v| v.0 >= 0.0);
        assert!((0.2..0.8).contains(&t), "plant and pivot ({t:.2} s)");
        assert!(path.iter().all(|v| v.1.abs() < 1e-4), "straight back along the line");
        assert!(path.last().unwrap().0 > 4.0, "then off the other way");
        assert_eq!(skidding((-4.6, 0.0), (4.6, 0.0)), Skid::Cut);
        assert_eq!(skidding((-1.5, 0.0), (1.5, 0.0)), Skid::None, "walking pace just turns round");
    }

    #[test]
    fn a_sprint_turns_in_an_arc_a_walk_turns_on_the_spot() {
        let turned = |speed: f32| {
            let p = run((0.0, speed), (speed, 0.0), 0.15);
            let v = *p.last().unwrap();
            v.0.atan2(v.1).abs()
        };
        assert!(turned(4.6) < turned(1.7), "sprint {} vs walk {}", turned(4.6), turned(1.7));
        assert_eq!(skidding((0.0, 4.6), (4.6, 0.0)), Skid::None, "a right-angle turn is a curve, not a skid");
    }
}
