fn hash2(p: vec2f) -> f32 {
  let q = fract(p * vec2f(127.1, 311.7));
  return fract((q.x + q.y + q.x * q.y) * 43758.5453 + dot(q, vec2f(12.9898, 78.233)));
}
fn vnoise(p: vec2f) -> f32 {
  let i = floor(p);
  let f = fract(p);
  let s = f * f * (3.0 - 2.0 * f);
  let a = hash2(i);
  let b = hash2(i + vec2f(1.0, 0.0));
  let c = hash2(i + vec2f(0.0, 1.0));
  let d = hash2(i + vec2f(1.0, 1.0));
  return mix(mix(a, b, s.x), mix(c, d, s.x), s.y);
}
@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var sm: texture_depth_2d;
@group(0) @binding(2) var smp: sampler_comparison;
struct VO { @builtin(position) pos: vec4f, @location(0) w: vec3f, @location(1) @interpolate(flat) ci: u32 };
@vertex fn vs(@location(0) p: vec3f, @location(1) c: f32, @location(2) ia: vec4f, @location(3) ib: vec4f, @location(4) ic: vec4f) -> VO {
  let w = animate(p, c, ia, ib, ic, u.time.x);
  var o: VO; o.pos = u.m * vec4f(w, 1.0); o.w = w;
  o.ci = palette_index(c, ia);
  return o;
}
struct FO { @location(0) c: vec4f, @location(1) n: vec4f };
@fragment fn fs(i: VO) -> FO {
  var n = normalize(cross(dpdx(i.w), dpdy(i.w)));
  if (dot(n, u.vdir.xyz) < 0.0) { n = -n; }
  // Hard sun shadow: one comparison tap into the light-space depth map.
  let lp = u.lm * vec4f(i.w, 1.0);
  let uv = vec2f(lp.x * 0.5 + 0.5, 0.5 - lp.y * 0.5);
  // Bias in metres (a few centimetres, more on surfaces turned away from the light), converted to light depth.
  let ndl = max(dot(n, u.light.xyz), 0.0);
  let lit = textureSampleCompareLevel(sm, smp, uv, lp.z - (0.03 + 0.08 * (1.0 - ndl)) * u.time.z);
  let inside = all(uv > vec2f(0.0)) && all(uv < vec2f(1.0)) && lp.z < 1.0;
  // time.w = 1 turns sun shadows off (space views, where a shadow map cannot resolve planets).
  var shade = select(1.0, lit, inside && u.time.w < 0.5);
  // Drifting cloud shadows (time.y = 1 when the sky has clouds): two octaves of value noise on the ground plane.
  if (u.time.y > 0.5) {
    let cp = i.w.xy * 0.012 + vec2f(u.time.x * 0.036, u.time.x * 0.014);
    let cloud = 0.65 * vnoise(cp) + 0.35 * vnoise(cp * 2.3 + vec2f(17.0, 5.0));
    shade = shade * (1.0 - 0.55 * smoothstep(0.52, 0.62, cloud));
  }
  let l = max(dot(n, u.light.xyz), 0.0) * shade;
  // Four light steps with a 4x4 ordered dither between them: the hand-shaded pixel-art look.
  let bayer = array<f32, 16>(0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0);
  let px = vec2u(i.pos.xy) % vec2u(4u, 4u);
  // Space views (time.w = 1) light smoothly: dithering a faceted globe makes moire rings.
  let q = select(floor(l * 4.0 + bayer[px.y * 4u + px.x] / 16.0) / 4.0, l, u.time.w > 0.5);
  var o: FO;
  o.c = vec4f(u.pal[i.ci & 15u].rgb * (u.ambient.rgb + u.light_col.rgb * q), 1.0);
  o.n = vec4f(n * 0.5 + 0.5, 1.0);
  return o;
}
