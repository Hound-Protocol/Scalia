// Point lights (muzzle flashes, blasts): lcount.x of them; pl = (x, y, z, radius m), plc = (rgb, intensity).
struct U { m: mat4x4f, lm: mat4x4f, vdir: vec4f, pal: array<vec4f, 32>, light: vec4f, light_col: vec4f, ambient: vec4f, time: vec4f, bend: vec4f, bend2: vec4f, lcount: vec4f, pl: array<vec4f, 16>, plc: array<vec4f, 16> };

// Instances: a = (x, y, z, palette for the body), b = (heading, phase offset, steps per second, pose id),
// c = (previous pose id, time of the change in seconds, unused, unused): poses blend over 0.35 s.
// A figure faces +y; heading turns it around z. Poses port `pose.rs` (the tested reference) exactly.

struct PP { lean: f32, hip: vec2f, knee: vec2f, sh: vec2f, el: vec2f, head: f32, rifle: f32, prone: f32, drop: f32, reach: vec2f, sling: f32, bob: f32, twist: f32, look: f32, abd: vec2f, ryaw: f32, side: vec2f, flat: f32, tilt: f32 };

fn rot(q: vec3f, py: f32, pz: f32, ang: f32) -> vec3f {
  let s = sin(ang);
  let c = cos(ang);
  let dy = q.y - py;
  let dz = q.z - pz;
  return vec3f(q.x, py + dy * c - dz * s, pz + dy * s + dz * c);
}

// An arm point (port of pose.rs `arm`): elbow bend, shoulder swing, then an inward turn around the vertical
// through the shoulder at x = sx.
fn arm(q: vec3f, sh: f32, el: f32, reach: f32, abd: f32, sx: f32) -> vec3f {
  let p0 = rot(rot(q, 0.0, 1.09, el), 0.0, 1.40, sh);
  // Raise the arm out to the side around the front-to-back axis through the shoulder.
  let g = sign(sx);
  let dx0 = p0.x - sx;
  let dz0 = p0.z - 1.40;
  let p = vec3f(sx + dx0 * cos(abd) - dz0 * sin(abd) * g, p0.y, 1.40 + dx0 * sin(abd) * g + dz0 * cos(abd));
  let a = reach * g;
  let s = sin(a);
  let c = cos(a);
  let dx = p.x - sx;
  return vec3f(sx + dx * c - p.y * s, dx * s + p.y * c, p.z);
}

// The rifle slung on the back (port of pose.rs `slung`): rolled flat, muzzle down, diagonal from the right
// shoulder blade.
fn slung(v: vec3f) -> vec3f {
  let d = v - vec3f(0.238, 0.0, 0.81);
  let r = vec3f(d.z, d.y, -d.x);
  let s = sin(-1.5);
  let c = cos(-1.5);
  let p = vec3f(r.x, r.y * c - r.z * s, r.y * s + r.z * c);
  let s2 = sin(0.3);
  let c2 = cos(0.3);
  return vec3f(p.x * c2 - p.z * s2, p.y, p.x * s2 + p.z * c2) + vec3f(0.06, -0.27, 1.2);
}

fn leg(q: vec3f, hip: f32, knee: f32) -> vec3f {
  return rot(rot(q, 0.0, 0.46, -knee), 0.0, 0.88, hip);
}

// Ankle bend that keeps the boot level under weight (port of pose.rs `ankle`).
fn ankle(hip: f32, knee: f32) -> f32 {
  let shin = hip - knee;
  let t = clamp((knee - 0.45) / 0.45, 0.0, 1.0);
  let planted = 1.0 - t * t * (3.0 - 2.0 * t);
  return -(max(shin, 0.0) + 0.4 * min(shin, 0.0)) * planted;
}
// Port of pose.rs `ankle_flat`: the boot held flat to the ground by `flat`.
fn ankle_flat(hip: f32, knee: f32, flat: f32) -> f32 {
  let a = ankle(hip, knee);
  return a + (-(hip - knee) - a) * flat;
}

fn foot(q: vec3f, hip: f32, knee: f32, flat: f32) -> vec3f {
  return leg(rot(q, 0.0, 0.085, ankle_flat(hip, knee, flat)), hip, knee);
}

// Gait for both legs (port of pose.rs `gait`): x = left, y = right; the right leg is half a cycle behind.
// One leg through a stride (port of pose.rs `gait_leg`): x = hip, y = knee; heel strike at u = 0.
fn gait_leg(u: f32, swing: f32, forward: f32, lift: f32, base: f32, stance: f32) -> vec2f {
  if (u < stance) {
    let s = u / stance;
    let ks = sin(3.14159265 * s);
    return vec2f(forward + swing * (1.0 - 2.0 * s), base + 0.03 * ks * ks);
  }
  let w = (u - stance) / (1.0 - stance);
  let m = -2.0 * swing / stance * (1.0 - stance);
  let w2 = w * w;
  let w3 = w2 * w;
  let hip = (2.0 * w3 - 3.0 * w2 + 1.0) * -swing + (w3 - 2.0 * w2 + w) * m + (-2.0 * w3 + 3.0 * w2) * swing + (w3 - w2) * m;
  let kw = sin(3.14159265 * w);
  return vec2f(forward + hip, base + lift * kw * kw);
}
// Port of pose.rs `gait_dir`: `dir` is the way the body moves while facing ahead (from straight ahead towards the
// left); the swing splits into forward (hip) and sideways (side) parts, shorter to the side (`stride_scale`), feet
// a little apart (SIDE_SPREAD 0.06).
fn gait(p: ptr<function, PP>, phase: f32, swing: f32, forward: f32, lift: f32, base: f32, stance: f32, dir: f32) {
  let l = gait_leg(fract(phase), swing, forward, lift, base, stance);
  let r = gait_leg(fract(phase + 0.5), swing, forward, lift, base, stance);
  (*p).knee = vec2f(l.y, r.y);
  let s = sin(dir);
  let c = cos(dir);
  let k = 1.0 - 0.45 * abs(s);
  let e = (vec2f(l.x, r.x) - vec2f(forward)) * k;
  (*p).hip = vec2f(forward) + e * c;
  (*p).side = -e * s + vec2f(-1.0, 1.0) * 0.06 * abs(s);
}

// Port of pose.rs `crouch_gait`: foot targets move evenly while planted and arc forward in the swing; hip and knee
// by two-bone IK (thigh 0.42, shin 0.375), ankle `height` below the hip.
fn crouch_gait(p: ptr<function, PP>, phase: f32, reach: f32, height: f32, lift: f32, stance: f32, dir: f32) {
  let l1 = 0.42;
  let l2 = 0.375;
  let k = 1.0 - 0.45 * abs(sin(dir));
  let s = sin(dir);
  let c = cos(dir);
  for (var i = 0; i < 2; i++) {
    let u = fract(phase + select(0.5, 0.0, i == 0));
    var along = reach * (1.0 - 2.0 * u / stance);
    var raise = 0.0;
    if (u >= stance) {
      let w = (u - stance) / (1.0 - stance);
      let m = -2.0 * reach / stance * (1.0 - stance);
      let w2 = w * w;
      let w3 = w2 * w;
      along = (2.0 * w3 - 3.0 * w2 + 1.0) * -reach + (w3 - 2.0 * w2 + w) * m + (-2.0 * w3 + 3.0 * w2) * reach + (w3 - w2) * m;
      raise = lift * pow(sin(3.1415927 * w), 2.0);
    }
    let out = select(1.0, -1.0, i == 0);
    let fy = along * k * c;
    let fx = -along * k * s + out * 0.06 * abs(s);
    let h = height - raise;
    let hz = sqrt(h * h + fx * fx);
    let d = min(sqrt(fy * fy + hz * hz), l1 + l2 - 1e-3);
    let bend = acos(clamp((d * d - l1 * l1 - l2 * l2) / (2.0 * l1 * l2), -1.0, 1.0));
    (*p).side[i] = atan2(fx, h);
    (*p).hip[i] = atan2(fy, hz) + asin(clamp(l2 * sin(bend) / d, -1.0, 1.0));
    (*p).knee[i] = bend;
  }
}

