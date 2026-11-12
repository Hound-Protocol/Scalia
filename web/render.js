// WebGPU viewer: low-resolution render target scaled up with nearest-neighbour, flat shading, 16 colours.
const PALETTE = [
  [0.10, 0.12, 0.15], [0.42, 0.46, 0.52], [0.55, 0.58, 0.60], [0.30, 0.33, 0.38],
  [0.72, 0.58, 0.20], [0.80, 0.35, 0.15], [0.25, 0.50, 0.70], [0.30, 0.62, 0.40],
  [0.90, 0.88, 0.80], [0.60, 0.20, 0.20], [0.25, 0.40, 0.20], [0.32, 0.48, 0.24],
  [0.40, 0.52, 0.28], [0.48, 0.44, 0.30], [0.95, 0.80, 0.35], [0.85, 0.85, 0.90],
];

const SCENE_WGSL = `
struct U { m: mat4x4f, vdir: vec4f, pal: array<vec4f, 16> };
@group(0) @binding(0) var<uniform> u: U;
struct VO { @builtin(position) pos: vec4f, @location(0) w: vec3f, @location(1) @interpolate(flat) ci: u32 };
@vertex fn vs(@location(0) p: vec3f, @location(1) c: f32) -> VO {
  var o: VO; o.pos = u.m * vec4f(p, 1.0); o.w = p; o.ci = u32(c); return o;
}
struct FO { @location(0) c: vec4f, @location(1) n: vec4f };
@fragment fn fs(i: VO) -> FO {
  var n = normalize(cross(dpdx(i.w), dpdy(i.w)));
  if (dot(n, u.vdir.xyz) < 0.0) { n = -n; }
  let l = max(dot(n, normalize(vec3f(0.4, -0.3, 0.85))), 0.0);
  let q = floor(l * 4.0 + 0.5) / 4.0;
  var o: FO;
  o.c = vec4f(u.pal[i.ci & 15u].rgb * (0.45 + 0.55 * q), 1.0);
  o.n = vec4f(n * 0.5 + 0.5, 1.0);
  return o;
}`;

// Upscale pass: fetches the low-resolution texel under each screen pixel (shifted by the camera
// snap error), darkens object silhouettes from depth and lightens convex edges from normals.
const BLIT_WGSL = `
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
}`;

const ELEV = Math.atan(1 / Math.SQRT2); // 35.264 degrees: true isometric

/** Column-major view-projection for an isometric orthographic camera. */
export function isoMatrix(target, yaw, viewHeight, aspect, range) {
  const c = Math.cos(yaw), s = Math.sin(yaw), se = Math.sin(ELEV), ce = Math.cos(ELEV);
  const hh = viewHeight / 2, hw = hh * aspect;
  const rows = [
    [c / hw, s / hw, 0],
    [(-s * se) / hh, (c * se) / hh, ce / hh],
    [(-s * ce) / (2 * range), (c * ce) / (2 * range), -se / (2 * range)],
  ];
  const off = [0, 0, 0.5];
  const m = new Float32Array(16);
  for (let r = 0; r < 3; r++) {
    for (let k = 0; k < 3; k++) m[k * 4 + r] = rows[r][k];
    m[12 + r] = off[r] - (rows[r][0] * target[0] + rows[r][1] * target[1] + rows[r][2] * target[2]);
  }
  m[15] = 1;
  // direction from the scene towards the camera, in world space (the camera looks along +y', down)
  return { m, vdir: [s * ce, -c * ce, se] };
}

/**
 * Moves `target` onto the camera-plane texel grid. Returns the snapped target and the error in
 * texels (ex to the right, ey down the screen) by which the upscale must sample to undo the snap.
 */
export function snapTarget(target, yaw, texel) {
  const c = Math.cos(yaw), s = Math.sin(yaw), se = Math.sin(ELEV), ce = Math.cos(ELEV);
  const R = [c, s, 0], U = [-s * se, c * se, ce];
  const tr = R[0] * target[0] + R[1] * target[1];
  const tu = U[0] * target[0] + U[1] * target[1] + U[2] * target[2];
  const er = tr - Math.round(tr / texel) * texel, eu = tu - Math.round(tu / texel) * texel;
  const t = [0, 1, 2].map((k) => target[k] - R[k] * er - U[k] * eu);
  return { target: t, ex: er / texel, ey: -eu / texel };
}

