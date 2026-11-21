// Skeletal animation (AN-03): one thread per soldier turns its instance record (the same one scene.wgsl reads)
// into the skinning transforms of every joint, which the skinned vertex shaders (`vs_skin`) apply. Mirrors the
// CPU reference in skin.rs (`Anims::sample`, `world`, `skinning`) and the timing in common.wgsl `animate`.

struct Params { time: f32, count: u32, joints: u32, pad: u32 };
@group(0) @binding(0) var<uniform> prm: Params;
// Instance records: 3 vec4 each (scene.wgsl ia, ib, ic).
@group(0) @binding(1) var<storage, read> inst: array<vec4f>;
// Per joint 3 vec4: bind inverse rotation; bind inverse position and parent (-1 root); bone offset and upper flag.
@group(0) @binding(2) var<storage, read> skel: array<vec4f>;
// Per clip 2 vec4u: first word, frames, loops, frames per second (f32 bits); seconds (f32 bits).
@group(0) @binding(3) var<storage, read> clipinfo: array<vec4u>;
// Frame data (skin.rs): per frame the root position (3 f32) then per joint a snorm16 quaternion (2 words).
@group(0) @binding(4) var<storage, read> frames: array<u32>;
// Per pose id 3 vec4u: mode, directions, then up to 8 clips (skin.rs `pose_clips`).
@group(0) @binding(5) var<storage, read> posemap: array<vec4u>;
// Output: per soldier per joint 2 vec4: rotation, position.
@group(0) @binding(6) var<storage, read_write> palette: array<vec4f>;

const MAXJ: u32 = 64u;
const LOOP: u32 = 0u;   // phase over the loop; standing still, it runs on the clock instead
const ONCE: u32 = 1u;   // phase over the clip, once
const HOLD: u32 = 2u;   // the last frame
const CLOCK: u32 = 3u;  // always on the clock (idles)
const DROP_REACH: f32 = 0.45; // where the drop keys sit in the crouch-to-prone clip
const DROP_PLANT: f32 = 0.75;

struct Xf { q: vec4f, p: vec3f };

fn qmul(a: vec4f, b: vec4f) -> vec4f {
  return vec4f(a.w * b.xyz + b.w * a.xyz + cross(a.xyz, b.xyz), a.w * b.w - dot(a.xyz, b.xyz));
}
fn qrot(q: vec4f, v: vec3f) -> vec3f {
  let t = 2.0 * cross(q.xyz, v);
  return v + q.w * t + cross(q.xyz, t);
}
fn xmul(a: Xf, b: Xf) -> Xf { return Xf(qmul(a.q, b.q), a.p + qrot(a.q, b.p)); }
fn xmix(a: Xf, b: Xf, w: f32) -> Xf {
  let bq = select(b.q, -b.q, dot(a.q, b.q) < 0.0);
  return Xf(normalize(mix(a.q, bq, w)), mix(a.p, b.p, w));
}

fn frame_q(base: u32, j: u32) -> vec4f {
  let a = unpack2x16snorm(frames[base + 3u + 2u * j]);
  let b = unpack2x16snorm(frames[base + 4u + 2u * j]);
  return vec4f(a, b);
}
fn frame_p(base: u32) -> vec3f {
  return vec3f(bitcast<f32>(frames[base]), bitcast<f32>(frames[base + 1u]), bitcast<f32>(frames[base + 2u]));
}

// Joint `j` of `clip` at `t` seconds (a loop wraps; otherwise clamped).
fn sample(clip: u32, t: f32, j: u32) -> Xf {
  let ci = clipinfo[clip * 2u];
  let n = ci.y;
  let fps = bitcast<f32>(ci.w);
  let stride = 3u + 2u * prm.joints;
  var x = t * fps;
  var i0 = 0u;
  var i1 = 0u;
  if (ci.z != 0u) {
    x = x - floor(x / f32(n)) * f32(n);
    i0 = u32(floor(x)) % n;
    i1 = (i0 + 1u) % n;
  } else {
    x = clamp(x, 0.0, f32(n - 1u));
    i0 = min(u32(floor(x)), n - 1u);
    i1 = min(i0 + 1u, n - 1u);
  }
  let w = x - floor(x);
  let b0 = ci.x + i0 * stride;
  let b1 = ci.x + i1 * stride;
  let q0 = frame_q(b0, j);
  var q1 = frame_q(b1, j);
  q1 = select(q1, -q1, dot(q0, q1) < 0.0);
  var p = skel[j * 3u + 2u].xyz;
  if (j == 0u) { p = mix(frame_p(b0), frame_p(b1), w); }
  return Xf(normalize(mix(q0, q1, w)), p);
}

fn seconds(clip: u32) -> f32 { return bitcast<f32>(clipinfo[clip * 2u + 1u].x); }
fn pose_clip(pose: u32, k: u32) -> u32 {
  let m = posemap[pose * 3u + (k + 2u) / 4u];
  let i = (k + 2u) % 4u;
  return select(select(select(m.w, m.z, i == 2u), m.y, i == 1u), m.x, i == 0u);
}

