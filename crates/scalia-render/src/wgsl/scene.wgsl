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
struct VO { @builtin(position) pos: vec4f, @location(0) w: vec3f, @location(1) @interpolate(flat) ci: u32, @location(2) @interpolate(flat) fade: f32 };
@vertex fn vs(@location(0) p: vec3f, @location(1) c: f32, @location(2) ia: vec4f, @location(3) ib: vec4f, @location(4) ic: vec4f) -> VO {
  let w = bend_point(animate(p, c, ia, ib, ic, u.time.x), u.bend, u.bend2.x);
  var o: VO; o.pos = u.m * vec4f(w, 1.0); o.w = w;
  o.ci = palette_index(c, ia);
  // Instance c.w: how far this draw has dissolved (0 solid, 1 gone); used to cross-fade terrain detail levels.
  o.fade = ic.w;
  // A fading layer is nudged towards the camera so it lies over the solid one instead of fighting it for depth.
  if (ic.w > 0.0) { o.pos.z = o.pos.z - 0.0005 * o.pos.w; }
  return o;
}
struct FO { @location(0) c: vec4f, @location(1) n: vec4f };
@fragment fn fs(i: VO) -> FO {
  var n = normalize(cross(dpdx(i.w), dpdy(i.w)));
  if (dot(n, u.vdir.xyz) < 0.0) { n = -n; }
  let bayer = array<f32, 16>(0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0);
  let px = vec2u(i.pos.xy) % vec2u(4u, 4u);
  let threshold = (bayer[px.y * 4u + px.x] + 0.5) / 16.0;
  // Dissolve with the same 4x4 pattern as the shading: the pixel-art cross-fade.
  if (i.fade > 0.0 && threshold < i.fade) { discard; }
  // Hard sun shadow: one comparison tap into the light-space depth map.
  let lp = u.lm * vec4f(i.w, 1.0);
  let uv = vec2f(lp.x * 0.5 + 0.5, 0.5 - lp.y * 0.5);
  // Bias in metres, converted to light depth: a few centimetres close up, and at least a couple of shadow-map
  // texels (their size read from the light matrix: it maps 2 / texel-width metres across 2048 texels) when zoomed
  // out, more on slopes turned from the sun. Too little and gentle slopes shadow themselves in moiré rings.
  let ndl = max(dot(n, u.light.xyz), 0.0);
  let texel_m = 2.0 / (length(vec3f(u.lm[0].x, u.lm[1].x, u.lm[2].x)) * 2048.0);
  let slope = sqrt(max(1.0 - ndl * ndl, 0.0)) / max(ndl, 0.2);
  let bias_m = max(0.03 + 0.08 * (1.0 - ndl), texel_m * (1.5 + 1.5 * slope));
  let lit = textureSampleCompareLevel(sm, smp, uv, lp.z - bias_m * u.time.z);
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
  // Smooth flat shading (the art reference): each facet lit by its own angle, no stepped dither.
  let q = l;
  var o: FO;
  o.c = vec4f(u.pal[i.ci & 31u].rgb * (u.ambient.rgb + u.light_col.rgb * q), 1.0);
  o.n = vec4f(n * 0.5 + 0.5, 1.0);
  return o;
}
