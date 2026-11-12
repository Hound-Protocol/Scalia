@group(0) @binding(0) var<uniform> u: U;
@vertex fn vs(@location(0) p: vec3f, @location(1) c: f32, @location(2) ia: vec4f, @location(3) ib: vec4f) -> @builtin(position) vec4f {
  return u.lm * vec4f(animate(p, c, ia, ib, u.time.x), 1.0);
}