// A leg point swung out to the side around the front-to-back line through the hip at px (port of pose.rs
// `sideways`, HIP_X 0.1).
fn sideways(q: vec3f, a: f32, px: f32) -> vec3f {
  let s = sin(a);
  let c = cos(a);
  let dx = q.x - px;
  let dz = q.z - 0.88;
  return vec3f(px + dx * c - dz * s, q.y, 0.88 + dx * s + dz * c);
}

// Rifle carries (port of pose.rs `carry`): 0 safe hang, 1 low ready, 2 collapsed low ready, 3 high ready,
// 4 ready up, 5 rush (combat run). Sets both arms and the rifle angle.
// The athletic base of the ready and the aim (pose.rs `athletic`): feet staggered, knees flexed, boots flat.
fn athletic(p: ptr<function, PP>) {
  crouch_gait(p, 0.0, 0.13, 0.75, 0.0, 0.62, 0.0);
  (*p).flat = 1.0;
}

fn carry(p: ptr<function, PP>, c: u32) {
  // Each carry turns the rifle across the body (ryaw): stock out to the right shoulder, muzzle to the left.
  // Generated from pose.rs `carry` (scripts: carry_ik, then copy the table).
  if (c == 0u) {
    (*p).sh = vec2f(0.42, -0.44); (*p).el = vec2f(0.44, 2.01); (*p).reach = vec2f(0.54, 1.22); (*p).abd = vec2f(0.12, 0.57); (*p).rifle = -0.85; (*p).ryaw = 0.90;
  } else if (c == 1u) {
    (*p).sh = vec2f(0.55, -0.09); (*p).el = vec2f(0.73, 1.92); (*p).reach = vec2f(0.13, 0.87); (*p).abd = vec2f(-0.04, 0.53); (*p).rifle = -0.50; (*p).ryaw = 0.74;
  } else if (c == 2u) {
    (*p).sh = vec2f(0.48, -0.39); (*p).el = vec2f(0.42, 1.96); (*p).reach = vec2f(0.17, 1.24); (*p).abd = vec2f(-0.11, 0.63); (*p).rifle = -0.75; (*p).ryaw = 0.81;
  } else if (c == 3u) {
    (*p).sh = vec2f(1.21, 0.12); (*p).el = vec2f(0.95, 1.76); (*p).reach = vec2f(0.43, 0.57); (*p).abd = vec2f(0.18, 0.51); (*p).rifle = 0.85; (*p).ryaw = 0.41;
  } else if (c == 4u) {
    (*p).sh = vec2f(1.13, 0.31); (*p).el = vec2f(0.51, 1.84); (*p).reach = vec2f(0.14, 0.58); (*p).abd = vec2f(-0.03, 0.50); (*p).rifle = 0.00; (*p).ryaw = 0.55;
    // Ready up is the aim (pose.rs `aim_arms`): bladed stance, muzzle straight ahead, head looking along it.
    (*p).twist = -0.55; (*p).look = 0.55;
  } else if (c == 5u) {
    (*p).sh = vec2f(0.97, 0.13); (*p).el = vec2f(0.52, 1.86); (*p).reach = vec2f(0.24, 0.80); (*p).abd = vec2f(0.22, 0.69); (*p).rifle = -0.20; (*p).ryaw = 0.58;
  } else if (c == 6u) {
    // Combat low ready (pose.rs `Carry::LowReadyFwd`), bladed so the muzzle points ahead and down.
    (*p).sh = vec2f(0.65, -0.13); (*p).el = vec2f(0.45, 1.95); (*p).reach = vec2f(0.36, 0.70); (*p).abd = vec2f(0.14, 0.34); (*p).rifle = -0.60; (*p).ryaw = 0.70;
    (*p).twist = -0.70; (*p).look = 0.70;
  }
}

// The stride's life (port of pose.rs `stride`): rise and fall, upper body turning against the legs, rifle sway.
fn stride(p: ptr<function, PP>, phase: f32, bob: f32, twist: f32, arms: f32, rifle: f32, swing: f32) {
  let t = phase * 6.2831853;
  (*p).bob = bob * cos(2.0 * t);
  (*p).twist = twist * sin(t);
  (*p).look = (*p).look - 0.8 * (*p).twist;
  let sway = sin(2.0 * t + 0.5);
  // SWAY_GAIN (pose.rs) 1.6: the weapon swings more than the body.
  (*p).sh = (*p).sh + vec2f(1.6 * arms * sway);
  (*p).rifle = (*p).rifle + 1.25 * rifle * sin(2.0 * t);
  (*p).ryaw = (*p).ryaw + swing * sin(t);
}

const IDLE_PRONE_BREATH: f32 = 0.006;
const IDLE_PRONE_HEAD_BREATH: f32 = 0.010;
const IDLE_PRONE_SCAN: f32 = 0.045;
const IDLE_PRONE_SCAN_PERIOD_S: f32 = 8.5;
const IDLE_PRONE_RIFLE_SWAY: f32 = 0.004;
const IDLE_PRONE_RIFLE_PERIOD_S: f32 = 3.2;
const IDLE_PRONE_RIFLE_YAW: f32 = 0.006;
const IDLE_PRONE_RIFLE_YAW_PERIOD_S: f32 = 4.1;
const IDLE_TACTICAL_BREATH: f32 = 0.012;
const IDLE_TACTICAL_LEAN: f32 = 0.003;
const IDLE_TACTICAL_TWIST: f32 = 0.028;
const IDLE_TACTICAL_TWIST_PERIOD_S: f32 = 10.0;
const IDLE_TACTICAL_SCAN: f32 = 0.16;
const IDLE_TACTICAL_SCAN_PERIOD_S: f32 = 12.0;
const IDLE_TACTICAL_RIFLE_SWAY: f32 = 0.006;
const IDLE_TACTICAL_RIFLE_PERIOD_S: f32 = 4.6;
const IDLE_TACTICAL_RIFLE_YAW: f32 = 0.009;
const IDLE_TACTICAL_RIFLE_YAW_PERIOD_S: f32 = 6.0;
const IDLE_PEEK_MIN: f32 = 0.12;
const IDLE_PEEK_PERIOD_S: f32 = 5.2;

