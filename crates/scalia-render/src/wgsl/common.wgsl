// Point lights (muzzle flashes, blasts): lcount.x of them; pl = (x, y, z, radius m), plc = (rgb, intensity).
struct U { m: mat4x4f, lm: mat4x4f, vdir: vec4f, pal: array<vec4f, 32>, light: vec4f, light_col: vec4f, ambient: vec4f, time: vec4f, bend: vec4f, bend2: vec4f, lcount: vec4f, pl: array<vec4f, 16>, plc: array<vec4f, 16> };

// Instances: a = (x, y, z, palette for the body), b = (heading, phase offset, steps per second, pose id),
// c = (previous pose id, time of the change in seconds, unused, unused): poses blend over 0.35 s.
// A figure faces +y; heading turns it around z. Poses port `pose.rs` (the tested reference) exactly.

struct PP { lean: f32, hip: vec2f, knee: vec2f, sh: vec2f, el: vec2f, head: f32, rifle: f32, prone: f32, drop: f32, reach: vec2f, sling: f32, bob: f32, twist: f32, look: f32, abd: vec2f, ryaw: f32, side: vec2f, flat: f32, tilt: f32, yaw: f32, raise: f32, roll: vec2f, toe: vec2f };

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

// prone points the foot back, boots flat, only once settled (pose.rs PRONE_POINT -1.2, prone_settle).
// toe: the boot's roll (pose.rs PoseParams::toe).
fn foot(q: vec3f, hip: f32, knee: f32, flat: f32, prone: f32, toe: f32) -> vec3f {
  return leg(rot(q, 0.0, 0.085, ankle_flat(hip, knee, flat) - 1.2 * pow(max(prone, 0.0), 12.0) + toe), hip, knee);
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
// Leg roll lying prone (pose.rs leg_roll_angle, AN-02-S10): up to a quarter turn about the leg's own axis as the
// knee draws from 0.1 to 0.4, only once settled prone; inward, so the foot tucks behind the knee (AN-02-S11).
fn leg_roll(q: vec3f, p: PP, knee: f32, px: f32) -> vec3f {
  let t = clamp((knee - 0.2) / 0.2, 0.0, 1.0); // pose.rs LEG_ROLL_FROM, LEG_ROLL_FULL
  // Only lying flat: raised at all (the crawl solves its own leg roll) it is gone (pose.rs LEG_ROLL_RAISE 0.025).
  let flat = 1.0 - clamp(p.raise / 0.025, 0.0, 1.0);
  let a = 1.5707963 * pow(max(p.prone, 0.0), 12.0) * t * t * (3.0 - 2.0 * t) * flat * -sign(px) + select(p.roll.y, p.roll.x, px < 0.0);
  if (a == 0.0) { return q; }
  let s = sin(a);
  let c = cos(a);
  let dx = q.x - px;
  return vec3f(px + dx * c - q.y * s, dx * s + q.y * c, q.z);
}

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
// AN-02-S13 baked rifle gaits (pose.rs `gait_bake`/`apply_gait`): the 8 direction loops of a gait sit back to
// back in the clip buffer from `at0` (WALK_F_AT, RUN_F_AT, SPRINT_F_AT), WALK_F_N rows of 11 fields.
// Returns the body lift (the drop never goes below it).
// pose.rs `SPRINT_TWIST`: the part of the baked sprint's torso twist that is kept.
const SPRINT_TWIST: f32 = 0.8;
// pose.rs `RUN_TILT`: the part of the baked run and sprint upper-body side lean that is kept.
const RUN_TILT: f32 = 0.4;
fn bake_gait(p: ptr<function, PP>, at0: u32, phase: f32, dir: f32, twist_k: f32, tilt_k: f32) -> f32 {
  let x = (dir - floor(dir / 6.2831853) * 6.2831853) / 0.78539816;
  let i = u32(floor(x)) % 8u;
  let w = x - floor(x);
  let n = WALK_F_N;
  let a = clip_pos(at0 + i * 11u * n, 11u, n, phase);
  let b = clip_pos(at0 + ((i + 1u) % 8u) * 11u * n, 11u, n, phase);
  let g = gait_mix(a, b, w);
  // Patch: Animations 2: the boots' roll for these loops (pose.rs `gait_toe`, GAIT_TOE_AT).
  let gs = (at0 - WALK_F_AT) / (8u * 11u * n);
  // Four fields a row: boot roll left, right, then each leg's heading turn (roll) left, right.
  let ta = clip_pos(GAIT_TOE_AT + (gs * 8u + i) * 4u * n, 4u, n, phase);
  let tb = clip_pos(GAIT_TOE_AT + (gs * 8u + (i + 1u) % 8u) * 4u * n, 4u, n, phase);
  (*p).toe = mix(vec2f(clip_get(ta, 0u), clip_get(ta, 1u)), vec2f(clip_get(tb, 0u), clip_get(tb, 1u)), w);
  (*p).roll = mix(vec2f(clip_get(ta, 2u), clip_get(ta, 3u)), vec2f(clip_get(tb, 2u), clip_get(tb, 3u)), w);
  (*p).hip = vec2f(g[0], g[1]);
  (*p).knee = vec2f(g[2], g[3]);
  (*p).side = vec2f(g[4], g[5]);
  (*p).yaw = g[6];
  (*p).lean = (*p).lean + g[7];
  let twist = g[8] * twist_k;
  (*p).look = (*p).look + 0.8 * ((*p).twist - twist);
  (*p).twist = twist;
  (*p).tilt = (*p).tilt + g[9] * tilt_k;
  (*p).bob = 0.0;
  (*p).flat = 1.0;
  return g[10];
}
fn gait_mix(a: ClipPos, b: ClipPos, w: f32) -> array<f32, 11> {
  var o: array<f32, 11>;
  for (var k = 0u; k < 11u; k++) { o[k] = mix(clip_get(a, k), clip_get(b, k), w); }
  return o;
}

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
const DROP_REACH_P: PP = PP(1.550000, vec2f(1.550000, -0.100000), vec2f(1.800000, 1.750000), vec2f(1.000000, -0.090000), vec2f(0.100000, 1.920000), 0.900000, 0.900000, 0.000000, -0.390930, vec2f(0.100000, 0.870000), 0.000000, 0.000000, 0.000000, 0.000000, vec2f(0.150000, 0.530000), 0.740000, vec2f(0.000000, 0.000000), 0.000000, 0.000000, 0.000000, 0.000000, vec2f(0.000000, 0.000000), vec2f(0.000000, 0.000000));
const DROP_PLANT_P: PP = PP(1.100000, vec2f(0.900000, 0.900000), vec2f(1.350000, 1.350000), vec2f(0.900000, 0.600000), vec2f(0.500000, 2.200000), 0.600000, 0.000000, 0.650000, -0.143823, vec2f(0.100000, 0.245123), 0.000000, 0.000000, 0.000000, 0.000000, vec2f(0.150000, 0.149328), 1.500000, vec2f(0.000000, 0.000000), 0.000000, 0.000000, 0.000000, 0.000000, vec2f(0.000000, 0.000000), vec2f(0.000000, 0.000000));

fn pose_params(pose: u32, phase: f32, dir: f32) -> PP {
  if (pose == 34u) { return DROP_REACH_P; }
  if (pose == 35u) { return DROP_PLANT_P; }
  let sw = sin(phase * 6.2831853);
  var p = PP(0.0, vec2f(0.0), vec2f(0.0), vec2f(0.0), vec2f(0.0), 0.0, 0.0, 0.0, 0.0, vec2f(0.0), 0.0, 0.0, 0.0, 0.0, vec2f(0.0), 0.0, vec2f(0.0), 0.0, 0.0, 0.0, 0.0, vec2f(0.0), vec2f(0.0));
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
  } else if (pose == 5u || pose == 36u) {
    // Prone ready and aim share the same legs; grip() solves the left support arm in both.
    p.prone = 1.0;
    p.hip = vec2f(0.18, 0.16);
    p.knee = vec2f(0.06, 0.04);
    p.rifle = 1.5707963;
    p.ryaw = 0.55;
    p.twist = -0.55;
    // Chest up on the elbows, hips flat (pose.rs PRONE_LEAN -0.28, PRONE_AIM_LEAN -0.30); rifle and head take back
    // the extra lean over the old -0.1.
    p.lean = select(-0.30, -0.28, pose == 36u);
    let arch = p.lean + 0.1;
    p.rifle = p.rifle + arch;
    p.look = 0.55;
    if (pose == 36u) {
      // Aiming: shouldered, pointed ahead (pose.rs).
      p.sh = vec2f(2.8, 1.571);
      p.el = vec2f(1.3, 2.084);
      p.reach = vec2f(0.0, 0.401);
      p.abd = vec2f(0.0, 0.604);
      p.head = 1.0 + arch;
    } else {
      // At rest: elbows down, forearms ahead, the rifle across in front (pose.rs).
      p.rifle = 0.0;
      p.ryaw = 1.5707963;
      p.twist = 0.0;
      p.look = 0.0;
      p.sh = vec2f(1.75);
      p.el = vec2f(1.45);
      p.head = 0.8 + arch;
    }
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
    // Patch: Animations 3 low crawl (mirrors pose.rs): body flat on the ground, rifle cradled
    // across the forearms. Backwards plays the loop in reverse; sideways leans the knees and chest to the side.
    let fwd = cos(dir);
    let lateral = sin(dir);
    let wb = clamp((-0.2 - fwd) * 5.0, 0.0, 1.0);
    let ca = clip_pos(HIGH_CRAWL_AT, HIGH_CRAWL_F, HIGH_CRAWL_N, phase);
    let cb = clip_pos(HIGH_CRAWL_AT, HIGH_CRAWL_F, HIGH_CRAWL_N, -phase);
    var r: array<f32, 23>;
    for (var k = 0u; k < 23u; k = k + 1u) { r[k] = mix(clip_get(ca, k), clip_get(cb, k), wb); }
    // Patch: Animations 5: the legs' roll fades out crawling sideways (pose.rs).
    r[18] = r[18] * abs(fwd);
    p.prone = 1.0;
    p.hip = vec2f(r[0], r[1]);
    p.knee = vec2f(r[2], r[3]);
    p.side = vec2f(r[4], r[5]) + vec2f(0.3 * lateral);
    p.sh = vec2f(r[6], r[7]);
    p.el = vec2f(r[8], r[9]);
    p.abd = vec2f(r[10], r[11]);
    p.reach = vec2f(r[12], r[13]);
    p.lean = r[14];
    p.raise = r[15];
    p.twist = r[16];
    p.head = r[17];
    p.yaw = r[18];
    p.roll = vec2f(r[19], r[20]);
    // Patch: Animations 5: the spine bends toward the drawn knee; the head takes back most of the roll.
    p.tilt = r[21];
    p.look = r[22] - 0.4 * lateral;
    // The rifle along the right forearm, muzzle ahead, on its side (crawl.rs RIFLE_PITCH 1.5707963, RIFLE_ROLL -1.5707963).
    p.rifle = 1.5707963;
    p.ryaw = -1.5707963;
  }

 else if (pose == 28u) {
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
  // AN-02-S13: the rifle gaits follow their baked loops (pose.rs `params_dir`).
  var lift = -1e9;
  if (pose == 1u) { lift = bake_gait(&p, WALK_F_AT, phase, dir, 1.0, 1.0); }
  else if (pose == 2u) { lift = bake_gait(&p, RUN_F_AT, phase, dir, 1.0, RUN_TILT); }
  else if (pose == 7u) { lift = bake_gait(&p, SPRINT_F_AT, phase, dir, SPRINT_TWIST, RUN_TILT); }
  p.drop = max(ground_drop(p.hip, p.knee, p.side, p.flat, p.toe), lift);
  // The crawl keeps the prone legs' grounding offset (pose.rs): lying down it runs fore and aft.
  clear_arms(&p);
  if (pose == 32u) { p.drop = ground_drop(vec2f(0.18, 0.16), vec2f(0.06, 0.04), vec2f(0.0), 0.0, vec2f(0.0)); }
  return p;
}

// Arms clear of the body (pose.rs `clear_arms`, TORSO_RX 0.18 0.22, TORSO_RY 0.16, TORSO_Z 0.85 1.30, ARM_R 0.05):
// an arm whose elbow, forearm or hand would sink into the torso swings out just enough. Upright poses only.
fn clear_arms(p: ptr<function, PP>) {
  if ((*p).prone != 0.0) { return; }
  for (var i = 0; i < 2; i++) {
    // The rifle arm moves only with the rifle slung (its stock sits in the shoulder).
    if (i == 1 && (*p).sling < 1.0) { continue; }
    let g = select(1.0, -1.0, i == 0);
    for (var it = 0; it < 2; it++) {
      var need = 0.0;
      for (var j = 0; j < 3; j++) {
        var q = vec3f(g * 0.18, 0.0, 1.09);
        if (j == 1) { q = vec3f(g * 0.18, 0.0, 0.945); }
        if (j == 2) { q = vec3f(g * 0.18, 0.0, 0.80); }
        let w = arm(q, (*p).sh[i], (*p).el[i], (*p).reach[i], (*p).abd[i], g * 0.238);
        if (w.z < 0.80 || w.z > 1.30) { continue; }
        let t = clamp((w.z - 0.85) / 0.45, 0.0, 1.0);
        let rx = 0.18 + 0.04 * t * t * (3.0 - 2.0 * t) + 0.05;
        let yy = w.y / 0.21;
        if (abs(yy) >= 1.0) { continue; }
        let out = rx * sqrt(1.0 - yy * yy) - w.x * g;
        if (out > 0.0) { need = max(need, out / max(1.40 - w.z, 0.15)); }
      }
      (*p).abd[i] = (*p).abd[i] + need;
    }
  }
}

// Port of pose.rs `ground_drop_toe` (boots rolled by `toe`).
fn ground_drop(hip: vec2f, knee: vec2f, side: vec2f, flat: f32, toe: vec2f) -> f32 {
  var low = 1e9;
  for (var i = 0; i < 2; i++) {
    let px = select(0.1, -0.1, i == 0);
    low = min(low, sideways(foot(vec3f(px, -0.09, 0.0), hip[i], knee[i], flat, 0.0, toe[i]), side[i], px).z);
    low = min(low, sideways(foot(vec3f(px, 0.14, 0.0), hip[i], knee[i], flat, 0.0, toe[i]), side[i], px).z);
  }
  return -low;
}

fn blend(a: PP, b: PP, w0: f32) -> PP {
  // Eased (smoothstep), like pose.rs.
  let w = w0 * w0 * (3.0 - 2.0 * w0);
  var p = PP(mix(a.lean, b.lean, w), mix(a.hip, b.hip, w), mix(a.knee, b.knee, w), mix(a.sh, b.sh, w),
             mix(a.el, b.el, w), mix(a.head, b.head, w), mix(a.rifle, b.rifle, w), mix(a.prone, b.prone, w), 0.0,
             mix(a.reach, b.reach, w), mix(a.sling, b.sling, w), mix(a.bob, b.bob, w), mix(a.twist, b.twist, w),
             mix(a.look, b.look, w), mix(a.abd, b.abd, w), mix(a.ryaw, b.ryaw, w), vec2f(0.0), 0.0, mix(a.tilt, b.tilt, w), mix(a.yaw, b.yaw, w), mix(a.raise, b.raise, w), mix(a.roll, b.roll, w), mix(a.toe, b.toe, w));
  p.side = mix(a.side, b.side, w);
  p.flat = mix(a.flat, b.flat, w);
  p.drop = select(select(ground_drop(p.hip, p.knee, p.side, p.flat, p.toe), b.drop, w >= 1.0), a.drop, w <= 0.0);
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
fn two_handed(pose: u32) -> f32 { return select(0.0, 1.0, pose <= 8u || (pose >= 11u && pose <= 16u) || pose == 30u || pose == 31u || pose == 36u); }
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
  t.sh.x = t.sh.x + x.x; t.el.x = t.el.x + x.y; t.reach.x = t.reach.x + x.z; t.abd.x = t.abd.x + x.w;
  return t;
}
// Port of pose.rs `grip`: puts the left palm back on the handguard (GRIP_TILT 0.6, GRIP_DAMP 0.05,
// GRIP_STEPS 6, GRIP_MAX 0.7; GRIP_RIFLE_COST 0 so the rifle unknown never moves).
fn grip(p_in: PP, w_in: f32) -> PP {
  let w = w_in * (1.0 - p_in.sling);
  if (w <= 0.0) { return p_in; }
  var p = p_in;
  p.rifle = p.rifle + reach_pitch(p_in) * 0.6 * w;
  // Prone support reaches farther across the body than the standing carry.
  let cap = mix(0.7, 4.0, clamp(p.prone, 0.0, 1.0));
  return reach_left(p, guard(p), w, cap);
}

// Port of pose.rs `reach_left_capped`: the left palm onto `goal`, each joint's correction softly capped at
// `cap` (0.7 for the handguard, REACH_MAX 2.0 for actions).
fn reach_left(p: PP, goal: vec3f, w: f32, cap: f32) -> PP {
  if (w <= 0.0) { return p; }
  let excess = max(distance(goal, vec3f(-0.238, 0.0, 1.40)) - 0.595, 0.0);
  // Ease out reach damping as the torso lowers; prone support needs the arm's full extension.
  let damp = 0.05 + 2.0 * excess * (1.0 - clamp(p.prone, 0.0, 1.0));
  var q = p;
  for (var it = 0; it < 16; it = it + 1) {
    let r = goal - hand_l(q);
    if (dot(r, r) < 1e-6) { break; }
    let d = 0.01;
    let j0 = (goal - hand_l(grip_try(q, vec4f(d, 0.0, 0.0, 0.0))) - r) / d;
    let j1 = (goal - hand_l(grip_try(q, vec4f(0.0, d, 0.0, 0.0))) - r) / d;
    let j2 = (goal - hand_l(grip_try(q, vec4f(0.0, 0.0, d, 0.0))) - r) / d;
    let j3 = (goal - hand_l(grip_try(q, vec4f(0.0, 0.0, 0.0, d))) - r) / d;
    let jm = mat3x3f(j0, j1, j2);
    let m = jm * transpose(jm) + mat3x3f(j3 * j3.x, j3 * j3.y, j3 * j3.z)
      + mat3x3f(damp * damp, 0.0, 0.0, 0.0, damp * damp, 0.0, 0.0, 0.0, damp * damp);
    let det = determinant(m);
    if (abs(det) < 1e-9) { break; }
    // Inverse by cofactors (columns of m are c0, c1, c2).
    let c0 = m[0]; let c1 = m[1]; let c2 = m[2];
    let inv = transpose(mat3x3f(cross(c1, c2), cross(c2, c0), cross(c0, c1))) * (1.0 / det);
    let y = inv * r;
    let x = clamp(vec4f(-dot(j0, y), -dot(j1, y), -dot(j2, y), -dot(j3, y)), vec4f(-0.5), vec4f(0.5));
    q = grip_try(q, x);
  }
  var out = p;
  out.sh.x = out.sh.x + cap * tanh((q.sh.x - p.sh.x) / cap) * w;
  out.el.x = out.el.x + cap * tanh((q.el.x - p.el.x) / cap) * w;
  out.reach.x = out.reach.x + cap * tanh((q.reach.x - p.reach.x) / cap) * w;
  out.abd.x = out.abd.x + cap * tanh((q.abd.x - p.abd.x) / cap) * w;
  return out;
}

// Seams (pose.rs `bake_seams`, `seam_of`, `seam_joint`, `pose_mesh_vertex`): the soldier is one mesh cut into rigid
// parts; a vertex near a cut carries, in its colour code's fraction, the part across it and a blend (0.5 on the cut),
// and is turned part way from its own part's turn to the other's about the joint, so no cut opens and bent joints
// keep their thickness.
const SEAM_SCALE: f32 = 4096.0;
// The part across the cut (x, as f32) and the blend (y; 0 for none).
fn seam_of(c: f32) -> vec2f {
  if (c < 0.0) { return vec2f(0.0); }
  let packed = u32(round((c - floor(c)) * SEAM_SCALE));
  return vec2f(f32(packed >> 8u), f32(packed & 255u) / 254.0);
}
// The joint pivot (rest pose) between two parts in xyz; w = 0 when they share none.
fn seam_joint(p0: u32, p1: u32) -> vec4f {
  let a = min(p0, p1);
  let b = max(p0, p1);
  if (a == 6u && b == 13u) { return vec4f(-0.1, 0.0, 0.085, 1.0); }
  if (a == 7u && b == 14u) { return vec4f(0.1, 0.0, 0.085, 1.0); }
  if (a == 1u && b == 6u) { return vec4f(-0.1, 0.0, 0.46, 1.0); }
  if (a == 2u && b == 7u) { return vec4f(0.1, 0.0, 0.46, 1.0); }
  if (a == 1u && b == 8u) { return vec4f(-0.1, 0.0, 0.88, 1.0); }
  if (a == 2u && b == 8u) { return vec4f(0.1, 0.0, 0.88, 1.0); }
  if (a == 3u && b == 11u) { return vec4f(-0.238, 0.0, 1.09, 1.0); }
  if (a == 4u && b == 12u) { return vec4f(0.238, 0.0, 1.09, 1.0); }
  if (a == 3u && b == 8u) { return vec4f(-0.238, 0.0, 1.4, 1.0); }
  if (a == 4u && b == 8u) { return vec4f(0.238, 0.0, 1.4, 1.0); }
  if (a == 8u && b == 10u) { return vec4f(0.0, 0.0, 1.46, 1.0); }
  return vec4f(0.0);
}
// A rotation matrix's quaternion (xyz, w), as glam's Quat::from_mat3.
fn quat_of(m: mat3x3f) -> vec4f {
  if (m[2].z <= 0.0) {
    let dif10 = m[1].y - m[0].x;
    let omm22 = 1.0 - m[2].z;
    if (dif10 <= 0.0) {
      let f = omm22 - dif10; let i = 0.5 / sqrt(f);
      return vec4f(f * i, (m[0].y + m[1].x) * i, (m[0].z + m[2].x) * i, (m[1].z - m[2].y) * i);
    }
    let f = omm22 + dif10; let i = 0.5 / sqrt(f);
    return vec4f((m[0].y + m[1].x) * i, f * i, (m[1].z + m[2].y) * i, (m[2].x - m[0].z) * i);
  }
  let sum10 = m[1].y + m[0].x;
  let opm22 = 1.0 + m[2].z;
  if (sum10 <= 0.0) {
    let f = opm22 - sum10; let i = 0.5 / sqrt(f);
    return vec4f((m[0].z + m[2].x) * i, (m[1].z + m[2].y) * i, f * i, (m[0].y - m[1].x) * i);
  }
  let f = opm22 + sum10; let i = 0.5 / sqrt(f);
  return vec4f((m[1].z - m[2].y) * i, (m[2].x - m[0].z) * i, (m[0].y - m[1].x) * i, f * i);
}
fn quat_rotate(q: vec4f, v: vec3f) -> vec3f {
  return v + 2.0 * cross(q.xyz, cross(q.xyz, v) + q.w * v);
}
// A part's turn about a pivot, read off three posed points: (posed pivot, quaternion).
struct Frame { o: vec3f, q: vec4f };
fn part_frame(j: vec3f, part: u32, p: PP) -> Frame {
  let o = pose_rigid(j, part, p);
  let x = normalize(pose_rigid(j + vec3f(0.1, 0.0, 0.0), part, p) - o);
  let y0 = pose_rigid(j + vec3f(0.0, 0.1, 0.0), part, p) - o;
  let y = normalize(y0 - x * dot(y0, x));
  return Frame(o, quat_of(mat3x3f(x, y, cross(x, y))));
}

// A figure vertex in the pose, blended across a nearby cut (pose.rs `pose_mesh_vertex`).
fn pose_mesh_vertex(v: vec3f, c: f32, part: u32, p: PP) -> vec3f {
  let s = seam_of(c);
  if (s.y <= 0.0) { return pose_rigid(v, part, p); }
  let other = u32(s.x);
  let j = seam_joint(part, other);
  if (j.w <= 0.0) { return pose_rigid(v, part, p); }
  let fa = part_frame(j.xyz, part, p);
  let fb = part_frame(j.xyz, other, p);
  let qb = select(fb.q, -fb.q, dot(fa.q, fb.q) < 0.0);
  let q = normalize(mix(fa.q, qb, s.y));
  return mix(fa.o, fb.o, s.y) + quat_rotate(q, v - j.xyz);
}

fn pose_rigid(v: vec3f, part: u32, p: PP) -> vec3f {
  var q = v;
  if (part == 1u) { q = sideways(leg_roll(rot(v, 0.0, 0.88, p.hip.x), p, p.knee.x, -0.1), p.side.x, -0.1); }
  else if (part == 2u) { q = sideways(leg_roll(rot(v, 0.0, 0.88, p.hip.y), p, p.knee.y, 0.1), p.side.y, 0.1); }
  else if (part == 6u) { q = sideways(leg_roll(leg(v, p.hip.x, p.knee.x), p, p.knee.x, -0.1), p.side.x, -0.1); }
  else if (part == 7u) { q = sideways(leg_roll(leg(v, p.hip.y, p.knee.y), p, p.knee.y, 0.1), p.side.y, 0.1); }
  else if (part == 13u) { q = sideways(leg_roll(foot(v, p.hip.x, p.knee.x, p.flat, p.prone, p.toe.x), p, p.knee.x, -0.1), p.side.x, -0.1); }
  else if (part == 14u) { q = sideways(leg_roll(foot(v, p.hip.y, p.knee.y, p.flat, p.prone, p.toe.y), p, p.knee.y, 0.1), p.side.y, 0.1); }
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
  let is_leg = part == 1u || part == 2u || part == 6u || part == 7u || part == 13u || part == 14u;
  if (is_leg) { q = turn(q, p.yaw); }
  if (!is_leg) {
    q = rot(turn(q, p.twist), 0.0, 0.88, -p.lean);
    // Sideways lean around the hips (pose.rs `tilt`), positive to +x.
    let ts = sin(p.tilt);
    let tc = cos(p.tilt);
    let tz = q.z - 0.88;
    q = vec3f(q.x * tc + tz * ts, q.y, 0.88 - q.x * ts + tz * tc);
  }
  q.z = q.z + p.drop + p.bob;
  if (p.prone != 0.0) {
    // Positive onto his front, negative onto his back (pose.rs PRONE_LIFT 0.14, SUPINE_LIFT 0.16).
    q = rot(q, 0.0, 0.0, -1.5707963 * p.prone);
    q.z = q.z + prone_lift(p.prone) + p.raise;
  }
  return q;
}

// How far the body is lifted at a prone amount (pose.rs `prone_lift`). Prone keeps the transition lift 0.23 until
// the end, then settles to 0.14 (pose.rs PRONE_LIFT_MID, prone_settle).
fn prone_lift(prone: f32) -> f32 {
  let ap = abs(prone);
  let lift = select(0.16 * ap, 0.23 * ap - 0.09 * pow(ap, 12.0), prone > 0.0);
  return lift + 0.12 * sin(3.1415927 * ap);
}

// pose_rigid's upper-body chain and its inverse (pose.rs `pose_upper`, `unpose_upper`).
fn tilt_hips(q: vec3f, a: f32) -> vec3f {
  let ts = sin(a);
  let tc = cos(a);
  let tz = q.z - 0.88;
  return vec3f(q.x * tc + tz * ts, q.y, 0.88 - q.x * ts + tz * tc);
}
fn pose_upper(v: vec3f, p: PP) -> vec3f {
  var q = tilt_hips(rot(turn(v, p.twist), 0.0, 0.88, -p.lean), p.tilt);
  q.z = q.z + p.drop + p.bob;
  if (p.prone != 0.0) {
    q = rot(q, 0.0, 0.0, -1.5707963 * p.prone);
    q.z = q.z + prone_lift(p.prone) + p.raise;
  }
  return q;
}
fn unpose_upper(w: vec3f, p: PP) -> vec3f {
  var q = w;
  if (p.prone != 0.0) {
    q.z = q.z - prone_lift(p.prone) - p.raise;
    q = rot(q, 0.0, 0.0, 1.5707963 * p.prone);
  }
  q.z = q.z - p.drop - p.bob;
  return turn(rot(tilt_hips(q, -p.tilt), 0.0, 0.88, p.lean), -p.twist);
}

// The drop's left palm (pose.rs DROP_HAND, DROP_HAND_REACH, DROP_HAND_SLIDE, `drop_hand`): planted on the ground,
// the body coming down over it, then slid along the ground into the prone rest.
const DROP_HAND: vec3f = vec3f(-0.28, 0.8, 0.04);
const DROP_HAND_REACH: vec2f = vec2f(0.38, 0.5);
const DROP_HAND_SLIDE: vec2f = vec2f(0.8, 1.0);
fn drop_hand(p: PP, t: f32) -> PP {
  let w = smoothstep(DROP_HAND_REACH.x, DROP_HAND_REACH.y, t);
  if (w <= 0.0) { return p; }
  let own = pose_upper(hand_l(p), p);
  let s = smoothstep(DROP_HAND_SLIDE.x, DROP_HAND_SLIDE.y, t);
  return reach_left(p, unpose_upper(mix(DROP_HAND, own, s), p), w, 2.0);
}
// Where on the drop he is, from the two keys he was handed and the blend between them (pose.rs `drop_keys`,
// DROP_KNEEL 0.35, DROP_REACH 0.55, DROP_PLANT 0.78); -1 off the drop. These key pairs only occur on it.
// Seconds a pose change takes (pose.rs BLEND_S, KNEEL_BLEND_S, `blend_s`): quicker between standing (stand 0,
// stand aim 6, ready 30) and kneeling (crouch 3, crouch aim 14).
const BLEND_S: f32 = 0.35;
const KNEEL_BLEND_S: f32 = 0.26923078;
fn blend_s(a: u32, b: u32) -> f32 {
  let sa = a == 0u || a == 6u || a == 30u;
  let sb = b == 0u || b == 6u || b == 30u;
  let ka = a == 3u || a == 14u;
  let kb = b == 3u || b == 14u;
  return select(BLEND_S, KNEEL_BLEND_S, (sa && kb) || (ka && sb));
}
fn drop_t(prev: u32, pose: u32, w: f32) -> f32 {
  if (prev == 3u && pose == 34u) { return 0.35 + 0.2 * w; }
  if (prev == 34u && pose == 35u) { return 0.55 + 0.23 * w; }
  if (prev == 35u && pose == 5u) { return 0.78 + 0.22 * w; }
  return -1.0;
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
    let w = clamp((time - d.y) / blend_s(u32(d.x), u32(b.w)), 0.0, 1.0);
    // Instance c.z: upper-body pose id + 1; its fraction holds the movement direction (1/1024 turns) and the legs'
    // lag behind the upper body (127ths of LEG_LAG_MAX 0.9 rad, signed) (pose.rs `pack_layer_lag`).
    let layer = floor(d.z);
    let fx = round(fract(d.z) * 262144.0);
    let lraw = floor(fx / 1024.0);
    let turn = (fx - lraw * 1024.0) / 1024.0 * 6.2831853;
    let lag = select(lraw, lraw - 256.0, lraw >= 128.0) / 127.0 * 0.9;
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
    // At rest: idle life, each soldier out of step with the next. An aimed prone soldier still has
    // the Prone base, so fade its unaimed scan and sway with the rifle layer, not with the legs.
    if (b.z == 0.0) {
      let base_id = u32(b.w);
      let idle_time = time + b.y * 97.0;
      if (base_id == 5u && (upper_id == 36u || lb.x == 37.0)) {
        let idle_weight = select(1.0 - lb.y, lb.y, upper_id == 36u);
        if (idle_weight > 0.0) { pp = blend(pp, idle(pp, base_id, idle_time, b.y), idle_weight); }
      } else {
        pp = idle(pp, base_id, idle_time, b.y);
      }
    }
    // Rifle first (Patch: Animations 2): standing or kneeling, the hips and legs trail the chest by the lag.
    if (pp.prone == 0.0) { pp.yaw = pp.yaw + lag; }
    // Recoil pressure, carried in the pose id's fraction.
    let kick = fract(b.w);
    if (kick > 0.0) {
      pp.rifle = pp.rifle + 0.0288 * kick * kick;
      pp.sh = pp.sh - vec2f(0.108 * kick);
      pp.lean = pp.lean - 0.024 * kick;
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
        pp.drop = ground_drop(pp.hip, pp.knee, pp.side, pp.flat, pp.toe);
      }
    }
    // Left palm onto the handguard or, while reloading, along the magazine path.
    // The shadow pass skips this: a few centimetres in an arm's shadow cannot be seen.
    if (hands && (part == 3u || part == 11u || part == 9u)) {
      let hold = select(two_handed(u32(b.w)), two_handed(upper_id), layer >= 1.0);
      let gw = select(hold, grip_weight(u32(d.x), u32(b.w), w), w < 1.0 && layer < 1.0);
      pp = grip(pp, gw);
      // On the drop the left palm goes to the ground and takes the weight.
      let on_drop = drop_t(u32(d.x), u32(b.w), w);
      if (on_drop >= 0.0) { pp = drop_hand(pp, on_drop); }
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
    // Banking into a turn and leaning with acceleration.
    q = tilt_figure(pose_mesh_vertex(v, c, part, pp), a);
  }
  return place_figure(q, a, b);
}

