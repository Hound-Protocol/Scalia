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
  }
}

// The stride's life (port of pose.rs `stride`): rise and fall, upper body turning against the legs, rifle sway.
fn stride(p: ptr<function, PP>, phase: f32, bob: f32, twist: f32, arms: f32, rifle: f32, swing: f32) {
  let t = phase * 6.2831853;
  (*p).bob = bob * cos(2.0 * t);
  (*p).twist = twist * sin(t);
  (*p).look = (*p).look - 0.8 * (*p).twist;
  let sway = sin(2.0 * t + 0.5);
  (*p).sh = (*p).sh + vec2f(arms * sway);
  (*p).rifle = (*p).rifle + rifle * sin(2.0 * t);
  (*p).ryaw = (*p).ryaw + swing * sin(t);
}

// Idle life for soldiers standing still (port of pose.rs `idle`): breathing, weight shifts, glances, a rifle hitch.
fn idle(p0: PP, pose: u32, t: f32, seed: f32) -> PP {
  var q = p0;
  if (pose == 1u || pose == 2u || pose == 7u || pose == 5u || pose == 11u) { return q; }
  let tau = 6.2831853;
  let breathe = sin(tau * (t / 3.8 + seed));
  if (pose == 6u || pose == 4u || pose == 12u || pose == 13u || pose == 14u) {
    q.sh = q.sh + vec2f(0.008 * breathe);
    q.look = q.look + 0.03 * sin(tau * (t / 7.0 + seed));
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

fn pose_params(pose: u32, phase: f32, dir: f32) -> PP {
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
    carry(&p, 5u);
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
    carry(&p, 4u);
    p.head = -0.35;
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
fn two_handed(pose: u32) -> f32 { return select(0.0, 1.0, pose <= 4u || pose == 6u || pose == 7u || pose == 8u || (pose >= 11u && pose <= 16u)); }
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
  if (p.prone > 0.0) {
    q = rot(q, 0.0, 0.0, -1.5707963 * p.prone);
    q.z = q.z + 0.23 * p.prone + 0.12 * sin(3.1415927 * p.prone);
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
    // Instance b.z: cadence (strides/s, phase from the clock) or, when negative, the phase is b.y itself: the CPU
    // integrates it for figures whose pace changes (winding up, braking), so the legs never jump.
    let phase = select(select(0.0, fract(time * b.z + b.y), b.z > 0.0), fract(b.y), b.z < 0.0);
    let w = clamp((time - d.y) / 0.35, 0.0, 1.0);
    // Instance c.z (pose.rs `pack_layer`): whole part the upper-body pose id + 1 (0 none), fractional part the way
    // the soldier moves from his facing towards the left, as a share of a turn (strafe, backpedal).
    let layer = floor(d.z);
    let turn = fract(d.z) * 6.2831853;
    let dir = select(turn, turn - 6.2831853, turn > 3.1415927);
    var pp = pose_params(u32(b.w), phase, dir);
    if (w < 1.0) { pp = blend(pose_params(u32(d.x), phase, dir), pp, w); }
    // Instance c.z: an upper-body stance (pose id + 1) over these legs, e.g. walking while aiming.
    if (layer >= 1.0) {
      let up = pose_params(u32(layer - 1.0), phase, 0.0);
      pp.lean = up.lean; pp.sh = up.sh; pp.el = up.el; pp.reach = up.reach; pp.rifle = up.rifle; pp.ryaw = up.ryaw; pp.tilt = up.tilt; pp.twist = pp.twist + up.twist; pp.head = up.head; pp.sling = up.sling; pp.look = up.look; pp.abd = up.abd;
    }
    // Standing still: idle life, each soldier out of step with the next (instance b.y is his own offset).
    if (b.z == 0.0) { pp = idle(pp, u32(b.w), time + b.y * 97.0, b.y); }
    // Left palm onto the handguard (only the left arm and the rifle move, so only they pay for it).
    // The shadow pass skips it: a few centimetres in an arm's shadow cannot be seen.
    if (hands && (part == 3u || part == 11u || part == 9u)) {
      let hold = select(two_handed(u32(b.w)), two_handed(u32(layer - 1.0)), layer >= 1.0);
      let gw = select(hold, grip_weight(u32(d.x), u32(b.w), w), w < 1.0 && layer < 1.0);
      pp = grip(pp, gw);
      // Reload: the left hand follows its path to the magazine well and pouch (fading in with the blend).
      if (u32(b.w) == 18u && layer < 1.0) { pp = reach_left(pp, reload_target(pp, phase), min(w, 1.0), 2.0); }
    }
    q = pose_vertex(v, part, pp);
  }
  let ch = cos(b.x);
  let sh = sin(b.x);
  return vec3f(q.x * ch - q.y * sh, q.x * sh + q.y * ch, q.z) + a.xyz;
}

fn palette_index(c: f32, a: vec4f) -> u32 {
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
