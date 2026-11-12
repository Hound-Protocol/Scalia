struct B { off: vec2f, size: vec2f, thresh: f32, outline: f32, pad: vec2f };
@group(0) @binding(0) var col: texture_2d<f32>;
@group(0) @binding(1) var nrm: texture_2d<f32>;
@group(0) @binding(2) var dep: texture_depth_2d;
@group(0) @binding(3) var<uniform> b: B;
struct VO { @builtin(position) pos: vec4f, @location(0) uv: vec2f };
@vertex fn vs(@builtin(vertex_index) i: u32) -> VO {
  var p = array<vec2f, 3>(vec2f(-1.0, -1.0), vec2f(3.0, -1.0), vec2f(-1.0, 3.0));
  var o: VO; o.pos = vec4f(p[i], 0.0, 1.0); o.uv = vec2f((p[i].x + 1.0) / 2.0, (1.0 - p[i].y) / 2.0); return o;
}
fn ld(p: vec2i) -> f32 { return textureLoad(dep, p, 0); }
fn ln(p: vec2i) -> vec3f { return textureLoad(nrm, p, 0).xyz * 2.0 - 1.0; }
@fragment fn fs(i: VO) -> @location(0) vec4f {
  let full = vec2i(textureDimensions(col));
  let p = clamp(vec2i(floor(i.uv * b.size + b.off)) + vec2i(1, 1), vec2i(1, 1), full - vec2i(2, 2));
  var c = textureLoad(col, p, 0).rgb;
  if (b.outline < 0.5) { return vec4f(c, 1.0); }
  let d = ld(p);
  if (d >= 1.0) { return vec4f(c, 1.0); }
  let n = ln(p);
  var silhouette = false;
  var crease = false;
  let dirs = array<vec2i, 4>(vec2i(1, 0), vec2i(-1, 0), vec2i(0, 1), vec2i(0, -1));
  for (var k = 0; k < 4; k++) {
    let q = p + dirs[k];
    let dq = ld(q);
    if (dq - d > b.thresh) { silhouette = true; }
    else if (abs(dq - d) <= b.thresh && dq >= d && dot(n, ln(q)) < 0.85) { crease = true; }
  }
  if (silhouette) { c = c * 0.45; }
  else if (crease) { c = min(c * 1.3 + 0.04, vec3f(1.0)); }
  return vec4f(c, 1.0);
}