// The figure's momentum tilt (instance a.w fraction) around its feet.
fn tilt_figure(p: vec3f, a: vec4f) -> vec3f {
  var q = p;
  let k = tilt_of(a);
  if (k.x != 0.0 || k.y != 0.0) {
    let cb = cos(k.x);
    let sb = sin(k.x);
    q = vec3f(q.x * cb - q.z * sb, q.y, q.x * sb + q.z * cb);
    let cp = cos(k.y);
    let sp = sin(k.y);
    q = vec3f(q.x, q.y * cp + q.z * sp, -q.y * sp + q.z * cp);
  }
  return q;
}

// From the figure frame to the world: turned to the heading (b.x) and set at the instance position.
fn place_figure(q: vec3f, a: vec4f, b: vec4f) -> vec3f {
  let ch = cos(b.x);
  let sh = sin(b.x);
  return vec3f(q.x * ch - q.y * sh, q.x * sh + q.y * ch, q.z) + a.xyz;
}

// An upper-body stance laid over the legs' pose (pose.rs `layer`).
fn upper_over(legs: PP, up: PP) -> PP {
  var pp = legs;
  pp.lean = up.lean; pp.sh = up.sh; pp.el = up.el; pp.reach = up.reach; pp.rifle = up.rifle; pp.ryaw = up.ryaw; pp.tilt = up.tilt; pp.head = up.head; pp.sling = up.sling; pp.look = up.look; pp.abd = up.abd;
  pp.twist = select(legs.twist + up.twist, up.twist, up.prone == 1.0);
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
fn palette_index(c0: f32, a0: vec4f) -> u32 {
  // The fraction carries the seam blend (pose.rs `bake_seams`), not colour.
  let c = select(floor(c0), c0, c0 < 0.0);
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

// CLIP REGISTRY BEGIN
// Generated by scripts/clip_table.py: baked clip tables (pose.rs `clip_buffer`), read-only on the GPU.
@group(0) @binding(3) var<storage, read> clips: array<f32>;
const CRAWL_BACK_AT: u32 = 0u;
const CRAWL_BACK_F: u32 = 11u;
const CRAWL_BACK_N: u32 = 24u;
const WALK_F_AT: u32 = 264u;
const WALK_F_F: u32 = 11u;
const WALK_F_N: u32 = 48u;
const WALK_FL_AT: u32 = 792u;
const WALK_FL_F: u32 = 11u;
const WALK_FL_N: u32 = 48u;
const WALK_L_AT: u32 = 1320u;
const WALK_L_F: u32 = 11u;
const WALK_L_N: u32 = 48u;
const WALK_BL_AT: u32 = 1848u;
const WALK_BL_F: u32 = 11u;
const WALK_BL_N: u32 = 48u;
const WALK_B_AT: u32 = 2376u;
const WALK_B_F: u32 = 11u;
const WALK_B_N: u32 = 48u;
const WALK_BR_AT: u32 = 2904u;
const WALK_BR_F: u32 = 11u;
const WALK_BR_N: u32 = 48u;
const WALK_R_AT: u32 = 3432u;
const WALK_R_F: u32 = 11u;
const WALK_R_N: u32 = 48u;
const WALK_FR_AT: u32 = 3960u;
const WALK_FR_F: u32 = 11u;
const WALK_FR_N: u32 = 48u;
const RUN_F_AT: u32 = 4488u;
const RUN_F_F: u32 = 11u;
const RUN_F_N: u32 = 48u;
const RUN_FL_AT: u32 = 5016u;
const RUN_FL_F: u32 = 11u;
const RUN_FL_N: u32 = 48u;
const RUN_L_AT: u32 = 5544u;
const RUN_L_F: u32 = 11u;
const RUN_L_N: u32 = 48u;
const RUN_BL_AT: u32 = 6072u;
const RUN_BL_F: u32 = 11u;
const RUN_BL_N: u32 = 48u;
const RUN_B_AT: u32 = 6600u;
const RUN_B_F: u32 = 11u;
const RUN_B_N: u32 = 48u;
const RUN_BR_AT: u32 = 7128u;
const RUN_BR_F: u32 = 11u;
const RUN_BR_N: u32 = 48u;
const RUN_R_AT: u32 = 7656u;
const RUN_R_F: u32 = 11u;
const RUN_R_N: u32 = 48u;
const RUN_FR_AT: u32 = 8184u;
const RUN_FR_F: u32 = 11u;
const RUN_FR_N: u32 = 48u;
const SPRINT_F_AT: u32 = 8712u;
const SPRINT_F_F: u32 = 11u;
const SPRINT_F_N: u32 = 48u;
const SPRINT_FL_AT: u32 = 9240u;
const SPRINT_FL_F: u32 = 11u;
const SPRINT_FL_N: u32 = 48u;
const SPRINT_L_AT: u32 = 9768u;
const SPRINT_L_F: u32 = 11u;
const SPRINT_L_N: u32 = 48u;
const SPRINT_BL_AT: u32 = 10296u;
const SPRINT_BL_F: u32 = 11u;
const SPRINT_BL_N: u32 = 48u;
const SPRINT_B_AT: u32 = 10824u;
const SPRINT_B_F: u32 = 11u;
const SPRINT_B_N: u32 = 48u;
const SPRINT_BR_AT: u32 = 11352u;
const SPRINT_BR_F: u32 = 11u;
const SPRINT_BR_N: u32 = 48u;
const SPRINT_R_AT: u32 = 11880u;
const SPRINT_R_F: u32 = 11u;
const SPRINT_R_N: u32 = 48u;
const SPRINT_FR_AT: u32 = 12408u;
const SPRINT_FR_F: u32 = 11u;
const SPRINT_FR_N: u32 = 48u;
const HIGH_CRAWL_AT: u32 = 12936u;
const HIGH_CRAWL_F: u32 = 23u;
const HIGH_CRAWL_N: u32 = 32u;
// The gaits' boot roll (pose.rs GAIT_TOE_AT): 24 loops of WALK_F_N rows: roll left, right, turn left, right.
const GAIT_TOE_AT: u32 = 13672u;
// Where a loop of `n` rows of `f` fields at `at` is at `phase`: the two rows around it and the weight.
struct ClipPos { a: u32, b: u32, w: f32 }
fn clip_pos(at: u32, f: u32, n: u32, phase: f32) -> ClipPos {
  let x = fract(phase) * f32(n);
  let i0 = u32(floor(x)) % n;
  return ClipPos(at + i0 * f, at + ((i0 + 1u) % n) * f, x - floor(x));
}
fn clip_get(c: ClipPos, k: u32) -> f32 { return mix(clips[c.a + k], clips[c.b + k], c.w); }
// CLIP REGISTRY END
