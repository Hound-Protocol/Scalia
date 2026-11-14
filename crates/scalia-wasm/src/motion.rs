//! Footwork with momentum: a body on foot cannot change its velocity faster than its feet can push. Speeding
//! up, braking and turning all draw on the same grip, and the faster he goes the less of it is left for
//! turning, so a sprint bends round in an arc and a reversal is a skid to a stop, then a push off the other way.

/// One step of a runner's velocity (m/s, x and y) towards `want` (where he is trying to go, at the pace he
/// wants) over `dt` seconds. `push` is how hard his feet can speed him up (m/s²), `brake` how hard they can
/// stop him; at `top` speed only `grip_at_top` of that is left.
pub fn step_velocity(vel: (f32, f32), want: (f32, f32), dt: f32, push: f32, brake: f32, top: f32) -> (f32, f32) {
    let speed = (vel.0 * vel.0 + vel.1 * vel.1).sqrt();
    let (dx, dy) = (want.0 - vel.0, want.1 - vel.1);
    let need = (dx * dx + dy * dy).sqrt();
    if need < 1e-5 {
        return want;
    }
    // Slowing down (the change points against his motion) uses the brake, otherwise the push; grip thins with
    // speed, so fast turns are wide.
    let against = speed > 1e-3 && (dx * vel.0 + dy * vel.1) < 0.0;
    let grip = (1.0 - (1.0 - GRIP_AT_TOP) * (speed / top.max(0.1)).min(1.0)) * if against { brake } else { push };
    let k = (grip * dt / need).min(1.0);
    (vel.0 + dx * k, vel.1 + dy * k)
}

/// The share of grip left at top speed.
pub const GRIP_AT_TOP: f32 = 0.45;

/// Whether he is skidding: going fast and being told to go some other way than he is (more than about 110
/// degrees off, or stop dead from a run). He plants his feet and slides until he has stopped enough to push off.
pub fn skidding(vel: (f32, f32), want: (f32, f32)) -> bool {
    let speed = (vel.0 * vel.0 + vel.1 * vel.1).sqrt();
    if speed < SKID_FROM {
        return false;
    }
    let w = (want.0 * want.0 + want.1 * want.1).sqrt();
    if w < 0.05 {
        return speed > SKID_STOP_FROM;
    }
    (vel.0 * want.0 + vel.1 * want.1) / (speed * w) < -0.35
}

/// Below this pace (m/s) he simply turns; above it a reversal is a skid.
pub const SKID_FROM: f32 = 2.2;
/// Stopping dead from at least this pace (m/s) is a skid too.
pub const SKID_STOP_FROM: f32 = 3.8;

#[cfg(test)]
mod tests {
    use super::*;

    fn run(mut v: (f32, f32), want: (f32, f32), secs: f32) -> Vec<(f32, f32)> {
        let mut out = Vec::new();
        for _ in 0..(secs / 0.016) as usize {
            v = step_velocity(v, want, 0.016, 4.2, 6.0, 4.6);
            out.push(v);
        }
        out
    }

    #[test]
    fn reversing_from_a_sprint_stops_first_then_takes_off_the_other_way() {
        // Sprinting left (-x), then told to sprint right.
        let path = run((-4.6, 0.0), (4.6, 0.0), 3.0);
        let stop = path.iter().position(|v| v.0 >= 0.0).expect("he gets turned round");
        assert!(stop as f32 * 0.016 > 0.7, "the stop takes a while at sprint ({:.2} s)", stop as f32 * 0.016);
        assert!(path.iter().all(|v| v.1.abs() < 1e-4), "straight back along the line, no sideways jump");
        assert!(path.last().unwrap().0 > 4.0, "then he gets going the other way");
        assert!(skidding((-4.6, 0.0), (4.6, 0.0)) && !skidding((-1.5, 0.0), (1.5, 0.0)));
    }

    #[test]
    fn a_sprint_turns_in_an_arc_a_walk_turns_on_the_spot() {
        let turned = |speed: f32| {
            let p = run((0.0, speed), (speed, 0.0), 0.25);
            let v = *p.last().unwrap();
            v.0.atan2(v.1).abs() // how far round from +y towards +x after a quarter second
        };
        assert!(turned(4.6) < turned(1.7), "sprint {} vs walk {}", turned(4.6), turned(1.7));
        assert!(!skidding((0.0, 4.6), (4.6, 0.0)), "a right-angle turn is a curve, not a skid");
    }
}
