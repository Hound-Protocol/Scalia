struct U { m: mat4x4f, lm: mat4x4f, vdir: vec4f, pal: array<vec4f, 32>, light: vec4f, light_col: vec4f, ambient: vec4f, time: vec4f, bend: vec4f, bend2: vec4f };

// Instances: a = (x, y, z, palette for the body), b = (heading, phase offset, steps per second, pose id),
// c = (previous pose id, time of the change in seconds, unused, unused): poses blend over 0.35 s.
// A figure faces +y; heading turns it around z. Poses port `pose.rs` (the tested reference) exactly.

struct PP { lean: f32, hip: vec2f, knee: vec2f, sh: vec2f, el: vec2f, head: f32, rifle: f32, prone: f32, drop: f32, reach: vec2f };

fn rot(q: vec3f, py: f32, pz: f32, ang: f32) -> vec3f {
  let s = sin(ang);
  let c = cos(ang);
  let dy = q.y - py;
  let dz = q.z - pz;
  return vec3f(q.x, py + dy * c - dz * s, pz + dy * s + dz * c);
}

// An arm point (port of pose.rs `arm`): elbow bend, shoulder swing, then an inward turn around the vertical
// through the shoulder at x = sx.
fn arm(q: vec3f, sh: f32, el: f32, reach: f32, sx: f32) -> vec3f {
  let p = rot(rot(q, 0.0, 1.09, el), 0.0, 1.40, sh);
  let a = reach * sign(sx);
  let s = sin(a);
  let c = cos(a);
  let dx = p.x - sx;
  return vec3f(sx + dx * c - p.y * s, dx * s + p.y * c, p.z);
}

fn leg(q: vec3f, hip: f32, knee: f32) -> vec3f {
  return rot(rot(q, 0.0, 0.46, -knee), 0.0, 0.88, hip);
}

// Gait for both legs (port of pose.rs `gait`): x = left, y = right; the right leg is half a cycle behind.
fn gait_hip(phase: f32, swing: f32, forward: f32) -> vec2f {
  return vec2f(swing * sin(phase * 6.2831853) + forward, swing * sin((phase + 0.5) * 6.2831853) + forward);
}
fn gait_knee(phase: f32, lift: f32, base: f32) -> vec2f {
  let cl = cos(phase * 6.2831853);
  let cr = cos((phase + 0.5) * 6.2831853);
  return vec2f(base + lift * pow(max(cl, 0.0), 1.5) + 0.22 * max(-cl, 0.0),
               base + lift * pow(max(cr, 0.0), 1.5) + 0.22 * max(-cr, 0.0));
}

// Rifle carries (port of pose.rs `carry`): 0 safe hang, 1 low ready, 2 collapsed low ready, 3 high ready,
// 4 ready up. Sets both arms and the rifle angle.
fn carry(p: ptr<function, PP>, c: u32) {
  if (c == 0u) { (*p).sh = vec2f(0.65, -0.05); (*p).el = vec2f(0.05, 1.5); (*p).reach = vec2f(0.85, 0.75); (*p).rifle = -1.2; }
  else if (c == 1u) { (*p).sh = vec2f(1.1, 0.2); (*p).el = vec2f(0.05, 1.9); (*p).reach = vec2f(0.65, 0.5); (*p).rifle = -0.5; }
  else if (c == 2u) { (*p).sh = vec2f(0.8, 0.0); (*p).el = vec2f(0.05, 1.7); (*p).reach = vec2f(0.75, 0.6); (*p).rifle = -1.0; }
  else if (c == 3u) { (*p).sh = vec2f(1.0, 0.0); (*p).el = vec2f(0.95, 1.9); (*p).reach = vec2f(0.75, 0.45); (*p).rifle = 0.85; }
  else { (*p).sh = vec2f(1.45, 0.5); (*p).el = vec2f(-0.05, 1.95); (*p).reach = vec2f(0.6, 0.4); (*p).rifle = 0.0; }
}