// Idle life for soldiers standing still (port of pose.rs `idle`): breathing, weight shifts, glances, a rifle hitch.
fn idle(p0: PP, pose: u32, t: f32, seed: f32) -> PP {
  var q = p0;
  // Moving gaits and the close-up tablet view supply their own motion (or must stay perfectly still).
  if (pose == 1u || pose == 2u || pose == 7u || pose == 11u || pose == 10u || pose >= 21u) { return q; }
  let tau = 6.2831853;
  let breathe = sin(tau * (t / 3.8 + seed));

  // Prone: the chest works against the elbows, with only a small head scan and rifle settle.
  // Keep the legs and body-to-ground transform fixed so the soldier never floats or sinks.
  if (pose == 5u) {
    q.sh = q.sh + vec2f(IDLE_PRONE_BREATH * breathe);
    q.head = q.head + IDLE_PRONE_HEAD_BREATH * breathe;
    q.look = q.look + IDLE_PRONE_SCAN * sin(tau * (t / IDLE_PRONE_SCAN_PERIOD_S + seed * 1.4));
    q.rifle = q.rifle + IDLE_PRONE_RIFLE_SWAY * sin(tau * (t / IDLE_PRONE_RIFLE_PERIOD_S + seed));
    q.ryaw = q.ryaw + IDLE_PRONE_RIFLE_YAW * sin(tau * (t / IDLE_PRONE_RIFLE_YAW_PERIOD_S + seed * 1.3));
    return q;
  }

  // Behind cover and stacked: quiet breathing, a restrained weight shift and short sector scans.
  if (pose == 8u || pose == 15u) {
    q.sh = q.sh + vec2f(IDLE_TACTICAL_BREATH * breathe);
    q.lean = q.lean + IDLE_TACTICAL_LEAN * breathe;
    q.twist = q.twist + IDLE_TACTICAL_TWIST * sin(tau * (t / IDLE_TACTICAL_TWIST_PERIOD_S + seed * 1.7));
    q.look = q.look + IDLE_TACTICAL_SCAN * sin(tau * (t / IDLE_TACTICAL_SCAN_PERIOD_S + seed * 2.3));
    q.rifle = q.rifle + IDLE_TACTICAL_RIFLE_SWAY * sin(tau * (t / IDLE_TACTICAL_RIFLE_PERIOD_S + seed));
    q.ryaw = q.ryaw + IDLE_TACTICAL_RIFLE_YAW * sin(tau * (t / IDLE_TACTICAL_RIFLE_YAW_PERIOD_S + seed * 1.3));
    return q;
  }

  if (pose == 6u || pose == 4u || pose == 12u || pose == 13u || pose == 14u) {
    q.sh = q.sh + vec2f(0.008 * breathe);
    q.look = q.look + 0.03 * sin(tau * (t / 7.0 + seed));
    q.rifle = q.rifle + 0.018 * sin(tau * (t / 2.3 + seed));
    q.ryaw = q.ryaw + 0.022 * sin(tau * (t / 3.1 + seed * 1.3));
    // Eased return towards cover and lean back out; zero velocity at both ends.
    if (pose == 12u || pose == 13u) {
      let cycle = tau * (t / IDLE_PEEK_PERIOD_S + seed);
      let exposure = 0.5 - 0.5 * cos(cycle);
      q.tilt = q.tilt * (IDLE_PEEK_MIN + (1.0 - IDLE_PEEK_MIN) * exposure);
    }
    return q;
  }

  q.sh = q.sh + vec2f(0.03 * breathe);
  q.lean = q.lean + 0.008 * breathe;
  q.twist = q.twist + 0.06 * sin(tau * (t / 9.5 + seed * 1.7));
  if (pose != 10u) {
    let g = sin(tau * (t / 14.0 + seed * 2.3));
    q.look = q.look + 0.6 * g * g * g;
  }
  if (pose == 0u) {
    q.rifle = q.rifle + 0.1 * pow(max(sin(tau * (t / 12.0 + seed)), 0.0), 12.0);
  }
  return q;
}

fn turn(q: vec3f, a: f32) -> vec3f {
  let s = sin(a);
  let c = cos(a);
  return vec3f(q.x * c - q.y * s, q.x * s + q.y * c, q.z);
}

// The drop's middle keys (pose.rs `drop_reach`, `drop_plant`), as literals: the CPU hands the shader the two keys
// of the drop around its phase (pose.rs `drop_keys`) so the drop costs no extra pose evaluations per vertex.
const DROP_REACH_P: PP = PP(1.100000, vec2f(1.550000, -0.100000), vec2f(1.800000, 1.750000), vec2f(1.000000, -0.090000), vec2f(0.100000, 1.920000), 0.900000, -0.500000, 0.000000, -0.390930, vec2f(0.100000, 0.870000), 0.000000, 0.000000, 0.000000, 0.000000, vec2f(0.150000, 0.530000), 0.740000, vec2f(0.000000, 0.000000), 0.000000, 0.000000);
const DROP_PLANT_P: PP = PP(0.933810, vec2f(0.900000, 0.900000), vec2f(1.350000, 1.350000), vec2f(0.900000, 1.985742), vec2f(0.500000, 1.510598), 0.600000, 0.987349, 0.718250, -0.143823, vec2f(0.100000, 0.245123), 0.000000, 0.000000, 0.000000, 0.000000, vec2f(0.150000, 0.149328), 0.208495, vec2f(0.000000, 0.000000), 0.000000, 0.000000);