export class Mesh {
  constructor() { this.v = []; }
  tri(a, b, c, ci) { for (const p of [a, b, c]) this.v.push(p[0], p[1], p[2], ci); }
  quad(a, b, c, d, ci) { this.tri(a, b, c, ci); this.tri(a, c, d, ci); }
  box(x0, y0, z0, x1, y1, z1, ci) {
    const p = (x, y, z) => [x, y, z];
    const a = p(x0, y0, z0), b = p(x1, y0, z0), c = p(x1, y1, z0), d = p(x0, y1, z0);
    const e = p(x0, y0, z1), f = p(x1, y0, z1), g = p(x1, y1, z1), h = p(x0, y1, z1);
    this.quad(a, d, c, b, ci); this.quad(e, f, g, h, ci);
    this.quad(a, b, f, e, ci); this.quad(b, c, g, f, ci); this.quad(c, d, h, g, ci); this.quad(d, a, e, h, ci);
  }
  /** Low-poly diamond: 8 triangles. */
  diamond(cx, cy, cz, r, ci) {
    const t = [cx, cy, cz + r], b = [cx, cy, cz - r];
    const e = [[cx + r, cy, cz], [cx, cy + r, cz], [cx - r, cy, cz], [cx, cy - r, cz]];
    for (let i = 0; i < 4; i++) { this.tri(t, e[i], e[(i + 1) % 4], ci); this.tri(b, e[(i + 1) % 4], e[i], ci); }
  }
  /** Flat ring in the z = 0 plane. */
  ring(cx, cy, radius, width, segs, ci) {
    for (let i = 0; i < segs; i++) {
      const a0 = (i / segs) * Math.PI * 2, a1 = ((i + 1) / segs) * Math.PI * 2;
      const q = (a, r) => [cx + Math.cos(a) * r, cy + Math.sin(a) * r, 0];
      this.quad(q(a0, radius - width), q(a1, radius - width), q(a1, radius + width), q(a0, radius + width), ci);
    }
  }
  get count() { return this.v.length / 4; }
  array() { return new Float32Array(this.v); }
}

