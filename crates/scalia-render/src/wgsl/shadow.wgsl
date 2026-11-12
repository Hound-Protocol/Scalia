struct U { m: mat4x4f, lm: mat4x4f, vdir: vec4f, pal: array<vec4f, 16> };
@group(0) @binding(0) var<uniform> u: U;
@vertex fn vs(@location(0) p: vec3f, @location(1) c: f32, @location(2) inst: vec4f) -> @builtin(position) vec4f {
  return u.lm * vec4f(p + inst.xyz, 1.0);
}
