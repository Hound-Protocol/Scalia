struct U { m: mat4x4f, lm: mat4x4f, vdir: vec4f, pal: array<vec4f, 16>, light: vec4f, light_col: vec4f, ambient: vec4f, time: vec4f };

// Instances: a = (x, y, z, palette for the body), b = (heading, phase offset, steps per second, pose id),
// c = (previous pose id, time of the change in seconds, unused, unused): poses blend over 0.35 s.
// A figure faces +y; heading turns it around z. Poses port `pose.rs` (the tested reference) exactly.

struct PP { lean: f32, hip: vec2f, knee: vec2f, sh: vec2f, el: vec2f, head: f32, rifle: f32, prone: f32, drop: f32 };

fn rot(q: vec3f, py: f32, pz: f32, ang: f32) -> vec3f {
  let s = sin(ang);
  let c = cos(ang);
  let dy = q.y - py;
  let dz = q.z - pz;
  return vec3f(q.x, py + dy * c - dz * s, pz + dy * s + dz * c);
}

fn arm(q: vec3f, sh: f32, el: f32) -> vec3f {
  return rot(rot(q, 0.0, 1.14, el), 0.0, 1.40, sh);
}

fn leg(q: vec3f, hip: f32, knee: f32) -> vec3f {
  return rot(rot(q, 0.0, 0.46, -knee), 0.0, 0.88, hip);
}

fn pose_params(pose: u32, phase: f32) -> PP {
  let l = sin(phase * 6.2831853);
  let r = -l;
  var p = PP(0.0, vec2f(0.0), vec2f(0.0), vec2f(0.0), vec2f(0.0), 0.0, 0.0, 0.0, 0.0);
  if (pose == 1u) {
    p.hip = vec2f(0.45 * l, 0.45 * r);
    p.knee = vec2f(0.8 * max(l, 0.0) + 0.05, 0.8 * max(r, 0.0) + 0.05);
    p.sh = vec2f(-0.4 * l, 0.2 - 0.08 * r);
    p.el = vec2f(0.25 + 0.15 * max(l, 0.0), 0.6);
    p.rifle = 0.25;
  } else if (pose == 2u) {
    p.lean = 0.3;
    p.hip = vec2f(0.75 * l + 0.1, 0.75 * r + 0.1);
    p.knee = vec2f(1.3 * max(l, 0.0) + 0.3, 1.3 * max(r, 0.0) + 0.3);
    p.sh = vec2f(0.5 - 0.3 * l, 0.6 - 0.1 * r);
    p.el = vec2f(1.3, 1.1);
    p.rifle = 0.6;
    p.head = 0.25;
  } else if (pose == 3u) {
    p.lean = 0.45;
    p.hip = vec2f(1.4);
    p.knee = vec2f(2.4);
    p.sh = vec2f(0.3);
    p.el = vec2f(0.9);
    p.rifle = 0.5;
    p.head = 0.35;
  } else if (pose == 4u) {
    p.lean = 0.2;
    p.hip = vec2f(1.4);
    p.knee = vec2f(2.4);
    p.sh = vec2f(1.1, 1.2);
    p.el = vec2f(0.4, 0.6);
    p.head = 0.15;
  } else if (pose == 5u) {
    p.prone = 1.0;
    p.sh = vec2f(2.8);
    p.el = vec2f(1.3, 1.35);
    p.rifle = 1.5707963;
    p.head = 1.0;
  } else if (pose == 6u) {
    p.sh = vec2f(1.1, 1.2);
    p.el = vec2f(0.4, 0.6);
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

fn blend(a: PP, b: PP, w: f32) -> PP {
  var p = PP(mix(a.lean, b.lean, w), mix(a.hip, b.hip, w), mix(a.knee, b.knee, w), mix(a.sh, b.sh, w),
             mix(a.el, b.el, w), mix(a.head, b.head, w), mix(a.rifle, b.rifle, w), mix(a.prone, b.prone, w), 0.0);
  p.drop = select(select(ground_drop(p.hip, p.knee), b.drop, w >= 1.0), a.drop, w <= 0.0);
  return p;
}

fn pose_vertex(v: vec3f, part: u32, p: PP) -> vec3f {
  var q = v;
  if (part == 1u) { q = rot(v, 0.0, 0.88, p.hip.x); }
  else if (part == 2u) { q = rot(v, 0.0, 0.88, p.hip.y); }
  else if (part == 6u) { q = leg(v, p.hip.x, p.knee.x); }
  else if (part == 7u) { q = leg(v, p.hip.y, p.knee.y); }
  else if (part == 3u) { q = rot(v, 0.0, 1.40, p.sh.x); }
  else if (part == 4u) { q = rot(v, 0.0, 1.40, p.sh.y); }
  else if (part == 11u) { q = arm(v, p.sh.x, p.el.x); }
  else if (part == 12u) { q = arm(v, p.sh.y, p.el.y); }
  else if (part == 10u) { q = rot(v, 0.0, 1.46, p.head); }
  else if (part == 9u) {
    let hand = arm(vec3f(0.0, 0.0, 0.95), p.sh.y, p.el.y);
    q = rot(v, 0.0, 0.95, p.rifle) + vec3f(0.0, hand.y, hand.z - 0.95);
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
  let part = select(u32(floor(c / 16.0)), 8u, c < 0.0);
  if (part == 5u) {
    // Wind: a slow gust that travels across the field, scaled by height so the roots stay put.
    let gust = sin(time * 1.7 + a.x * 0.35 + a.y * 0.21 + b.y * 6.28) * 0.5 + 0.5;
    q = q + vec3f(0.09, 0.04, 0.0) * gust * q.z * 2.5;
  } else if (part != 0u) {
    let phase = select(0.0, fract(time * b.z + b.y), b.z > 0.0);
    let w = clamp((time - d.y) / 0.35, 0.0, 1.0);
    var pp = pose_params(u32(b.w), phase);
    if (w < 1.0) { pp = blend(pose_params(u32(d.x), phase), pp, w); }
    q = pose_vertex(v, part, pp);
  }
  let ch = cos(b.x);
  let sh = sin(b.x);
  return vec3f(q.x * ch - q.y * sh, q.x * sh + q.y * ch, q.z) + a.xyz;
}

fn palette_index(c: f32, a: vec4f) -> u32 {
  // Negative codes and swaying parts (grass) take the instance colour.
  let instance_colour = c < 0.0 || floor(c / 16.0) == 5.0;
  return u32(max(select(c - 16.0 * floor(c / 16.0), a.w, instance_colour), 0.0));
}