fn pose_params(pose: u32, phase: f32, dir: f32) -> PP {
  if (pose == 34u) { return DROP_REACH_P; }
  if (pose == 35u) { return DROP_PLANT_P; }
  let sw = sin(phase * 6.2831853);
  var p = PP(0.0, vec2f(0.0), vec2f(0.0), vec2f(0.0), vec2f(0.0), 0.0, 0.0, 0.0, 0.0, vec2f(0.0), 0.0, 0.0, 0.0, 0.0, vec2f(0.0), 0.0, vec2f(0.0), 0.0, 0.0);
  if (pose == 0u) {
    carry(&p, 0u);
  } else if (pose == 1u) {
    gait(&p, phase, 0.45, 0.0, 0.75, 0.06, 0.6, dir);
    p.lean = 0.05;
    carry(&p, 1u);
    stride(&p, phase, 0.018, 0.06, 0.035, 0.025, 0.04);
  } else if (pose == 2u) {
    gait(&p, phase, 0.6, 0.2, 1.15, 0.4, 0.45, dir);
    p.lean = 0.32;
    // The combat ready carried into the jog (pose.rs `Pose::Run`): same hold, bladed.
    carry(&p, 6u);
    p.rifle = p.rifle + 0.15;
    p.head = 0.28;
    stride(&p, phase, 0.03, 0.2, 0.08, 0.06, 0.16);
  } else if (pose == 7u) {
    gait(&p, phase, 0.56, 0.1, 1.45, 0.2, 0.36, dir);
    p.lean = 0.42;
    carry(&p, 2u);
    p.head = 0.38;
    stride(&p, phase, 0.035, 0.3, 0.06, 0.08, 0.24);
  } else if (pose == 3u) {
    p.hip = vec2f(1.55, -0.1);
    p.knee = vec2f(1.8, 1.75);
    p.lean = 0.12;
    carry(&p, 1u);
    p.head = 0.35;
  } else if (pose == 4u) {
    p.hip = vec2f(1.55, -0.1);
    p.knee = vec2f(1.8, 1.75);
    p.lean = 0.12;
    carry(&p, 4u);
    p.head = -0.25;
  } else if (pose == 5u) {
    p.prone = 1.0;
    p.sh = vec2f(2.8);
    p.el = vec2f(1.3, 1.35);
    p.rifle = 1.5707963;
    p.head = 1.0;
  } else if (pose == 6u) {
    // On the athletic base (pose.rs `athletic`), leaning in by AIM_LEAN 0.1 with the rifle and head taking it back.
    athletic(&p);
    carry(&p, 4u);
    p.lean = 0.1;
    p.rifle = p.rifle + 0.1;
    p.head = -0.45;
  } else if (pose == 30u) {
    // Combat ready (pose.rs `Pose::Ready`): athletic base, low ready forward, READY_LEAN 0.16.
    athletic(&p);
    carry(&p, 6u);
    p.lean = 0.16;
    p.head = -0.16;
  } else if (pose == 8u) {
    carry(&p, 3u);
  } else if (pose == 15u) {
    // Stacked at a door (pose.rs `Pose::Stack`).
    crouch_gait(&p, 0.0, 0.12, 0.7, 0.0, 0.6, 0.0);
    p.flat = 1.0;
    p.lean = 0.22;
    carry(&p, 1u);
    p.head = 0.05;
  } else if (pose == 16u) {
    // Sweeping a room (pose.rs `Pose::Sweep`, SWEEP 0.6).
    carry(&p, 4u);
    p.head = -0.35;
    let st = sin(phase * 6.2831853);
    p.twist = p.twist + 0.6 * st;
    p.look = p.look - 0.3 * 0.6 * st;
    p.hip = vec2f(0.05);
    p.knee = vec2f(0.12);
  } else if (pose == 17u) {
    // Grenade throw (pose.rs `Pose::Throw`).
    p.sling = 1.0;
    p.hip = vec2f(0.22, -0.12);
    p.knee = vec2f(0.18, 0.12);
    let k = array<f32, 5>(0.0, 0.35, 0.55, 0.75, 1.0);
    p.sh = vec2f(keyed(phase, k, array<f32, 5>(0.9, 1.5, 0.8, 0.5, 0.9)), keyed(phase, k, array<f32, 5>(0.7, 2.6, 2.0, 0.9, 0.7)));
    p.el = vec2f(keyed(phase, k, array<f32, 5>(1.4, 0.3, 1.4, 1.6, 1.4)), keyed(phase, k, array<f32, 5>(1.6, 1.9, 0.3, 0.4, 1.6)));
    p.reach = vec2f(keyed(phase, k, array<f32, 5>(0.4, 0.2, 0.5, 0.5, 0.4)), keyed(phase, k, array<f32, 5>(0.3, 0.0, 0.2, 0.6, 0.3)));
    p.abd = vec2f(0.1, keyed(phase, k, array<f32, 5>(0.2, 0.6, 0.2, 0.1, 0.2)));
    p.twist = keyed(phase, k, array<f32, 5>(0.0, -0.5, 0.4, 0.5, 0.0));
    p.lean = keyed(phase, k, array<f32, 5>(0.05, -0.1, 0.25, 0.3, 0.05));
    p.head = 0.0;
  } else if (pose == 18u) {
    // Magazine change (pose.rs `Pose::Reload`); the left hand is placed in `animate` by reach_left.
    carry(&p, 1u);
    p.rifle = p.rifle + 0.3 * sin(3.1415927 * phase);
    p.head = 0.4;
  } else if (pose == 19u) {
    // Hand signal (pose.rs `Pose::Signal`).
    carry(&p, 1u);
    let k = array<f32, 5>(0.0, 0.2, 0.45, 0.7, 1.0);
    p.sh.x = keyed(phase, k, array<f32, 5>(0.8, 2.9, 2.9, 1.6, 0.8));
    p.el.x = keyed(phase, k, array<f32, 5>(1.4, 1.9, 1.9, 0.15, 1.4));
    p.reach.x = keyed(phase, k, array<f32, 5>(0.5, 0.1, 0.1, 0.1, 0.5));
    p.abd.x = keyed(phase, k, array<f32, 5>(0.0, 0.3, 0.3, 0.0, 0.0));
    p.head = 0.1;
  } else if (pose == 20u) {
    // Kneeling drone launch (pose.rs `Pose::Drone`).
    p.hip = vec2f(1.55, -0.1);
    p.knee = vec2f(1.8, 1.75);
    p.lean = 0.12;
    p.sling = 1.0;
    let k = array<f32, 5>(0.0, 0.35, 0.55, 0.8, 1.0);
    let sh = keyed(phase, k, array<f32, 5>(0.9, 1.0, 2.7, 2.7, 0.9));
    let el = keyed(phase, k, array<f32, 5>(1.6, 1.6, 0.4, 0.4, 1.6));
    p.sh = vec2f(sh);
    p.el = vec2f(el);
    p.reach = vec2f(0.45);
    p.head = keyed(phase, k, array<f32, 5>(0.2, 0.2, 0.6, 0.7, 0.2));
  } else if (pose == 21u) {
    // Wounded, on his back (pose.rs `Pose::Wounded`).
    p.prone = -1.0;
    p.sling = 1.0;
    let k = array<f32, 5>(0.0, 0.3, 0.5, 0.75, 1.0);
    p.hip = vec2f(keyed(phase, k, array<f32, 5>(0.85, 1.05, 0.95, 0.75, 0.85)), 0.1);
    p.knee = vec2f(keyed(phase, k, array<f32, 5>(1.45, 1.7, 1.55, 1.3, 1.45)), 0.12);
    p.sh = vec2f(0.55, 0.15);
    p.el = vec2f(1.95, 0.3);
    p.reach = vec2f(0.75, 0.0);
    p.abd = vec2f(0.0, 1.05);
    p.head = keyed(phase, k, array<f32, 5>(-0.45, -0.6, -0.5, -0.35, -0.45));
    p.look = 0.25;
  } else if (pose == 22u) {
    // Dragging a wounded man (pose.rs `Pose::Drag`).
    crouch_gait(&p, 0.0, 0.15, 0.6, 0.0, 0.6, 0.0);
    p.flat = 1.0;
    p.lean = 0.55;
    p.sling = 1.0;
    p.sh = vec2f(1.0);
    p.el = vec2f(0.25);
    p.reach = vec2f(0.3);
    p.head = 0.45;
  } else if (pose == 25u) {
    // Shot, falling back onto his back (pose.rs `Pose::FallBack`; ends in the wounded pose).
    let k = array<f32, 5>(0.0, 0.2, 0.45, 0.75, 1.0);
    let t = phase;
    p.sling = keyed(t, k, array<f32, 5>(0.0, 0.6, 1.0, 1.0, 1.0));
    p.lean = keyed(t, k, array<f32, 5>(0.0, -0.35, -0.15, 0.0, 0.0));
    p.hip = vec2f(keyed(t, k, array<f32, 5>(0.0, -0.25, 0.9, 0.95, 0.85)), keyed(t, k, array<f32, 5>(0.0, 0.35, 0.7, 0.3, 0.1)));
    p.knee = vec2f(keyed(t, k, array<f32, 5>(0.0, 0.3, 1.5, 1.6, 1.45)), keyed(t, k, array<f32, 5>(0.0, 0.5, 1.3, 0.6, 0.12)));
    p.sh = vec2f(keyed(t, k, array<f32, 5>(0.3, 0.9, 1.2, 0.8, 0.55)), keyed(t, k, array<f32, 5>(0.3, 1.1, 1.4, 0.6, 0.15)));
    p.el = vec2f(keyed(t, k, array<f32, 5>(1.2, 0.6, 0.8, 1.6, 1.95)), keyed(t, k, array<f32, 5>(1.2, 0.5, 0.4, 0.3, 0.3)));
    p.reach = vec2f(keyed(t, k, array<f32, 5>(0.4, 0.2, 0.3, 0.6, 0.75)), 0.0);
    p.abd = vec2f(0.0, keyed(t, k, array<f32, 5>(0.0, 0.5, 0.8, 1.0, 1.05)));
    p.prone = keyed(t, k, array<f32, 5>(0.0, 0.0, -0.15, -0.7, -1.0));
    p.head = keyed(t, k, array<f32, 5>(0.0, 0.35, -0.2, -0.5, -0.45));
    p.look = keyed(t, k, array<f32, 5>(0.0, 0.0, 0.1, 0.2, 0.25));
  } else if (pose == 26u) {
    // Killed, crumpling forward (pose.rs `Pose::FallForward`; ends in the dead pose).
    let k = array<f32, 5>(0.0, 0.18, 0.45, 0.75, 1.0);
    let t = phase;
    p.sling = keyed(t, k, array<f32, 5>(0.0, 0.7, 1.0, 1.0, 1.0));
    p.lean = keyed(t, k, array<f32, 5>(0.0, -0.25, 0.45, 0.2, 0.0));
    p.hip = vec2f(keyed(t, k, array<f32, 5>(0.0, 0.1, 1.3, 0.8, 0.15)), keyed(t, k, array<f32, 5>(0.0, -0.1, 1.1, 0.5, -0.2)));
    p.knee = vec2f(keyed(t, k, array<f32, 5>(0.0, 0.2, 1.7, 1.0, 0.25)), keyed(t, k, array<f32, 5>(0.0, 0.15, 1.6, 1.0, 0.6)));
    p.sh = vec2f(keyed(t, k, array<f32, 5>(0.3, 0.6, 0.4, 1.3, 1.9)), keyed(t, k, array<f32, 5>(0.3, 0.7, 0.3, 0.8, 0.6)));
    p.el = vec2f(keyed(t, k, array<f32, 5>(1.2, 0.8, 0.3, 0.5, 0.6)), keyed(t, k, array<f32, 5>(1.2, 0.8, 0.3, 0.7, 0.9)));
    p.reach = vec2f(0.0);
    p.abd = vec2f(keyed(t, k, array<f32, 5>(0.0, 0.2, 0.3, 0.5, 0.7)), keyed(t, k, array<f32, 5>(0.0, 0.2, 0.3, 0.6, 0.9)));
    p.prone = keyed(t, k, array<f32, 5>(0.0, 0.0, 0.0, 0.55, 1.0));
    p.head = keyed(t, k, array<f32, 5>(0.0, 0.3, -0.4, 0.2, 0.55));
    p.look = keyed(t, k, array<f32, 5>(0.0, 0.0, 0.2, 0.6, 0.9));
  } else if (pose == 27u) {
    // Killed lying on his back (pose.rs `Pose::DeadBack`).
    p.prone = -1.0;
    p.sling = 1.0;
    p.hip = vec2f(0.25, 0.1);
    p.knee = vec2f(0.45, 0.15);
    p.sh = vec2f(0.35, 0.3);
    p.el = vec2f(0.5, 0.4);
    p.reach = vec2f(0.0);
    p.abd = vec2f(0.9, 1.0);
    p.head = -0.3;
    p.look = 0.8;
  } else if (pose == 24u) {
    // Killed, face down and limp (pose.rs `Pose::Dead`).
    p.prone = 1.0;
    p.sling = 1.0;
    p.hip = vec2f(0.15, -0.2);
    p.knee = vec2f(0.25, 0.6);
    p.sh = vec2f(1.9, 0.6);
    p.el = vec2f(0.6, 0.9);
    p.reach = vec2f(0.0);
    p.abd = vec2f(0.7, 0.9);
    p.head = 0.55;
    p.look = 0.9;
  } else if (pose == 23u) {
    // First aid (pose.rs `Pose::Aid`).
    p.hip = vec2f(1.55, -0.1);
    p.knee = vec2f(1.8, 1.75);
    p.lean = 0.4;
    p.sling = 1.0;
    let k = array<f32, 5>(0.0, 0.25, 0.5, 0.75, 1.0);
    let sh = keyed(phase, k, array<f32, 5>(0.85, 1.0, 0.85, 1.0, 0.85));
    let el = keyed(phase, k, array<f32, 5>(0.55, 0.3, 0.55, 0.3, 0.55));
    p.sh = vec2f(sh);
    p.el = vec2f(el);
    p.reach = vec2f(0.35);
    p.head = -0.55;
  } else if (pose == 12u || pose == 13u) {
    // Leaning out around a corner (pose.rs `Pose::LeanLeft` / `LeanRight`, LEAN_TILT 0.32).
    let g = select(-1.0, 1.0, pose == 13u);
    carry(&p, 4u);
    p.head = -0.35;
    p.tilt = 0.32 * g;
    p.side = vec2f(-0.06 + 0.04 * g, 0.06 + 0.04 * g);
    p.knee = vec2f(0.12);
    p.hip = vec2f(0.06);
  } else if (pose == 14u) {
    // Half crouch aiming over a low wall (pose.rs `Pose::CrouchAim`).
    crouch_gait(&p, 0.0, 0.15, 0.6, 0.0, 0.6, 0.0);
    p.flat = 1.0;
    p.lean = 0.15;
    carry(&p, 4u);
    p.head = -0.35;
  } else if (pose == 11u) {
    // Crouched walk (pose.rs `Pose::CrouchWalk`).
    crouch_gait(&p, phase, 0.3, 0.575, 0.12, 0.6, dir);
    p.flat = 1.0;
    p.lean = 0.3;
    carry(&p, 1u);
    p.head = 0.3;
    stride(&p, phase, 0.012, 0.05, 0.02, 0.02, 0.03);
  } else if (pose == 10u) {
    p.sh = vec2f(0.35);
    p.el = vec2f(1.5);
    p.reach = vec2f(-0.07);
    p.head = -0.45;
    p.sling = 1.0;
  } else if (pose == 31u) {
    // Hard stop (pose.rs `Pose::Brake`): left foot forward and wide, leaning back, rifle at the ready.
    p.hip = vec2f(0.5, -0.3);
    p.knee = vec2f(0.25, 0.3);
    p.side = vec2f(-0.2, 0.04);
    p.flat = 1.0;
    carry(&p, 6u);
    p.lean = -0.12;
    p.head = 0.12;
  } else if (pose == 32u) {
    // Direction-aware low crawl (pose.rs `Pose::Crawl`): reverse the diagonal pull when backing up and
    // change the support width for either side-crawl.
    let s = sin(phase * 6.2831853 + dir);
    let lateral = sin(dir);
    let draw = vec2f(max(s, 0.0), max(-s, 0.0));
    p.prone = 1.0;
    p.sh = vec2f(2.75 - 0.3 * s, 2.75 + 0.3 * s);
    p.el = vec2f(1.4 + 0.3 * s, 1.4 - 0.3 * s);
    p.hip = vec2f(0.1) + 0.3 * draw;
    p.knee = vec2f(0.35) + 1.2 * draw;
    p.side = vec2f(-0.12 - 0.55 * draw.x - 0.16 * lateral, 0.12 + 0.55 * draw.y + 0.16 * lateral);
    p.abd = vec2f(0.08 - 0.12 * lateral, 0.08 + 0.12 * lateral);
    p.twist = 0.06 * s + 0.08 * lateral;
    p.tilt = 0.05 * s + 0.04 * lateral;
    p.rifle = 1.5707963;
    p.head = 1.0;
  } else if (pose == 28u) {
    p.sh = vec2f(0.08);
    p.el = vec2f(0.3);
    p.sling = 1.0;
  } else if (pose == 9u) {
    carry(&p, 0u);
    p.sh.x = 1.45;
    p.el.x = 2.4;
    p.reach.x = 1.05;
    p.head = 0.1;
  }
  p.drop = ground_drop(p.hip, p.knee, p.side, p.flat);
  return p;
}

