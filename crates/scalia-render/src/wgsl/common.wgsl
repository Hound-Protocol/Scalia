struct U { m: mat4x4f, lm: mat4x4f, vdir: vec4f, pal: array<vec4f, 16>, light: vec4f, light_col: vec4f, ambient: vec4f, time: vec4f };

// Instances: a = (x, y, z, palette for the body), b = (heading, phase offset, steps per second, unused).
// A figure faces +y; heading turns it around z. Limbs (part codes 1-4) swing around their pivots.
fn animate(p: vec3f, c: f32, a: vec4f, b: vec4f, time: f32) -> vec3f {
  var q = p;
  let part = select(floor(c / 16.0), 0.0, c < 0.0);
  if (part > 0.5 && b.z > 0.0) {
    let ph = fract(time * b.z + b.y) * 6.2831853;
    let arm = part > 2.5;
    let pivot = select(0.85, 1.40, arm);
    let amp = select(0.55, 0.45, arm);
    // Left leg and right arm swing together, then the other pair.
    let side = select(-1.0, 1.0, part == 1.0 || part == 4.0);
    let ang = sin(ph) * amp * side;
    let dz = q.z - pivot;
    q = vec3f(q.x, q.y * cos(ang) - dz * sin(ang), pivot + q.y * sin(ang) + dz * cos(ang));
  }
  let ch = cos(b.x);
  let sh = sin(b.x);
  return vec3f(q.x * ch - q.y * sh, q.x * sh + q.y * ch, q.z) + a.xyz;
}

fn palette_index(c: f32, a: vec4f) -> u32 {
  return u32(max(select(c - 16.0 * floor(c / 16.0), a.w, c < 0.0), 0.0));
}
