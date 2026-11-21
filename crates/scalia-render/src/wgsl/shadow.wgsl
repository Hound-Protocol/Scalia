@group(0) @binding(0) var<uniform> u: U;
@vertex fn vs(@location(0) p: vec3f, @location(1) c: f32, @location(2) ia: vec4f, @location(3) ib: vec4f, @location(4) ic: vec4f) -> @builtin(position) vec4f {
  return u.lm * vec4f(bend_point(animate(p, c, ia, ib, ic, u.time.x, false), u.bend, u.bend2.x), 1.0);
}
@vertex fn vs_skin(@location(0) p: vec3f, @location(1) c: f32, @location(5) j: vec4u, @location(6) wt: vec4f, @location(2) ia: vec4f, @location(3) ib: vec4f, @location(4) ic: vec4f, @builtin(instance_index) ii: u32) -> @builtin(position) vec4f {
  return u.lm * vec4f(bend_point(place_figure(tilt_figure(skin_point(p, j, wt, ii), ia), ia, ib), u.bend, u.bend2.x), 1.0);
}