fn ground_drop(hip: vec2f, knee: vec2f, side: vec2f, flat: f32) -> f32 {
  var low = 1e9;
  for (var i = 0; i < 2; i++) {
    let px = select(0.1, -0.1, i == 0);
    low = min(low, sideways(foot(vec3f(px, -0.09, 0.0), hip[i], knee[i], flat), side[i], px).z);
    low = min(low, sideways(foot(vec3f(px, 0.14, 0.0), hip[i], knee[i], flat), side[i], px).z);
  }
  return -low;
}

fn blend(a: PP, b: PP, w0: f32) -> PP {
  // Eased (smoothstep), like pose.rs.
  let w = w0 * w0 * (3.0 - 2.0 * w0);
  var p = PP(mix(a.lean, b.lean, w), mix(a.hip, b.hip, w), mix(a.knee, b.knee, w), mix(a.sh, b.sh, w),
             mix(a.el, b.el, w), mix(a.head, b.head, w), mix(a.rifle, b.rifle, w), mix(a.prone, b.prone, w), 0.0,
             mix(a.reach, b.reach, w), mix(a.sling, b.sling, w), mix(a.bob, b.bob, w), mix(a.twist, b.twist, w),
             mix(a.look, b.look, w), mix(a.abd, b.abd, w), mix(a.ryaw, b.ryaw, w), vec2f(0.0), 0.0, mix(a.tilt, b.tilt, w));
  p.side = mix(a.side, b.side, w);
  p.flat = mix(a.flat, b.flat, w);
  p.drop = select(select(ground_drop(p.hip, p.knee, p.side, p.flat), b.drop, w >= 1.0), a.drop, w <= 0.0);
  return p;
}


