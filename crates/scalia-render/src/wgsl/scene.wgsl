const SUN = vec3f(SUN_XYZ);
struct U { m: mat4x4f, lm: mat4x4f, vdir: vec4f, pal: array<vec4f, 16> };
@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var sm: texture_depth_2d;
@group(0) @binding(2) var smp: sampler_comparison;
struct VO { @builtin(position) pos: vec4f, @location(0) w: vec3f, @location(1) @interpolate(flat) ci: u32 };
// Instance attribute: xyz offset and a palette index in w, used where the mesh colour is negative.
@vertex fn vs(@location(0) p: vec3f, @location(1) c: f32, @location(2) inst: vec4f) -> VO {
  let w = p + inst.xyz;
  var o: VO; o.pos = u.m * vec4f(w, 1.0); o.w = w;
  o.ci = u32(max(select(c, inst.w, c < 0.0), 0.0));
  return o;
}
struct FO { @location(0) c: vec4f, @location(1) n: vec4f };
@fragment fn fs(i: VO) -> FO {
  var n = normalize(cross(dpdx(i.w), dpdy(i.w)));
  if (dot(n, u.vdir.xyz) < 0.0) { n = -n; }
  // Hard sun shadow: one comparison tap into the light-space depth map.
  let lp = u.lm * vec4f(i.w, 1.0);
  let uv = vec2f(lp.x * 0.5 + 0.5, 0.5 - lp.y * 0.5);
  let lit = textureSampleCompareLevel(sm, smp, uv, lp.z - 0.0015);
  let inside = all(uv > vec2f(0.0)) && all(uv < vec2f(1.0)) && lp.z < 1.0;
  let sun = select(1.0, lit, inside);
  let l = max(dot(n, SUN), 0.0) * sun;
  let q = floor(l * 4.0 + 0.5) / 4.0;
  var o: FO;
  o.c = vec4f(u.pal[i.ci & 15u].rgb * (0.45 + 0.55 * q), 1.0);
  o.n = vec4f(n * 0.5 + 0.5, 1.0);
  return o;
}