fn pose_params(pose: u32, phase: f32) -> PP {
  let sw = sin(phase * 6.2831853);
  var p = PP(0.0, vec2f(0.0), vec2f(0.0), vec2f(0.0), vec2f(0.0), 0.0, 0.0, 0.0, 0.0, vec2f(0.0));
  if (pose == 0u) {
    carry(&p, 0u);
  } else if (pose == 1u) {
    p.hip = gait_hip(phase, 0.34, 0.0);
    p.knee = gait_knee(phase, 0.85, 0.06);
    p.lean = 0.05;
    carry(&p, 1u);
  } else if (pose == 2u) {
    p.hip = gait_hip(phase, 0.48, 0.08);
    p.knee = gait_knee(phase, 1.45, 0.2);
    p.lean = 0.25;
    carry(&p, 2u);
    p.head = 0.2;
  } else if (pose == 7u) {
    p.hip = gait_hip(phase, 0.58, 0.1);
    p.knee = gait_knee(phase, 1.9, 0.26);
    p.lean = 0.42;
    carry(&p, 2u);
    p.sh = p.sh + vec2f(0.04 * sw);
    p.head = 0.38;
  } else if (pose == 3u) {
    p.lean = 0.45;
    p.hip = vec2f(1.4);
    p.knee = vec2f(2.4);
    carry(&p, 1u);
    p.head = 0.35;
  } else if (pose == 4u) {
    p.lean = 0.2;
    p.hip = vec2f(1.4);
    p.knee = vec2f(2.4);
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
  }
  p.drop = ground_drop(p.hip, p.knee);
  return p;
}

fn ground_drop(hip: vec2f, knee: vec2f) -> f32 {
  var low = 1e9;
  for (var i = 0; i < 2; i++) {
    low = min(low, leg(vec3f(0.0, -0.10, 0.0), hip[i], knee[i]).z);
    low = min(low, leg(vec3f(0.0, 0.14, 0.0), hip[i], knee[i]).z);
  }
  return -low;
}

fn blend(a: PP, b: PP, w0: f32) -> PP {
  // Eased (smoothstep), like pose.rs.
  let w = w0 * w0 * (3.0 - 2.0 * w0);
  var p = PP(mix(a.lean, b.lean, w), mix(a.hip, b.hip, w), mix(a.knee, b.knee, w), mix(a.sh, b.sh, w),
             mix(a.el, b.el, w), mix(a.head, b.head, w), mix(a.rifle, b.rifle, w), mix(a.prone, b.prone, w), 0.0,
             mix(a.reach, b.reach, w));
  p.drop = select(select(ground_drop(p.hip, p.knee), b.drop, w >= 1.0), a.drop, w <= 0.0);
  return p;
}

fn pose_vertex(v: vec3f, part: u32, p: PP) -> vec3f {
  var q = v;
  if (part == 1u) { q = rot(v, 0.0, 0.88, p.hip.x); }
  else if (part == 2u) { q = rot(v, 0.0, 0.88, p.hip.y); }
  else if (part == 6u) { q = leg(v, p.hip.x, p.knee.x); }
  else if (part == 7u) { q = leg(v, p.hip.y, p.knee.y); }
  else if (part == 3u) { q = arm(v, p.sh.x, 0.0, p.reach.x, -0.238); }
  else if (part == 4u) { q = arm(v, p.sh.y, 0.0, p.reach.y, 0.238); }
  else if (part == 11u) { q = arm(v, p.sh.x, p.el.x, p.reach.x, -0.238); }
  else if (part == 12u) { q = arm(v, p.sh.y, p.el.y, p.reach.y, 0.238); }
  else if (part == 10u) { q = rot(v, 0.0, 1.46, p.head); }
  else if (part == 9u) {
    let hand = arm(vec3f(0.238, 0.0, 0.81), p.sh.y, p.el.y, p.reach.y, 0.238);
    q = rot(v, 0.0, 0.81, p.rifle) + vec3f(hand.x - 0.238, hand.y, hand.z - 0.81);
  }
  if (part != 1u && part != 2u && part != 6u && part != 7u) {
    q = rot(q, 0.0, 0.88, -p.lean);
  }
  q.z = q.z + p.drop;
  if (p.prone > 0.0) {
    q = rot(q, 0.0, 0.0, -1.5707963 * p.prone);
    q.z = q.z + 0.15 * p.prone + 0.12 * sin(3.1415927 * p.prone);
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
      pp.lean = up.lean; pp.sh = up.sh; pp.el = up.el; pp.reach = up.reach; pp.rifle = up.rifle; pp.head = up.head;
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