// A rifle point held in the right palm (port of pose.rs `held`): pitch around the grip, turn around the
// vertical through it, then into the palm.
fn held(v: vec3f, p: PP) -> vec3f {
  let hand = arm(vec3f(0.18, 0.0, 0.80), p.sh.y, p.el.y, p.reach.y, p.abd.y, 0.238);
  let d = rot(v, 0.03, 0.78, p.rifle) - vec3f(0.238, 0.03, 0.78);
  let s = sin(p.ryaw);
  let c = cos(p.ryaw);
  return hand + vec3f(d.x * c - d.y * s, d.x * s + d.y * c, d.z);
}
// Left palm and the rifle's handguard (ports of pose.rs `hand` and `held(RIFLE_GUARD)`).
fn hand_l(p: PP) -> vec3f { return arm(vec3f(-0.18, 0.0, 0.80), p.sh.x, p.el.x, p.reach.x, p.abd.x, -0.238); }
fn guard(p: PP) -> vec3f { return held(vec3f(0.238, 0.34, 0.82), p); }
fn two_handed(pose: u32) -> f32 { return select(0.0, 1.0, pose <= 4u || pose == 6u || pose == 7u || pose == 8u || (pose >= 11u && pose <= 16u) || pose == 30u || pose == 31u); }
// Port of pose.rs `grip_weight` (GRIP_SWAP 0.6).
fn grip_weight(a: u32, b: u32, w0: f32) -> f32 {
  let ha = two_handed(a);
  let hb = two_handed(b);
  var w = clamp((w0 - 0.4) / 0.6, 0.0, 1.0);
  if (ha > hb) { w = clamp(w0 / 0.6, 0.0, 1.0); }
  w = w * w * (3.0 - 2.0 * w);
  return mix(ha, hb, w);
}
// Port of pose.rs `reach_pitch` (GRIP_REACH 0.595).
fn reach_pitch(p: PP) -> f32 {
  // In the rifle's turned frame, where pitch is a turn around x.
  let h = arm(vec3f(0.18, 0.0, 0.80), p.sh.y, p.el.y, p.reach.y, p.abd.y, 0.238);
  let v = rot(vec3f(0.238, 0.34, 0.82), 0.03, 0.78, p.rifle) - vec3f(0.238, 0.03, 0.78);
  let a0 = h - vec3f(-0.238, 0.0, 1.40);
  let sy = sin(-p.ryaw);
  let cy = cos(-p.ryaw);
  let a = vec3f(a0.x * cy - a0.y * sy, a0.x * sy + a0.y * cy, a0.z);
  let k = dot(a, a) + dot(v, v) + 2.0 * a.x * v.x;
  let r2 = 0.595 * 0.595;
  if (k + 2.0 * (a.y * v.y + a.z * v.z) <= r2) { return 0.0; }
  let ra = length(a.yz);
  let rv = length(v.yz);
  if (ra * rv < 1e-6) { return 0.0; }
  let phi = atan2(v.z, v.y) - atan2(a.z, a.y);
  let c = clamp((r2 - k) / (2.0 * ra * rv), -1.0, 1.0);
  let base = acos(c);
  let tau = 6.2831853;
  let d1 = (base - phi + 3.1415927) - floor((base - phi + 3.1415927) / tau) * tau - 3.1415927;
  let d2 = (-base - phi + 3.1415927) - floor((-base - phi + 3.1415927) / tau) * tau - 3.1415927;
  let q1 = d1 * d1;
  let q2 = d2 * d2;
  if (q1 + q2 < 1e-9) { return 0.0; }
  return (d1 * q2 + d2 * q1) / (q1 + q2);
}
// A value through five keyframes (port of pose.rs `keyed`): one-shot actions.
fn keyed(t_in: f32, k_in: array<f32, 5>, v_in: array<f32, 5>) -> f32 {
  var k = k_in;
  var v = v_in;
  let t = clamp(t_in, 0.0, 1.0);
  for (var i = 0; i < 4; i++) {
    if (t <= k[i + 1]) {
      let u = clamp((t - k[i]) / max(k[i + 1] - k[i], 1e-6), 0.0, 1.0);
      return v[i] + (v[i + 1] - v[i]) * u * u * (3.0 - 2.0 * u);
    }
  }
  return v[4];
}
fn ease(a: vec3f, b: vec3f, u_in: f32) -> vec3f { let u = clamp(u_in, 0.0, 1.0); return mix(a, b, u * u * (3.0 - 2.0 * u)); }
// Port of pose.rs `reload_target`: handguard, magazine well (RIFLE_WELL), chest pouch (RELOAD_POUCH), and back.
fn reload_target(p: PP, t: f32) -> vec3f {
  let g = guard(p);
  let well = held(vec3f(0.238, 0.11, 0.70), p);
  let pouch = vec3f(-0.04, 0.17, 1.08);
  if (t >= 0.75) { return ease(well, g, (t - 0.75) / 0.25); }
  if (t >= 0.55) { return ease(pouch, well, (t - 0.55) / 0.2); }
  if (t >= 0.4) { return pouch; }
  if (t >= 0.15) { return ease(well, pouch, (t - 0.15) / 0.25); }
  return ease(g, well, t / 0.15);
}
fn grip_try(p: PP, x: vec4f) -> PP {
  var t = p;
  t.sh.x = t.sh.x + x.x; t.el.x = t.el.x + x.y; t.reach.x = t.reach.x + x.z; t.rifle = t.rifle + x.w;
  return t;
}
// Port of pose.rs `grip`: puts the left palm back on the handguard (GRIP_TILT 0.6, GRIP_DAMP 0.05,
// GRIP_STEPS 6, GRIP_MAX 0.7; GRIP_RIFLE_COST 0 so the rifle unknown never moves).
fn grip(p_in: PP, w_in: f32) -> PP {
  let w = w_in * (1.0 - p_in.sling);
  if (w <= 0.0) { return p_in; }
  var p = p_in;
  p.rifle = p.rifle + reach_pitch(p_in) * 0.6 * w;
  return reach_left(p, guard(p), w, 0.7);
}