// Joint `j` of game pose `pose` at `phase`, moving towards `dir` (radians, left of ahead). `clock` is a time
// that runs on while the soldier stands still (idles).
fn pose_joint(pose: u32, phase: f32, dir: f32, clock: f32, still: bool, j: u32) -> Xf {
  let head = posemap[pose * 3u];
  let mode = head.x;
  let dirs = head.y;
  var a = pose_clip(pose, 0u);
  var b = a;
  var w = 0.0;
  if (dirs == 8u) {
    let x = (dir - floor(dir / 6.2831853) * 6.2831853) / 0.78539816;
    let i = u32(floor(x)) % 8u;
    a = pose_clip(pose, i);
    b = pose_clip(pose, (i + 1u) % 8u);
    w = x - floor(x);
  } else if (dirs == 2u && cos(dir) < 0.0) {
    a = pose_clip(pose, 1u);
    b = a;
  }
  var t = phase;
  if (mode == HOLD) { t = 1.0e6; }
  else if (mode == CLOCK || (mode == LOOP && still)) { t = clock; }
  let ta = select(t * seconds(a), t, mode == CLOCK || (mode == LOOP && still) || mode == HOLD);
  let tb = select(t * seconds(b), t, mode == CLOCK || (mode == LOOP && still) || mode == HOLD);
  let xa = sample(a, ta, j);
  if (w > 0.0) { return xmix(xa, sample(b, tb, j), w); }
  return xa;
}

@compute @workgroup_size(64)
fn cs(@builtin(global_invocation_id) gid: vec3u) {
  let i = gid.x;
  if (i >= prm.count) { return; }
  let a = inst[i * 3u];
  let b = inst[i * 3u + 1u];
  let d = inst[i * 3u + 2u];
  let time = prm.time;
  // As common.wgsl `animate`: phase, reload progress, the blend from the previous pose, the upper-body layer.
  let phase = select(select(0.0, fract(time * b.z + b.y), b.z > 0.0), fract(b.y), b.z < 0.0);
  let reload_phase = clamp(-b.z - 1.0, 0.0, 1.0);
  let w = clamp((time - d.y) / 0.35, 0.0, 1.0);
  let layer = floor(d.z);
  let turn = fract(d.z) * 6.2831853;
  let dir = select(turn, turn - 6.2831853, turn > 3.1415927);
  let pose = u32(b.w);
  let prev = u32(d.x);
  let still = b.z == 0.0;
  let clock = time + b.y * 97.0;
  let base_phase = select(phase, reload_phase, pose == 18u);
  let prev_phase = select(phase, reload_phase, prev == 18u);
  // The drop to prone and back (pose.rs `drop_keys`): the record carries the two keys around the progress and how
  // far between them it is; the crouch-to-prone clip is played through instead.
  let drop_keys = pose == 34u || pose == 35u || prev == 34u || prev == 35u;
  var drop_t = 0.0;
  if (drop_keys) {
    let k0 = select(select(0.0, DROP_REACH, prev == 34u), DROP_PLANT, prev == 35u);
    let k1 = select(select(1.0, DROP_REACH, pose == 34u), DROP_PLANT, pose == 35u);
    drop_t = mix(k0, k1, w);
  }
  let upper_id = u32(max(layer - 1.0, 0.0));
  let upper_phase = select(phase, reload_phase, upper_id == 18u);
  let lb_whole = floor(d.w);
  let lb_code = floor(lb_whole / 64.0);
  let lb_w = select(0.0, (lb_whole - lb_code * 64.0) / 63.0, d.w >= 1.0);

  var world: array<Xf, MAXJ>;
  let nj = min(prm.joints, MAXJ);
  for (var j = 0u; j < nj; j++) {
    let s = skel[j * 3u + 2u];
    let upper = s.w > 0.5;
    var x: Xf;
    if (drop_keys) {
      let c = pose_clip(34u, 0u);
      x = sample(c, drop_t * seconds(c), j);
    } else {
      x = pose_joint(pose, base_phase, dir, clock, still, j);
      if (w < 1.0) { x = xmix(pose_joint(prev, prev_phase, dir, clock, still, j), x, w); }
    }
    if (upper && layer >= 1.0) {
      let up = pose_joint(upper_id, upper_phase, 0.0, clock, still, j);
      var was = x;
      if (lb_w > 0.0 && lb_code >= 1.0) { was = pose_joint(u32(lb_code - 1.0), phase, 0.0, clock, still, j); }
      x = up;
      if (lb_w > 0.0) { x = xmix(was, up, 1.0 - lb_w); }
    } else if (upper && lb_w > 0.0 && lb_code >= 1.0) {
      x = xmix(x, pose_joint(u32(lb_code - 1.0), phase, 0.0, clock, still, j), lb_w);
    }
    let parent = i32(skel[j * 3u + 1u].w);
    var wj = x;
    if (parent >= 0) { wj = xmul(world[parent], x); }
    world[j] = wj;
    let inv = Xf(skel[j * 3u], skel[j * 3u + 1u].xyz);
    let k = xmul(wj, inv);
    palette[(i * prm.joints + j) * 2u] = k.q;
    palette[(i * prm.joints + j) * 2u + 1u] = vec4f(k.p, 0.0);
  }
}
