struct U { m: mat4x4f, lm: mat4x4f, vdir: vec4f, pal: array<vec4f, 32>, light: vec4f, light_col: vec4f, ambient: vec4f, time: vec4f, bend: vec4f, bend2: vec4f };

// Instances: a = (x, y, z, palette for the body), b = (heading, phase offset, steps per second, pose id),
// c = (previous pose id, time of the change in seconds, unused, unused): poses blend over 0.35 s.
// A figure faces +y; heading turns it around z. Poses port `pose.rs` (the tested reference) exactly.

struct PP { lean: f32, hip: vec2f, knee: vec2f, sh: vec2f, el: vec2f, head: f32, rifle: f32, prone: f32, drop: f32, reach: vec2f, sling: f32, bob: f32, twist: f32, look: f32, abd: vec2f };

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
// 4 ready up. Sets both arms and the rifle angle.
fn carry(p: ptr<function, PP>, c: u32) {
  if (c == 0u) { (*p).sh = vec2f(0.43, 0.48); (*p).el = vec2f(0.75, 1.89); (*p).reach = vec2f(0.89, 0.86); (*p).abd = vec2f(0.55, 0.67); (*p).rifle = -1.2; }
  else if (c == 1u) { (*p).sh = vec2f(0.89, 0.33); (*p).el = vec2f(0.40, 1.60); (*p).reach = vec2f(0.18, 1.32); (*p).abd = vec2f(0.02, 0.94); (*p).rifle = -0.5; }
  else if (c == 2u) { (*p).sh = vec2f(0.70, 0.84); (*p).el = vec2f(0.97, 1.68); (*p).reach = vec2f(0.21, 0.87); (*p).abd = vec2f(0.12, 1.04); (*p).rifle = -0.75; }
  else if (c == 3u) { (*p).sh = vec2f(1.15, -0.31); (*p).el = vec2f(0.95, 1.80); (*p).reach = vec2f(0.62, 1.19); (*p).abd = vec2f(0.64, 0.72); (*p).rifle = 0.85; }
  else { (*p).sh = vec2f(1.45, 0.58); (*p).el = vec2f(0.29, 2.23); (*p).reach = vec2f(0.44, 0.56); (*p).abd = vec2f(0.28, 1.15); (*p).rifle = 0.0; }
}

// The stride's life (port of pose.rs `stride`): rise and fall, upper body turning against the legs, rifle sway.
fn stride(p: ptr<function, PP>, phase: f32, bob: f32, twist: f32, arms: f32, rifle: f32) {
  let t = phase * 6.2831853;
  (*p).bob = bob * cos(2.0 * t);
  (*p).twist = twist * sin(t);
  let sway = sin(2.0 * t + 0.5);
  (*p).sh = (*p).sh + vec2f(arms * sway);
  (*p).rifle = (*p).rifle + rifle * sin(2.0 * t);
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
  var p = PP(0.0, vec2f(0.0), vec2f(0.0), vec2f(0.0), vec2f(0.0), 0.0, 0.0, 0.0, 0.0, vec2f(0.0), 0.0, 0.0, 0.0, 0.0, vec2f(0.0));
  if (pose == 0u) {
    carry(&p, 0u);
  } else if (pose == 1u) {
    gait(&p, phase, 0.45, 0.0, 0.75, 0.06, 0.6);
    p.lean = 0.05;
    carry(&p, 1u);
    stride(&p, phase, 0.018, 0.06, 0.035, 0.025);
  } else if (pose == 2u) {
    gait(&p, phase, 0.6, 0.08, 1.15, 0.15, 0.45);
    p.lean = 0.25;
    carry(&p, 2u);
    p.head = 0.2;
    stride(&p, phase, 0.03, 0.1, 0.05, 0.04);
  } else if (pose == 7u) {
    gait(&p, phase, 0.64, 0.1, 1.45, 0.2, 0.36);
    p.lean = 0.42;
    carry(&p, 2u);
    p.head = 0.38;
    stride(&p, phase, 0.035, 0.12, 0.03, 0.05);
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
             mix(a.look, b.look, w), mix(a.abd, b.abd, w));
  p.drop = select(select(ground_drop(p.hip, p.knee), b.drop, w >= 1.0), a.drop, w <= 0.0);
  return p;
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
    let hand = arm(vec3f(0.18, 0.0, 0.80), p.sh.y, p.el.y, p.reach.y, p.abd.y, 0.238);
    q = rot(v, 0.03, 0.78, p.rifle) + hand - vec3f(0.238, 0.03, 0.78);
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

fn animate(v: vec3f, c: f32, a: vec4f, b: vec4f, d: vec4f, time: f32) -> vec3f {
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
      pp.lean = up.lean; pp.sh = up.sh; pp.el = up.el; pp.reach = up.reach; pp.rifle = up.rifle; pp.head = up.head; pp.sling = up.sling; pp.look = up.look; pp.abd = up.abd;
    }
    // Standing still: idle life, each soldier out of step with the next (instance b.y is his own offset).
    if (b.z <= 0.0) { pp = idle(pp, u32(b.w), time + b.y * 97.0, b.y); }
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