// Port of pose.rs `reach_left_capped`: the left palm onto `goal`, each joint's correction softly capped at
// `cap` (0.7 for the handguard, REACH_MAX 2.0 for actions).
fn reach_left(p: PP, goal: vec3f, w: f32, cap: f32) -> PP {
  if (w <= 0.0) { return p; }
  let excess = max(distance(goal, vec3f(-0.238, 0.0, 1.40)) - 0.595, 0.0);
  let damp = 0.05 + 2.0 * excess;
  var q = p;
  for (var it = 0; it < 6; it = it + 1) {
    let r = goal - hand_l(q);
    if (dot(r, r) < 1e-6) { break; }
    let d = 0.01;
    let j0 = (goal - hand_l(grip_try(q, vec4f(d, 0.0, 0.0, 0.0))) - r) / d;
    let j1 = (goal - hand_l(grip_try(q, vec4f(0.0, d, 0.0, 0.0))) - r) / d;
    let j2 = (goal - hand_l(grip_try(q, vec4f(0.0, 0.0, d, 0.0))) - r) / d;
    let jm = mat3x3f(j0, j1, j2);
    let m = jm * transpose(jm) + mat3x3f(damp * damp, 0.0, 0.0, 0.0, damp * damp, 0.0, 0.0, 0.0, damp * damp);
    let det = determinant(m);
    if (abs(det) < 1e-9) { break; }
    // Inverse by cofactors (columns of m are c0, c1, c2).
    let c0 = m[0]; let c1 = m[1]; let c2 = m[2];
    let inv = transpose(mat3x3f(cross(c1, c2), cross(c2, c0), cross(c0, c1))) * (1.0 / det);
    let y = inv * r;
    let x = clamp(-vec3f(dot(j0, y), dot(j1, y), dot(j2, y)), vec3f(-0.5), vec3f(0.5));
    q = grip_try(q, vec4f(x, 0.0));
  }
  var out = p;
  out.sh.x = out.sh.x + cap * tanh((q.sh.x - p.sh.x) / cap) * w;
  out.el.x = out.el.x + cap * tanh((q.el.x - p.el.x) / cap) * w;
  out.reach.x = out.reach.x + cap * tanh((q.reach.x - p.reach.x) / cap) * w;
  return out;
}

fn pose_vertex(v: vec3f, part: u32, p: PP) -> vec3f {
  var q = v;
  if (part == 1u) { q = sideways(rot(v, 0.0, 0.88, p.hip.x), p.side.x, -0.1); }
  else if (part == 2u) { q = sideways(rot(v, 0.0, 0.88, p.hip.y), p.side.y, 0.1); }
  else if (part == 6u) { q = sideways(leg(v, p.hip.x, p.knee.x), p.side.x, -0.1); }
  else if (part == 7u) { q = sideways(leg(v, p.hip.y, p.knee.y), p.side.y, 0.1); }
  else if (part == 13u) { q = sideways(foot(v, p.hip.x, p.knee.x, p.flat), p.side.x, -0.1); }
  else if (part == 14u) { q = sideways(foot(v, p.hip.y, p.knee.y, p.flat), p.side.y, 0.1); }
  else if (part == 3u) { q = arm(v, p.sh.x, 0.0, p.reach.x, p.abd.x, -0.238); }
  else if (part == 4u) { q = arm(v, p.sh.y, 0.0, p.reach.y, p.abd.y, 0.238); }
  else if (part == 11u) { q = arm(v, p.sh.x, p.el.x, p.reach.x, p.abd.x, -0.238); }
  else if (part == 12u) { q = arm(v, p.sh.y, p.el.y, p.reach.y, p.abd.y, 0.238); }
  else if (part == 10u) { q = turn(rot(v, 0.0, 1.46, p.head), p.look); }
  else if (part == 9u) {
    // Pistol grip (pose.rs RIFLE_GRIP) in the right palm (PALM).
    q = held(v, p);
    if (p.sling > 0.0) { q = mix(q, slung(v), p.sling); }
  }
  if (part != 1u && part != 2u && part != 6u && part != 7u && part != 13u && part != 14u) {
    q = rot(turn(q, p.twist), 0.0, 0.88, -p.lean);
    // Sideways lean around the hips (pose.rs `tilt`), positive to +x.
    let ts = sin(p.tilt);
    let tc = cos(p.tilt);
    let tz = q.z - 0.88;
    q = vec3f(q.x * tc + tz * ts, q.y, 0.88 - q.x * ts + tz * tc);
  }
  q.z = q.z + p.drop + p.bob;
  if (p.prone != 0.0) {
    // Positive onto his front, negative onto his back (pose.rs PRONE_LIFT 0.23, SUPINE_LIFT 0.16).
    q = rot(q, 0.0, 0.0, -1.5707963 * p.prone);
    let ap = abs(p.prone);
    q.z = q.z + select(0.16, 0.23, p.prone > 0.0) * ap + 0.12 * sin(3.1415927 * ap);
  }
  return q;
}