export async function createRenderer(canvas) {
  if (!navigator.gpu) throw new Error('WebGPU is not available in this browser');
  const adapter = await navigator.gpu.requestAdapter();
  if (!adapter) throw new Error('No WebGPU adapter found');
  const device = await adapter.requestDevice();
  const ctx = canvas.getContext('webgpu');
  const format = navigator.gpu.getPreferredCanvasFormat();
  ctx.configure({ device, format, alphaMode: 'opaque' });

  const uni = device.createBuffer({ size: 64 + 16 + 16 * 16, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });
  const pal = new Float32Array(64);
  PALETTE.forEach((c, i) => pal.set([c[0], c[1], c[2], 1], i * 4));
  const udata = new Float32Array(20 + 64); udata.set(pal, 20);

  const sceneMod = device.createShaderModule({ code: SCENE_WGSL });
  const scenePipe = device.createRenderPipeline({
    layout: 'auto',
    vertex: { module: sceneMod, entryPoint: 'vs', buffers: [{ arrayStride: 16, attributes: [
      { shaderLocation: 0, offset: 0, format: 'float32x3' }, { shaderLocation: 1, offset: 12, format: 'float32' }] }] },
    fragment: { module: sceneMod, entryPoint: 'fs', targets: [{ format: 'rgba8unorm' }, { format: 'rgba8unorm' }] },
    primitive: { topology: 'triangle-list', cullMode: 'none' },
    depthStencil: { format: 'depth24plus', depthWriteEnabled: true, depthCompare: 'less' },
  });
  const sceneBind = device.createBindGroup({ layout: scenePipe.getBindGroupLayout(0), entries: [{ binding: 0, resource: { buffer: uni } }] });

  const blitMod = device.createShaderModule({ code: BLIT_WGSL });
  const blitPipe = device.createRenderPipeline({
    layout: 'auto',
    vertex: { module: blitMod, entryPoint: 'vs' },
    fragment: { module: blitMod, entryPoint: 'fs', targets: [{ format }] },
  });
  const bUni = device.createBuffer({ size: 32, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });

  // Internal targets carry a one-texel margin on every side so the sub-texel shift never shows an edge.
  let color, normal, depth, blitBind, size = [0, 0];
  function setInternalSize(w, h) {
    if (size[0] === w && size[1] === h) return;
    size = [w, h];
    color?.destroy(); normal?.destroy(); depth?.destroy();
    const usage = GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING;
    const sz = [w + 2, h + 2];
    color = device.createTexture({ size: sz, format: 'rgba8unorm', usage });
    normal = device.createTexture({ size: sz, format: 'rgba8unorm', usage });
    depth = device.createTexture({ size: sz, format: 'depth24plus', usage });
    blitBind = device.createBindGroup({ layout: blitPipe.getBindGroupLayout(0), entries: [
      { binding: 0, resource: color.createView() }, { binding: 1, resource: normal.createView() },
      { binding: 2, resource: depth.createView() }, { binding: 3, resource: { buffer: bUni } }] });
  }

  const buffers = new Map(); // key -> { buf, count }
  function upload(key, f32) {
    let e = buffers.get(key);
    if (!e || e.buf.size < f32.byteLength) {
      e?.buf.destroy();
      e = { buf: device.createBuffer({ size: Math.max(16, f32.byteLength), usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST }), count: 0 };
      buffers.set(key, e);
    }
    if (f32.byteLength) device.queue.writeBuffer(e.buf, 0, f32);
    e.count = f32.length / 4;
  }
  function drop(key) { buffers.get(key)?.buf.destroy(); buffers.delete(key); }

  /** Draws the named meshes with the given camera at the internal size, then scales up to the canvas. */
  function frame({ keys, target, yaw, viewHeight, range, internal, outline = true }) {
    const [w, h] = internal;
    setInternalSize(w, h);
    // Snap the camera to the texel grid in its own plane, then shift the upscale by the error:
    // stable pixels with smooth motion.
    const snap = snapTarget(target, yaw, viewHeight / h);
    const { m, vdir } = isoMatrix(snap.target, yaw, (viewHeight * (h + 2)) / h, (w + 2) / (h + 2), range);
    udata.set(m, 0); udata.set([vdir[0], vdir[1], vdir[2], 0], 16);
    device.queue.writeBuffer(uni, 0, udata);
    // Depth is linear in an orthographic view: about three texels of world distance counts as an edge.
    const thresh = ((viewHeight / h) * 3) / (2 * range);
    device.queue.writeBuffer(bUni, 0, new Float32Array([snap.ex, snap.ey, w, h, thresh, outline ? 1 : 0, 0, 0]));
    const enc = device.createCommandEncoder();
    const p1 = enc.beginRenderPass({
      colorAttachments: [
        { view: color.createView(), clearValue: { r: 0.05, g: 0.06, b: 0.09, a: 1 }, loadOp: 'clear', storeOp: 'store' },
        { view: normal.createView(), clearValue: { r: 0.5, g: 0.5, b: 1, a: 1 }, loadOp: 'clear', storeOp: 'store' }],
      depthStencilAttachment: { view: depth.createView(), depthClearValue: 1, depthLoadOp: 'clear', depthStoreOp: 'store' },
    });
    p1.setPipeline(scenePipe); p1.setBindGroup(0, sceneBind);
    for (const k of keys) { const e = buffers.get(k); if (e && e.count) { p1.setVertexBuffer(0, e.buf); p1.draw(e.count); } }
    p1.end();
    const p2 = enc.beginRenderPass({ colorAttachments: [{ view: ctx.getCurrentTexture().createView(), loadOp: 'clear', storeOp: 'store', clearValue: { r: 0, g: 0, b: 0, a: 1 } }] });
    p2.setPipeline(blitPipe); p2.setBindGroup(0, blitBind); p2.draw(3); p2.end();
    device.queue.submit([enc.finish()]);
  }
  return { upload, drop, frame, device };
}
