struct U { m: mat4x4f, lm: mat4x4f, vdir: vec4f, pal: array<vec4f, 32>, light: vec4f, light_col: vec4f, ambient: vec4f, time: vec4f, bend: vec4f, bend2: vec4f };

// Instances: a = (x, y, z, palette for the body), b = (heading, phase offset, steps per second, pose id),
// c = (previous pose id, time of the change in seconds, unused, unused): poses blend over 0.35 s.
// A figure faces +y; heading turns it around z. Poses port `pose.rs` (the tested reference) exactly.

struct PP { lean: f32, hip: vec2f, knee: vec2f, sh: vec2f, el: vec2f, head: f32, rifle: f32, prone: f32, drop: f32, reach: vec2f, sling: f32, bob: f32, twist: f32, look: f32, abd: vec2f, ryaw: f32 };

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
fn foot(q: vec3f, hip: f32, knee: f32) -> vec3f {
  return leg(rot(q, 0.0, 0.085, ankle(hip, knee)), hip, knee);
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
fn gait(p: ptr<function, PP>, phase: f32, swing: f32, forward: f32, lift: f32, base: f32, stance: f32) {
  let l = gait_leg(fract(phase), swing, forward, lift, base, stance);
  let r = gait_leg(fract(phase + 0.5), swing, forward, lift, base, stance);
  (*p).hip = vec2f(l.x, r.x);
  (*p).knee = vec2f(l.y, r.y);
}

// Rifle carries (port of pose.rs `carry`): 0 safe hang, 1 low ready, 2 collapsed low ready, 3 high ready,
// 4 ready up, 5 rush (combat run). Sets both arms and the rifle angle.
fn carry(p: ptr<function, PP>, c: u32) {
  if (c == 0u) { (*p).sh = vec2f(0.32, 0.01); (*p).el = vec2f(0.48, 2.19); (*p).reach = vec2f(1.40, 0.49); (*p).abd = vec2f(0.67, 0.32); (*p).rifle = -1.2; }
  else if (c == 1u) { (*p).sh = vec2f(0.78, -0.10); (*p).el = vec2f(0.47, 1.98); (*p).reach = vec2f(0.24, 0.80); (*p).abd = vec2f(-0.01, 0.76); (*p).rifle = -0.5; }
  else if (c == 2u) { (*p).sh = vec2f(0.64, -0.10); (*p).el = vec2f(0.45, 2.05); (*p).reach = vec2f(0.69, 0.82); (*p).abd = vec2f(0.46, 0.74); (*p).rifle = -0.75; }
  else if (c == 3u) { (*p).sh = vec2f(1.18, 0.23); (*p).el = vec2f(0.86, 1.49); (*p).reach = vec2f(0.41, 0.37); (*p).abd = vec2f(0.20, 0.39); (*p).rifle = 0.85; }
  else if (c == 5u) { (*p).sh = vec2f(0.73, 0.01); (*p).el = vec2f(0.96, 2.06); (*p).reach = vec2f(0.29, 0.43); (*p).abd = vec2f(0.16, 0.37); (*p).rifle = -0.2; }
  else {
    (*p).sh = vec2f(1.22, 0.37); (*p).el = vec2f(0.46, 1.89); (*p).reach = vec2f(0.20, 0.40); (*p).abd = vec2f(-0.09, 0.92); (*p).rifle = 0.0;
    // Ready up is the aim (pose.rs `aim_arms`): bladed stance, muzzle straight ahead, head looking along it.
    (*p).twist = -0.75; (*p).look = 0.75;
  }
  // The rifle turned across the body: stock out to the right shoulder, muzzle to the left.
  (*p).ryaw = 0.75;
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
  if (pose == 1u || pose == 2u || pose == 7u || pose == 5u) { return q; }
  let tau = 6.2831853;
  let breathe = sin(tau * (t / 3.8 + seed));
  if (pose == 6u || pose == 4u) {
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

fn pose_params(pose: u32, phase: f32) -> PP {
  let sw = sin(phase * 6.2831853);
  var p = PP(0.0, vec2f(0.0), vec2f(0.0), vec2f(0.0), vec2f(0.0), 0.0, 0.0, 0.0, 0.0, vec2f(0.0), 0.0, 0.0, 0.0, 0.0, vec2f(0.0), 0.0);
  if (pose == 0u) {
    carry(&p, 0u);
  } else if (pose == 1u) {
    gait(&p, phase, 0.45, 0.0, 0.75, 0.06, 0.6);
    p.lean = 0.05;
    carry(&p, 1u);
    stride(&p, phase, 0.018, 0.06, 0.035, 0.025, 0.04);
  } else if (pose == 2u) {
    gait(&p, phase, 0.6, 0.2, 1.15, 0.4, 0.45);
    p.lean = 0.32;
    carry(&p, 5u);
    p.head = 0.28;
    stride(&p, phase, 0.03, 0.2, 0.08, 0.06, 0.16);
  } else if (pose == 7u) {
    gait(&p, phase, 0.56, 0.1, 1.45, 0.2, 0.36);
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
    p.head = 0.05;
  } else if (pose == 5u) {
    p.prone = 1.0;
    p.sh = vec2f(2.8);
    p.el = vec2f(1.3, 1.35);
    p.rifle = 1.5707963;
    p.head = 1.0;
  } else if (pose == 6u) {
    carry(&p, 4u);
    p.head = -0.1;
  } else if (pose == 8u) {
    carry(&p, 3u);
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
  p.drop = ground_drop(p.hip, p.knee);
  return p;
}

fn ground_drop(hip: vec2f, knee: vec2f) -> f32 {
  var low = 1e9;
  for (var i = 0; i < 2; i++) {
    low = min(low, foot(vec3f(0.0, -0.09, 0.0), hip[i], knee[i]).z);
    low = min(low, foot(vec3f(0.0, 0.14, 0.0), hip[i], knee[i]).z);
  }
  return -low;
}

fn blend(a: PP, b: PP, w0: f32) -> PP {
  // Eased (smoothstep), like pose.rs.
  let w = w0 * w0 * (3.0 - 2.0 * w0);
  var p = PP(mix(a.lean, b.lean, w), mix(a.hip, b.hip, w), mix(a.knee, b.knee, w), mix(a.sh, b.sh, w),
             mix(a.el, b.el, w), mix(a.head, b.head, w), mix(a.rifle, b.rifle, w), mix(a.prone, b.prone, w), 0.0,
             mix(a.reach, b.reach, w), mix(a.sling, b.sling, w), mix(a.bob, b.bob, w), mix(a.twist, b.twist, w),
             mix(a.look, b.look, w), mix(a.abd, b.abd, w), mix(a.ryaw, b.ryaw, w));
  p.drop = select(select(ground_drop(p.hip, p.knee), b.drop, w >= 1.0), a.drop, w <= 0.0);
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
fn two_handed(pose: u32) -> f32 { return select(0.0, 1.0, pose <= 4u || pose == 6u || pose == 7u || pose == 8u); }
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
fn grip_res(p: PP) -> vec3f { return guard(p) - hand_l(p); }
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
  let excess = max(distance(guard(p), vec3f(-0.238, 0.0, 1.40)) - 0.595, 0.0);
  let damp = 0.05 + 2.0 * excess;
  var q = p;
  for (var it = 0; it < 6; it = it + 1) {
    let r = grip_res(q);
    if (dot(r, r) < 1e-6) { break; }
    let d = 0.01;
    let j0 = (grip_res(grip_try(q, vec4f(d, 0.0, 0.0, 0.0))) - r) / d;
    let j1 = (grip_res(grip_try(q, vec4f(0.0, d, 0.0, 0.0))) - r) / d;
    let j2 = (grip_res(grip_try(q, vec4f(0.0, 0.0, d, 0.0))) - r) / d;
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
  let cap = 0.7;
  out.sh.x = out.sh.x + cap * tanh((q.sh.x - p.sh.x) / cap) * w;
  out.el.x = out.el.x + cap * tanh((q.el.x - p.el.x) / cap) * w;
  out.reach.x = out.reach.x + cap * tanh((q.reach.x - p.reach.x) / cap) * w;
  return out;
}

fn pose_vertex(v: vec3f, part: u32, p: PP) -> vec3f {
  var q = v;
  if (part == 1u) { q = rot(v, 0.0, 0.88, p.hip.x); }
  else if (part == 2u) { q = rot(v, 0.0, 0.88, p.hip.y); }
  else if (part == 6u) { q = leg(v, p.hip.x, p.knee.x); }
  else if (part == 7u) { q = leg(v, p.hip.y, p.knee.y); }
  else if (part == 13u) { q = foot(v, p.hip.x, p.knee.x); }
  else if (part == 14u) { q = foot(v, p.hip.y, p.knee.y); }
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
  } else if (part != 0u) {
    let phase = select(0.0, fract(time * b.z + b.y), b.z > 0.0);
    let w = clamp((time - d.y) / 0.35, 0.0, 1.0);
    var pp = pose_params(u32(b.w), phase);
    if (w < 1.0) { pp = blend(pose_params(u32(d.x), phase), pp, w); }
    // Instance c.z: an upper-body stance (pose id + 1) over these legs, e.g. walking while aiming.
    if (d.z > 0.5) {
      let up = pose_params(u32(d.z - 1.0), phase);
      pp.lean = up.lean; pp.sh = up.sh; pp.el = up.el; pp.reach = up.reach; pp.rifle = up.rifle; pp.ryaw = up.ryaw; pp.twist = pp.twist + up.twist; pp.head = up.head; pp.sling = up.sling; pp.look = up.look; pp.abd = up.abd;
    }
    // Standing still: idle life, each soldier out of step with the next (instance b.y is his own offset).
    if (b.z <= 0.0) { pp = idle(pp, u32(b.w), time + b.y * 97.0, b.y); }
    // Left palm onto the handguard (only the left arm and the rifle move, so only they pay for it).
    // The shadow pass skips it: a few centimetres in an arm's shadow cannot be seen.
    if (hands && (part == 3u || part == 11u || part == 9u)) {
      let hold = select(two_handed(u32(b.w)), two_handed(u32(d.z - 1.0)), d.z > 0.5);
      let gw = select(hold, grip_weight(u32(d.x), u32(b.w), w), w < 1.0 && d.z <= 0.5);
      pp = grip(pp, gw);
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