fn animate(v: vec3f, c: f32, a: vec4f, b: vec4f, d: vec4f, time: f32, hands: bool) -> vec3f {
  var q = v;
  let part = select(u32(floor(c / 32.0)), 8u, c < 0.0);
  if (part == 5u) {
    // Wind: a slow gust that travels across the field, scaled by height so the roots stay put.
    let gust = sin(time * 1.7 + a.x * 0.35 + a.y * 0.21 + b.y * 6.28) * 0.5 + 0.5;
    q = q + vec3f(0.09, 0.04, 0.0) * gust * q.z * 2.5;
  } else if (part != 0u && part != 15u) {
    // Negative b.z keeps the CPU-integrated leg phase in b.y. Values below -1 also carry reload progress.
    let phase = select(select(0.0, fract(time * b.z + b.y), b.z > 0.0), fract(b.y), b.z < 0.0);
    let reload_phase = clamp(-b.z - 1.0, 0.0, 1.0);
    let w = clamp((time - d.y) / 0.35, 0.0, 1.0);
    // Instance c.z: upper-body pose id + 1, with movement direction in its fractional part.
    let layer = floor(d.z);
    let turn = fract(d.z) * 6.2831853;
    let dir = select(turn, turn - 6.2831853, turn > 3.1415927);
    let base_phase = select(phase, reload_phase, u32(b.w) == 18u);
    var pp = pose_params(u32(b.w), base_phase, dir);
    if (w < 1.0) {
      let previous_phase = select(phase, reload_phase, u32(d.x) == 18u);
      pp = blend(pose_params(u32(d.x), previous_phase, dir), pp, w);
    }
    let base = pp;
    var upper_id = 0u;
    if (layer >= 1.0) {
      upper_id = u32(layer - 1.0);
      let upper_phase = select(phase, reload_phase, upper_id == 18u);
      pp = upper_over(base, pose_params(upper_id, upper_phase, 0.0));
    }
    // Blend the upper body it is leaving so raising/lowering the rifle never pops.
    let lb = layer_back(d.w);
    if (lb.y > 0.0) {
      var was = base;
      if (lb.x >= 1.0) {
        let previous_upper_id = u32(lb.x - 1.0);
        let previous_upper_phase = select(phase, reload_phase, previous_upper_id == 18u);
        was = upper_over(base, pose_params(previous_upper_id, previous_upper_phase, 0.0));
      }
      pp = blend(was, pp, 1.0 - lb.y);
    }
    // Standing still: idle life, each soldier out of step with the next.
    if (b.z == 0.0) { pp = idle(pp, u32(b.w), time + b.y * 97.0, b.y); }
    // Recoil pressure, carried in the pose id's fraction.
    let kick = fract(b.w);
    if (kick > 0.0) {
      pp.rifle = pp.rifle + 0.024 * kick * kick;
      pp.sh = pp.sh - vec2f(0.09 * kick);
      pp.lean = pp.lean - 0.02 * kick;
    }
    // Flinch, carried in the previous pose id's fraction.
    let fl = fract(d.x);
    if (fl > 0.0) {
      pp.head = pp.head - 0.35 * fl;
      pp.sh = pp.sh - vec2f(0.06 * fl);
      if (pp.prone == 0.0) {
        pp.lean = pp.lean + 0.18 * fl;
        pp.hip = pp.hip + vec2f(0.2 * fl);
        pp.knee = pp.knee + vec2f(0.35 * fl);
        pp.drop = ground_drop(pp.hip, pp.knee, pp.side, pp.flat);
      }
    }
    // Left palm onto the handguard or, while reloading, along the magazine path.
    // The shadow pass skips this: a few centimetres in an arm's shadow cannot be seen.
    if (hands && (part == 3u || part == 11u || part == 9u)) {
      let hold = select(two_handed(u32(b.w)), two_handed(upper_id), layer >= 1.0);
      let gw = select(hold, grip_weight(u32(d.x), u32(b.w), w), w < 1.0 && layer < 1.0);
      pp = grip(pp, gw);
      var reload_weight = 0.0;
      if (layer < 1.0) {
        if (u32(b.w) == 18u) {
          reload_weight = w;
        } else if (u32(d.x) == 18u && w < 1.0) {
          reload_weight = 1.0 - w;
        }
      } else if (upper_id == 18u) {
        reload_weight = select(1.0, 1.0 - lb.y, lb.y > 0.0);
      }
      let current_upper_reload = layer >= 1.0 && upper_id == 18u;
      if (lb.x == 19.0 && lb.y > 0.0 && !current_upper_reload) {
        reload_weight = lb.y;
      }
      if (reload_weight > 0.0) {
        pp = reach_left(pp, reload_target(pp, reload_phase), reload_weight, 2.0);
      }
    }
    q = pose_vertex(v, part, pp);
    // Banking into a turn and leaning with acceleration.
    let k = tilt_of(a);
    if (k.x != 0.0 || k.y != 0.0) {
      let cb = cos(k.x);
      let sb = sin(k.x);
      q = vec3f(q.x * cb - q.z * sb, q.y, q.x * sb + q.z * cb);
      let cp = cos(k.y);
      let sp = sin(k.y);
      q = vec3f(q.x, q.y * cp + q.z * sp, -q.y * sp + q.z * cp);
    }
  }
  let ch = cos(b.x);
  let sh = sin(b.x);
  return vec3f(q.x * ch - q.y * sh, q.x * sh + q.y * ch, q.z) + a.xyz;
}

// An upper-body stance laid over the legs' pose (pose.rs `layer`).
fn upper_over(legs: PP, up: PP) -> PP {
  var pp = legs;
  pp.lean = up.lean; pp.sh = up.sh; pp.el = up.el; pp.reach = up.reach; pp.rifle = up.rifle; pp.ryaw = up.ryaw; pp.tilt = up.tilt; pp.twist = legs.twist + up.twist; pp.head = up.head; pp.sling = up.sling; pp.look = up.look; pp.abd = up.abd;
  return pp;
}

// Instance c.w (pose.rs `pack_fade_layer`): below 1 it is only the dissolve (0 solid, 1 gone); from 1 up, the
// whole part also carries the upper body being left (id + 1, 0 = the legs' own) and how much of it remains
// (x: code, y: 0..1), the fraction the dissolve.
fn fade_of(w: f32) -> f32 { return select(w, fract(w), w >= 1.0); }
fn layer_back(w: f32) -> vec2f {
  if (w < 1.0) { return vec2f(0.0); }
  let whole = floor(w);
  let code = floor(whole / 64.0);
  return vec2f(code, (whole - code * 64.0) / 63.0);
}

// Instance colour a.w, fractional part: bank into a turn and lean with acceleration (pose.rs `pack_tilt`; no
// fraction = upright). Returns (roll, pitch) in radians.
fn tilt_of(a: vec4f) -> vec2f {
  let f = fract(a.w);
  if (f < 0.000001) { return vec2f(0.0); }
  let n = i32(round(f * 16385.0)) - 1;
  let r = n / 128;
  let p = n - r * 128;
  return vec2f(f32(r - 64) / 63.0 * 0.26, f32(p - 64) / 63.0 * 0.2);
}

// Instance colour a.w: palette index, +32 for a squad leader, +64 per selection level (1 stable, 2 hurt, 3 killed).
fn select_level(a: vec4f) -> f32 {
  return select(0.0, floor(a.w / 64.0), a.w >= 64.0);
}
fn palette_index(c: f32, a0: vec4f) -> u32 {
  var a = a0;
  a.w = a.w - 64.0 * select_level(a0);
  // A squad leader (instance colour flagged +32): his helmet is drawn as a red beret.
  if (a.w >= 32.0 && c >= 0.0 && (c - 32.0 * floor(c / 32.0)) == 26.0) { return 9u; }
  // Negative codes and swaying parts (grass) take the instance colour.
  // Negative codes, swaying parts (grass) and palette 31 on any part take the instance colour.
  let instance_colour = c < 0.0 || floor(c / 32.0) == 5.0 || (c - 32.0 * floor(c / 32.0)) == 31.0;
  return u32(max(select(c - 32.0 * floor(c / 32.0), a.w, instance_colour), 0.0));
}

// Bends the flat wrapping map into a globe (port of `bend.rs`): b = (target x, target y, planet radius, amount
// 0..1), lat_y = the target's absolute map y. The map lies on a sphere of radius R / amount touching the target.
fn bend_point(p: vec3f, b: vec4f, lat_y: f32) -> vec3f {
  if (b.w <= 0.0) { return p; }
  let r = b.z / min(b.w, 1.0);
  let rr = r + p.z;
  let lon = (p.x - b.x) / r;
  let dlat = (p.y - b.y) / r;
  let lat0 = lat_y / r;
  let lat = lat0 + dlat;
  let shl = sin(lon * 0.5);
  let sha = sin(dlat * 0.5);
  let cl = cos(lat);
  let vers = 2.0 * shl * shl;
  return vec3f(rr * cl * sin(lon) + b.x,
               rr * (sin(dlat) + cl * sin(lat0) * vers) + b.y,
               p.z - rr * (2.0 * sha * sha + cl * cos(lat0) * vers));
}
