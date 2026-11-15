// The main menu and its intro sequence.
//
// The prototype notice comes first, alone on black; then the menu fades in: the galaxy seen low from the side,
// turning slowly, home marked, SCALIA and a pulsing Start that reads Deploy under the pointer. The menu theme is
// played through Web Audio from a decoded buffer, so it starts at once and loops without a gap.
//
// Start / Deploy (the project lead's sequence, 14 Nov; camera moves follow his curve, position ~ t³: slow, then
// faster and faster):
//   Every flash sounds the same "massive thump".
//   1. Flash 1, the music stops: over 2 s the camera pulls back to see the galaxy at a
//      20 degree tilt while cinema bars slide in (they stay for the sequence); it holds there 2 s.
//   2. Flash 2: 3 s on home from an isometric galactic angle, barely turning to the right;
//      deep space ambience 01.
//   3. Flash 3: the commander's planet, held for 2 seconds with deep space ambience 03; then the zoom down into
//      him (the travel sound a second ahead of it, cut as the camera reaches him; the whoosh into his head) while the ambience fades and narrows to 5 kHz.
//   Then black, and his eyes open on the field with a dark braam whoosh. Escape, Space or a click skips; with reduced motion it is a fade.

const PULL_MS = 2000;         // Flash 1: the pull-back to the 20 degree view.
const LOOK_MS = 2000;         // Then the galaxy held, before flash 2.
const GALAXY_MS = 3000;       // Flash 2 to flash 3: home from the galactic angle.
const HOLD_MS = 2000;         // The planet held before the zoom.
const ZOOM_MS = 3600;         // The zoom from the planet into his head.
const BLACK_MS = 450;         // Inside him, in the dark.
const OPEN_MS = 3600;         // Eyes opening and the view easing out to the field.
const T_GALAXY = PULL_MS + LOOK_MS, T_PLANET = T_GALAXY + GALAXY_MS, T_ZOOM = T_PLANET + HOLD_MS, T_HIT = T_ZOOM + ZOOM_MS;
const ELEV = 0.6154797;       // The game camera's elevation (scalia-render camera.rs ELEV).
const PLANET_M = 3.2e6;       // The planet framed whole (viewer.rs shows the globe above 4 planet radii, 2.6e6 m).
const FIELD_M = 50;           // The widest field zoom (viewer.rs LADDER[STEP_FIELD_MAX]).
const AMB_TO_HZ = 5000;       // The zoom's ambience narrows to this.
const WAKE_FROM_HZ = 1000;    // Waking up: everything heard through a low-pass opening from here to the full range.
// After the pull-back the camera keeps drifting the same way at about 5% of its top speed (pull-backs per PULL_MS).
const DRIFT = 0.14;
const TRAVEL_LEAD_MS = 1000;  // The zoom's travel sound starts this far ahead of the zoom, to match its rush.
// The menu's camera is cinematic: 50 degrees vertical field of view (the game's own camera takes over at the
// planet). Distances are scaled so the framing matches the earlier 35.5 degree lens.
const FOV = 50 * Math.PI / 180;
const LENS = Math.tan(0.31) / Math.tan(FOV / 2);
// The project lead's speed curve: position over time as t³ (his chart: 0, .003, .04, .13, .31, .60, 1 at sixths).
export const curve = (t) => { t = Math.min(1, Math.max(0, t)); return t * t * t; };
// The same build-up (speed rising as t²) for a move that must come to rest: over its last quarter the speed
// eases back to nothing (as (1 - s)²), quickly but without a jolt. Speed is continuous throughout; ends at 1.
export const settle = (t) => {
  t = Math.min(1, Math.max(0, t));
  const tb = 0.75, L = 1 - tb, A = tb * tb * tb / 3, total = A + tb * tb * L / 3;
  if (t <= tb) return t * t * t / 3 / total;
  const s = (t - tb) / L;
  return (A + tb * tb * L * (1 - Math.pow(1 - s, 3)) / 3) / total;
};

const clamp01 = (x) => Math.min(1, Math.max(0, x));
const smooth = (x) => { x = clamp01(x); return x * x * x * (x * (x * 6 - 15) + 10); };
const logLerp = (a, b, t) => Math.exp(Math.log(a) + (Math.log(b) - Math.log(a)) * t);

// A seeded random source, so the galaxy is the same on every visit.
function rng(seed) {
  let s = seed >>> 0;
  return () => { s = (s + 0x6d2b79f5) >>> 0; let t = s; t = Math.imul(t ^ (t >>> 15), t | 1); t ^= t + Math.imul(t ^ (t >>> 7), t | 61); return ((t ^ (t >>> 14)) >>> 0) / 4294967296; };
}
const gauss = (r) => { const u = Math.max(1e-9, r()), v = r(); return Math.sqrt(-2 * Math.log(u)) * Math.cos(6.2831853 * v); };

// The galaxy as points (radius 1, disc in x-y, z up): position, colour (rgb), size and brightness per point.
// Stars and glows add light; dust (a separate set) takes it away. Home is out in an arm, about halfway out.
function buildGalaxy() {
  const r = rng(0x5ca1a);
  const stars = [], dust = [];
  const star = (x, y, z, cr, cg, cb, size, a) => stars.push(x, y, z, cr, cg, cb, size, a);
  const ARMS = 4, PITCH = 0.23;
  const arm = (k, rr) => k * Math.PI * 2 / ARMS + Math.log(Math.max(rr, 0.05) / 0.12) / Math.tan(PITCH);
  // The bulge: warm, puffed up out of the disc.
  for (let i = 0; i < 16000; i++) {
    const rad = Math.abs(gauss(r)) * 0.11, th = r() * 6.2832, ph = Math.acos(2 * r() - 1);
    star(rad * Math.sin(ph) * Math.cos(th) * 1.25, rad * Math.sin(ph) * Math.sin(th), rad * Math.cos(ph) * 0.55,
      1.0, 0.82 + r() * 0.1, 0.6 + r() * 0.15, 1.6, 0.16 + r() * 0.2);
  }
  // The bar through the middle.
  for (let i = 0; i < 6000; i++) {
    const t = gauss(r) * 0.16, w = gauss(r) * 0.035, a = 0.45;
    star(Math.cos(a) * t - Math.sin(a) * w, Math.sin(a) * t + Math.cos(a) * w, gauss(r) * 0.02, 1.0, 0.85, 0.65, 1.5, 0.18);
  }
  // Arms: two strong, two weak, loosening outwards; the disc thickens a little towards the edge.
  for (let i = 0; i < 90000; i++) {
    const k = (r() * ARMS) | 0;
    const strong = k % 2 === 0 || r() < 0.35;
    const rr = 0.1 + Math.pow(r(), 0.85) * 0.95;
    const th = arm(k, rr) + 0.45 + gauss(r) * (strong ? 0.15 : 0.25) * (0.6 + rr);
    const rad = rr + gauss(r) * 0.02;
    const hot = r() < 0.006;
    const b = 0.12 + r() * 0.3;
    if (hot) star(Math.cos(th) * rad, Math.sin(th) * rad, gauss(r) * 0.008, 1.0, 0.45, 0.65, 3.2, b + 0.25);
    else star(Math.cos(th) * rad, Math.sin(th) * rad, gauss(r) * (0.012 + 0.02 * rr), 0.72 + r() * 0.2, 0.8 + r() * 0.15, 1.0, 1.2 + r() * 0.8, b);
  }
  // The old disc between the arms.
  for (let i = 0; i < 30000; i++) {
    const rr = Math.min(1.1, -Math.log(Math.max(1e-6, r())) * 0.25), th = r() * 6.2832;
    star(Math.cos(th) * rr, Math.sin(th) * rr, gauss(r) * 0.03, 0.9, 0.86, 0.8, 1.1, 0.06 + r() * 0.1);
  }
  // Soft glows (sized in world units, see the shader): the core, and a haze over the disc.
  star(0, 0, 0, 1.0, 0.84, 0.62, 4000, 0.5);
  star(0, 0, 0, 1.0, 0.9, 0.76, 1600, 0.8);
  star(0, 0, 0, 1.0, 0.97, 0.9, 560, 0.9);
  for (let i = 0; i < 700; i++) {
    const rr = Math.pow(r(), 0.7) * 0.9, th = r() * 6.2832;
    star(Math.cos(th) * rr, Math.sin(th) * rr, 0, 0.5, 0.55, 0.8, 1200 + r() * 1200, 0.025);
  }
  // Dust lanes along the inner edges of the strong arms, thin in the plane: seen from the side, a dark band.
  for (const k of [0, 2]) {
    for (let i = 0; i < 9000; i++) {
      const rr = 0.12 + r() * 0.8;
      const th = arm(k, rr) + 0.45 - 0.13 + gauss(r) * 0.09;
      if (r() < 0.45) continue; // patchy, not a solid ring
      dust.push(Math.cos(th) * rr, Math.sin(th) * rr, gauss(r) * 0.006, 0, 0, 0, 6 + r() * 10, 0.03 + r() * 0.05);
    }
  }
  for (let i = 0; i < 6000; i++) {
    const rr = 0.08 + Math.pow(r(), 0.8) * 0.7, th = r() * 6.2832;
    dust.push(Math.cos(th) * rr, Math.sin(th) * rr, gauss(r) * 0.005, 0, 0, 0, 8 + r() * 10, 0.025);
  }
  // Home, and its own star (so the flight ends on a sun).
  const hr = 0.56, hth = arm(0, hr) + 0.45 + 0.05;
  const home = [Math.cos(hth) * hr, Math.sin(hth) * hr, 0];
  star(home[0], home[1], home[2], 1.0, 0.93, 0.78, 2.6, 0.9);
  // Its glow (size 21..30: sized with the scene and never faded up close): a speck from afar, the whole screen on
  // arrival.
  star(home[0], home[1], home[2], 1.0, 0.9, 0.72, 24, 0.6);
  return { stars: new Float32Array(stars), dust: new Float32Array(dust), home };
}

// Points: sizes up to 20 are stars (pixels at the resting distance, growing as the camera nears, capped), larger
// are glows sized with the scene; the very nearest fade out as the camera passes through them.
const VS = `
attribute vec3 pos; attribute vec3 col; attribute vec2 sa;
uniform mat4 view; uniform float proj; uniform float aspect; uniform float px; uniform float spin;
varying vec3 vcol; varying float va;
void main() {
  float c = cos(spin), s = sin(spin);
  vec3 p = vec3(pos.x * c - pos.y * s, pos.x * s + pos.y * c, pos.z);
  vec4 e = view * vec4(p, 1.0);
  float d = -e.z;
  gl_Position = vec4(e.x * proj / aspect, e.y * proj, 0.0, d);
  bool glow = sa.x > 20.0;
  float size = glow ? sa.x * px * 2.6 / max(d, 1e-4) * 0.05 : sa.x * px * clamp(2.6 / max(d, 1e-4), 0.7, 3.5);
  gl_PointSize = clamp(size, 0.0, 2048.0);
  va = sa.y * min(1.0, size);
  // Stars fade as the camera passes through them; the haze and core glows thin out once inside the disc (they
  // are seen from afar), except home's own sun.
  if (!glow) va *= smoothstep(0.0004, 0.004, d);
  else if (sa.x > 30.0) va *= smoothstep(0.45, 1.3, d);
  vcol = col;
}`;
const FS = `
precision mediump float;
varying vec3 vcol; varying float va;
uniform float dark;
void main() {
  vec2 q = gl_PointCoord * 2.0 - 1.0;
  float r2 = dot(q, q);
  // Soft, and exactly nothing at the edge of the point (no square corners on the big glows).
  float f = exp(-3.2 * r2) * max(0.0, 1.0 - r2) * max(0.0, 1.0 - r2);
  if (dark > 0.5) gl_FragColor = vec4(0.0, 0.0, 0.0, va * f);
  else gl_FragColor = vec4(vcol * va * f, 1.0);
}`;

// Look-at view matrix (column-major), camera at `eye` looking at `at`, z up.
function lookAt(eye, at) {
  const f = norm([at[0] - eye[0], at[1] - eye[1], at[2] - eye[2]]);
  const s = norm(cross(f, [0, 0, 1]));
  const u = cross(s, f);
  return [s[0], u[0], -f[0], 0, s[1], u[1], -f[1], 0, s[2], u[2], -f[2], 0, -dot(s, eye), -dot(u, eye), dot(f, eye), 1];
}
function norm(a) { const l = Math.hypot(a[0], a[1], a[2]) || 1; return [a[0] / l, a[1] / l, a[2] / l]; }
function cross(a, b) { return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]; }
function dot(a, b) { return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]; }
function lerp3(a, b, t) { return [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]; }

// The galaxy drawn with WebGL (points, added light, dust taking it away). Returns null if WebGL is unavailable.
// Built in two halves so the page never waits on the GPU: `galaxyRenderer` only queues the shader build and
// returns a `ready()` probe; asking whether a compile succeeded before it has finished stalls the page until the
// GPU process gets to it, and while the engine is starting (compiling its own pipelines) that stall ran to many
// seconds and froze the whole browser window. With KHR_parallel_shader_compile the probe answers without waiting.
function galaxyRenderer(canvas, g) {
  const gl = canvas.getContext('webgl', { alpha: false, antialias: false });
  if (!gl) return null;
  const par = gl.getExtension('KHR_parallel_shader_compile');
  const mk = (type, src) => { const s = gl.createShader(type); gl.shaderSource(s, src); gl.compileShader(s); return s; };
  const prog = gl.createProgram();
  const vs = mk(gl.VERTEX_SHADER, VS), fs = mk(gl.FRAGMENT_SHADER, FS);
  gl.attachShader(prog, vs); gl.attachShader(prog, fs); gl.linkProgram(prog);
  let draw = null;
  // Without the extension, wait a while (the engine's startup) before the one blocking question.
  const born = performance.now();
  return () => {
    if (draw) return draw;
    if (par ? !gl.getProgramParameter(prog, par.COMPLETION_STATUS_KHR) : performance.now() - born < 1500) return null;
    if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
      throw new Error(`${gl.getShaderInfoLog(vs) || ''} ${gl.getShaderInfoLog(fs) || ''} ${gl.getProgramInfoLog(prog) || ''}`);
    }
    draw = finish(gl, prog, canvas, g);
    return draw;
  };
}

function finish(gl, prog, canvas, g) {
  gl.useProgram(prog);
  const buf = (data) => { const b = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, b); gl.bufferData(gl.ARRAY_BUFFER, data, gl.STATIC_DRAW); return { b, n: data.length / 8 }; };
  const sets = { stars: buf(g.stars), dust: buf(g.dust) };
  const loc = { pos: gl.getAttribLocation(prog, 'pos'), col: gl.getAttribLocation(prog, 'col'), sa: gl.getAttribLocation(prog, 'sa') };
  const u = (n) => gl.getUniformLocation(prog, n);
  const U = { view: u('view'), proj: u('proj'), aspect: u('aspect'), px: u('px'), spin: u('spin'), dark: u('dark') };
  const bind = (set) => {
    gl.bindBuffer(gl.ARRAY_BUFFER, set.b);
    gl.enableVertexAttribArray(loc.pos); gl.vertexAttribPointer(loc.pos, 3, gl.FLOAT, false, 32, 0);
    gl.enableVertexAttribArray(loc.col); gl.vertexAttribPointer(loc.col, 3, gl.FLOAT, false, 32, 12);
    gl.enableVertexAttribArray(loc.sa); gl.vertexAttribPointer(loc.sa, 2, gl.FLOAT, false, 32, 24);
  };
  return (eye, at, spin, fovY) => {
    const W = canvas.width, H = canvas.height, aspect = W / H;
    gl.viewport(0, 0, W, H);
    gl.clearColor(0.012, 0.016, 0.03, 1); gl.clear(gl.COLOR_BUFFER_BIT);
    gl.enable(gl.BLEND);
    const proj = 1 / Math.tan(fovY / 2);
    const m = lookAt(eye, at);
    gl.uniformMatrix4fv(U.view, false, m);
    gl.uniform1f(U.proj, proj); gl.uniform1f(U.aspect, aspect); gl.uniform1f(U.px, H / 900); gl.uniform1f(U.spin, spin);
    gl.blendFunc(gl.ONE, gl.ONE); gl.uniform1f(U.dark, 0); bind(sets.stars); gl.drawArrays(gl.POINTS, 0, sets.stars.n);
    gl.blendFunc(gl.ZERO, gl.ONE_MINUS_SRC_ALPHA); gl.uniform1f(U.dark, 1); bind(sets.dust); gl.drawArrays(gl.POINTS, 0, sets.dust.n);
    // Where a point of the galaxy (its own frame) lands on screen, in CSS pixels; null behind the camera.
    return (p) => {
      const c = Math.cos(spin), s = Math.sin(spin);
      const q = [p[0] * c - p[1] * s, p[0] * s + p[1] * c, p[2]];
      const ex = m[0] * q[0] + m[4] * q[1] + m[8] * q[2] + m[12], ey = m[1] * q[0] + m[5] * q[1] + m[9] * q[2] + m[13], ez = m[2] * q[0] + m[6] * q[1] + m[10] * q[2] + m[14];
      if (ez > -1e-4) return null;
      const d = -ez;
      return [(0.5 + ex * proj / aspect / d / 2) * canvas.clientWidth, (0.5 - ey * proj / d / 2) * canvas.clientHeight];
    };
  };
}

export function createMenu({ viewer, onEnter, wake = () => {}, audio }) {
  const root = document.getElementById('menu');
  const sky = document.getElementById('menuSky');
  const marks = document.getElementById('menuMarks');
  const startBtn = document.getElementById('menuStart');
  const flash = document.getElementById('menuFlash');
  const lids = document.getElementById('menuLids');
  const foot = root.querySelector('.mn-foot');
  const mctx = marks.getContext('2d');
  const reduced = matchMedia('(prefers-reduced-motion: reduce)').matches;
  const galaxy = buildGalaxy();
  // The galaxy's GPU side is started only when the menu is first drawn (not during loading, when the engine is
  // busy with the GPU), and its shaders build in the background: until they are ready the sky is just black.
  let probe = null, draw = null;
  const makeRenderer = () => { draw = null; try { probe = galaxyRenderer(sky, galaxy); } catch (e) { probe = null; console.warn('galaxy:', e); } };
  const renderer = () => {
    if (draw || !probe) return draw;
    try { draw = probe(); } catch (e) { probe = null; console.warn('galaxy:', e); }
    return draw;
  };
  // A lost GPU context (driver reset, the engine's GPU work crowding it out) comes back and is rebuilt.
  sky.addEventListener('webglcontextlost', (e) => { e.preventDefault(); draw = null; probe = null; });
  sky.addEventListener('webglcontextrestored', makeRenderer);

  // Sound through Web Audio: everything decoded while the notice is up, so it plays the moment it is asked for.
  // Reuse Auralis' context, but create it only after the first user gesture.
  let ctx = null;
  const ensureContext = () => { if (!ctx) ctx = audio?.ensure() || null; return ctx; };
  const bufs = {};
  // One at a time, so decoding never piles up while the engine is starting: only the menu theme during loading,
  // the sequence's sounds once the menu is up (they are short; the ambiences are 15 s cuts).
  let queue = Promise.resolve();
  const load = (name, url) => {
    const context = ensureContext();
    if (!context || bufs[name]) return;
    bufs[name] = queue = queue.then(() => fetch(url)).then((r) => r.arrayBuffer()).then((a) => context.decodeAudioData(a)).then((b) => (bufs[name] = b)).catch((e) => { console.warn('menu sound', url, e); });
  };
  const loadSequence = () => {
    load('flash2', './sfx/menu-flash2-thump.mp3');
    load('amb1', './sfx/space-ambience-01-short.mp3');
    load('amb3', './sfx/space-ambience-03-short.mp3');
    load('travel', './sfx/menu-travel.mp3');
    load('enter', './sfx/menu-enter.mp3');
    load('eyes', './sfx/menu-eyes-braam.mp3');
  };
  const ready = (name) => bufs[name] instanceof AudioBuffer ? bufs[name] : null;
  // Where the sound in a buffer really starts and ends (MP3 encoders pad both ends with silence): the loop runs
  // between them, so it comes round without a gap.
  function audible(b) {
    const d = b.getChannelData(0), th = 0.002;
    let a = 0, z = d.length - 1;
    while (a < z && Math.abs(d[a]) < th) a++;
    while (z > a && Math.abs(d[z]) < th) z--;
    return [a / b.sampleRate, (z + 1) / b.sampleRate];
  }
  // Plays a decoded sound: `vol`, optionally looped over its audible part, through `via` (a node) if given.
  // Returns { src, gain, stop(fadeS) } or null if it is not decoded (yet).
  function play(name, vol, { loop = false, via = null, fadeIn = 0 } = {}) {
    const b = ready(name);
    if (!ctx || !b) return null;
    const src = ctx.createBufferSource();
    src.buffer = b;
    const gain = ctx.createGain();
    const now = ctx.currentTime;
    gain.gain.setValueAtTime(fadeIn > 0 ? 0 : vol, now);
    if (fadeIn > 0) gain.gain.linearRampToValueAtTime(vol, now + fadeIn);
    src.connect(gain).connect(via || audio?.ui || ctx.destination);
    let at = 0;
    if (loop) { const [a, z] = audible(b); src.loop = true; src.loopStart = a; src.loopEnd = z; at = a; }
    src.start(now, at);
    const stop = (fade = 0.03) => {
      const t = ctx.currentTime;
      try { gain.gain.cancelScheduledValues(t); gain.gain.setValueAtTime(gain.gain.value, t); gain.gain.linearRampToValueAtTime(0, t + fade); src.stop(t + fade + 0.02); } catch (_) { /* already stopped */ }
    };
    return { src, gain, stop };
  }
  let wakeLp = null;
  let tune = null, amb1 = null, amb3 = null, travel = null, enter = null, eyesSnd = null, ambFilter = null;
  const stopAll = (fade) => { for (const s of [tune, amb1, amb3, travel, enter, eyesSnd]) s?.stop(fade); tune = amb1 = amb3 = travel = enter = eyesSnd = null; };
  function startTune() {
    if (state !== 'menu' || tune || !ctx) return;
    ctx.resume().catch(() => {});
    tune = play('tune', 0.55, { loop: true, fadeIn: 0.4 });
    // Not decoded yet (a slow connection): as soon as it is.
    if (!tune && bufs.tune && bufs.tune.then) bufs.tune.then(() => startTune());
  }

  // 'hidden', 'menu' (waiting for Start), 'fly' (the sequence), 'done'.
  let state = 'hidden', t0 = 0, skipAt = null, raf = 0, step = 0;
  const v = () => viewer();
  const intro = (m, e, f) => { try { v()?.set_intro(m, e, f); } catch (_) { /* engine down: the overlays still play */ } };

  const fit = () => {
    const d = Math.min(2, window.devicePixelRatio || 1);
    const w = Math.round(innerWidth * d), h = Math.round(innerHeight * d);
    for (const c of [sky, marks]) if (c.width !== w || c.height !== h) { c.width = w; c.height = h; }
  };

  // The resting camera: low over the disc (about 6 degrees up), close enough that it fills the width; aimed a
  // little above the core so the galaxy lies across the lower middle and the title rides above its bulge.
  const REST_AT = [0.0, 0.0, 0.075];
  const REST_EYE = lerp3(REST_AT, [0.22, -2.05, 0.21], LENS);
  const spinAt = (now) => now / 1000 * 0.018;
  let flightSpin = 0;
  // Pulled back: the same bearing, farther out and 20 degrees above the disc.
  const restAz = Math.atan2(REST_EYE[1] - REST_AT[1], REST_EYE[0] - REST_AT[0]);
  const at3 = (o, d, az, el) => [o[0] + d * Math.cos(el) * Math.cos(az), o[1] + d * Math.cos(el) * Math.sin(az), o[2] + d * Math.sin(el)];
  const PULL_EYE = at3(REST_AT, 2.9 * LENS, restAz, 20 * Math.PI / 180);
  // The galactic angle on home: isometric (35.3 degrees down), close in, turning to the right.
  const ISO_EL = Math.atan(1 / Math.SQRT2), ISO_D = 1.35 * LENS, ORBIT = 0.012; // rad/s, barely turning; far enough to see the arms round home
  const homeAt = (spin) => { const c = Math.cos(spin), s = Math.sin(spin); return [galaxy.home[0] * c - galaxy.home[1] * s, galaxy.home[0] * s + galaxy.home[1] * c, galaxy.home[2]]; };

  function drawMarks(project, now, alpha) {
    mctx.setTransform(1, 0, 0, 1, 0, 0);
    mctx.clearRect(0, 0, marks.width, marks.height);
    if (alpha <= 0.01 || !project) return;
    const p = project(galaxy.home);
    if (!p) return;
    const d = marks.width / marks.clientWidth;
    mctx.setTransform(d, 0, 0, d, 0, 0);
    const rr = 10 + 2.5 * Math.sin(now / 700);
    mctx.strokeStyle = `rgba(224,165,38,${0.85 * alpha})`; mctx.lineWidth = 1.2;
    mctx.beginPath(); mctx.arc(p[0], p[1], rr, 0, 6.2832); mctx.stroke();
    mctx.beginPath(); mctx.moveTo(p[0] + rr + 4, p[1] + 2); mctx.lineTo(p[0] + rr + 40, p[1] + 26); mctx.lineTo(p[0] + rr + 124, p[1] + 26); mctx.stroke();
    mctx.font = "12px 'Saira Condensed', sans-serif";
    mctx.fillStyle = `rgba(232,220,190,${0.9 * alpha})`; mctx.fillText('HOME SYSTEM', p[0] + rr + 44, p[1] + 20);
    mctx.fillStyle = `rgba(160,150,120,${0.9 * alpha})`; mctx.fillText('1st Battalion deployed', p[0] + rr + 44, p[1] + 40);
  }

  function drawSky(now, eye, at, spin, markAlpha) {
    fit();
    if (!probe && !draw && !sky.dataset.tried) { sky.dataset.tried = '1'; makeRenderer(); }
    const d = renderer();
    const project = d ? d(eye, at, spin, FOV) : null;
    drawMarks(project, now, markAlpha);
  }

  // Eyelids: `open` 0 shut .. 1 gone. The gap between them is an eye's shape: a wide, flat slit that parts
  // upwards and downwards (it widens a little too, never round), soft at the lashes.
  function setLids(open) {
    open = Math.max(0, open);
    if (open >= 1) { lids.style.background = 'none'; return; }
    const h = 4 + open * 120;                 // % of the half height between the lids
    const w = 75 + 85 * Math.min(1, open * 1.4); // % of the half width
    lids.style.background = `radial-gradient(ellipse ${w}% ${h}% at 50% 52%, transparent 0%, transparent ${40 + 25 * open}%, rgba(0,0,0,0.9) ${78 + 10 * open}%, #000 100%)`;
  }
  // Two lids blink open: half open, a heavy half-close, then wide.
  const eyes = (x) => {
    x = clamp01(x);
    if (x < 0.28) return 0.55 * smooth(x / 0.28);
    if (x < 0.42) return 0.55 - 0.25 * smooth((x - 0.28) / 0.14);
    return 0.3 + 1.0 * smooth((x - 0.42) / 0.58);
  };
  // A hard white flash that hides a cut.
  function cut() {
    flash.classList.remove('go', 'cut');
    void flash.offsetWidth;
    flash.classList.add('cut');
  }

  function frame(now) {
    raf = requestAnimationFrame(frame);
    if (state === 'menu') { drawSky(now, REST_EYE, REST_AT, spinAt(now), 1); return; }
    if (state !== 'fly') return;
    const e = now - t0;
    if (skipAt !== null) return skipFrame(now);
    if (reduced) return gentle(e);
    // The cuts, each under a flash, with its sound.
    if (step === 0 && e >= T_GALAXY) {
      step = 1; cut(); play('flash2', 0.9);
      amb1 = play('amb1', 0.6, { loop: true, fadeIn: 0.25 });
    }
    if (step === 1 && e >= T_PLANET) {
      step = 2; cut(); play('flash2', 0.9);
      sky.style.display = 'none'; marks.style.display = 'none';
      amb1?.stop(0.4); amb1 = null;
      // The planet's ambience runs through a band that closes on 5 kHz as he zooms in.
      if (ctx) {
        const hp = ctx.createBiquadFilter(); hp.type = 'highpass'; hp.frequency.value = 20;
        const lp = ctx.createBiquadFilter(); lp.type = 'lowpass'; lp.frequency.value = 20000;
        hp.connect(lp).connect(audio?.ui || ctx.destination);
        ambFilter = { hp, lp };
        amb3 = play('amb3', 0.6, { loop: true, via: hp, fadeIn: 0.3 });
      }
    }
    if (step === 2 && e >= T_ZOOM - TRAVEL_LEAD_MS) {
      step = 3;
      travel = play('travel', 0.8);
    }
    if (step === 3 && e >= T_HIT - 300) {
      step = 4;
      enter = play('enter', 0.85);
    }
    // The travel sound ends the moment the camera reaches him.
    if (step === 4 && e >= T_HIT) { step = 5; travel?.stop(0.04); travel = null; }
    if (e < T_GALAXY) {
      // 1. Pulling back to the 20 degree view, then holding on the galaxy.
      // It slows to a drift (about 5% of its top speed) and keeps going the same way while the galaxy is held.
      const t = e / PULL_MS;
      const k = t <= 1 ? (1 - DRIFT) * settle(t) + DRIFT * t : 1 + DRIFT * (t - 1);
      drawSky(now, lerp3(REST_EYE, PULL_EYE, k), REST_AT, flightSpin, 1 - smooth(e / 400));
      return;
    }
    if (e < T_PLANET) {
      // 2. Home from the galactic angle, the camera orbiting it slowly to the right.
      const h = homeAt(flightSpin);
      const az = restAz - (e - T_GALAXY) / 1000 * ORBIT;
      drawSky(now, at3(h, ISO_D, az, ISO_EL), h, flightSpin, 0.8 * smooth((e - T_GALAXY - 600) / 900));
      return;
    }
    if (e < T_HIT) {
      if (e < T_ZOOM) {
        // 3. The planet, held (drifting in a touch).
        intro(PLANET_M * (1 - 0.06 * smooth((e - T_PLANET) / HOLD_MS)), ELEV, 0);
        return;
      }
      // The zoom: from the planet down to his head on the lead's curve (in log distance, so every scale passes).
      const u = clamp01((e - T_ZOOM) / ZOOM_MS);
      const m = logLerp(PLANET_M * 0.94, 1.0, curve(u));
      // Dropping straight down onto him over the last few hundred metres; he dissolves as the camera goes in.
      const steep = smooth((Math.log(400) - Math.log(m)) / Math.log(400));
      intro(m, ELEV + (1.5 - ELEV) * steep, smooth((Math.log(8) - Math.log(m)) / Math.log(8)));
      // The ambience fades and narrows: lows and highs cut in step until only 5 kHz is left, where it ends.
      if (ambFilter && amb3) {
        ambFilter.hp.frequency.value = logLerp(20, AMB_TO_HZ, u);
        ambFilter.lp.frequency.value = logLerp(20000, AMB_TO_HZ, u);
        amb3.gain.gain.value = 0.6 * Math.pow(1 - u, 1.5);
      }
      lids.style.background = e > T_HIT - 260 ? `rgba(0,0,0,${smooth((e - T_HIT + 260) / 220)})` : 'none';
      return;
    }
    if (amb3) { amb3.stop(0.05); amb3 = null; }
    // Inside: dark. Then the eyes open on him from close behind, the bars go, and the view eases out to the field.
    const o = e - T_HIT - BLACK_MS;
    if (o < 0) { lids.style.background = '#000'; intro(2.5, ELEV, 0); return; }
    // The eyes open on a dark braam.
    if (step === 5) {
      step = 6;
      if (ctx) { wakeLp = ctx.createBiquadFilter(); wakeLp.type = 'lowpass'; wakeLp.frequency.value = WAKE_FROM_HZ; wakeLp.Q.value = 0.5; wakeLp.connect(audio?.ui || ctx.destination); }
      eyesSnd = play('eyes', 0.3, { via: wakeLp });
    }
    root.classList.remove('letterbox');
    const k = o / OPEN_MS;
    setLids(eyes(k * 1.15));
    intro(logLerp(2.5, FIELD_M, smooth(k)), ELEV, 0);
    // Waking up: sight blurred until late in the opening, then clearing; hearing opening from 1 kHz to everything
    // over the whole of it.
    document.body.style.setProperty('--blur', `${(1 - smooth((k - 0.45) / 0.55)) * 9}px`);
    const hz = logLerp(WAKE_FROM_HZ, 22000, smooth(k));
    if (wakeLp) wakeLp.frequency.value = hz;
    wake(hz);
    if (k >= 1) finish();
  }

  // Reduced motion: the menu fades to black, the field fades in.
  function gentle(e) {
    lids.style.background = `rgba(0,0,0,${e < 700 ? smooth(e / 700) : 1 - smooth((e - 900) / 700)})`;
    if (e >= 700 && sky.style.display !== 'none') { sky.style.display = 'none'; marks.style.display = 'none'; intro(FIELD_M, ELEV, 0); }
    if (e > 1600) finish();
  }

  function skipFrame(now) {
    const s = now - skipAt;
    if (s < 220) { lids.style.background = `rgba(0,0,0,${smooth(s / 220)})`; return; }
    sky.style.display = 'none'; marks.style.display = 'none'; flash.style.opacity = '0';
    root.classList.remove('letterbox');
    intro(FIELD_M, ELEV, 0);
    document.body.style.setProperty('--blur', '0px');
    lids.style.background = `rgba(0,0,0,${1 - smooth((s - 220) / 600)})`;
    if (s > 820) finish();
  }

  function finish() {
    if (state === 'done') return;
    state = 'done';
    cancelAnimationFrame(raf);
    try { v()?.end_intro(); } catch (_) { /* engine down */ }
    wake(22000);
    if (wakeLp) wakeLp.frequency.value = 22000;
    // Everything but the eyes' braam, which rings out into the game.
    const keep = eyesSnd; eyesSnd = null; stopAll(0.3); eyesSnd = keep;
    document.body.style.removeProperty('--blur');
    root.remove();
    document.body.classList.remove('in-menu');
    removeEventListener('keydown', keys, true);
    onEnter();
  }

  function start() {
    if (state !== 'menu') return;
    flightSpin = spinAt(performance.now());
    state = 'fly';
    step = 0;
    t0 = performance.now();
    root.classList.add('flying');
    if (foot) foot.style.opacity = '0';
    // Flash 1: the menu music stops dead, the thump; the bars come in.
    tune?.stop(0.03); tune = null;
    ctx?.resume().catch(() => {});
    play('flash2', 0.9);
    if (!reduced) { root.classList.add('letterbox'); cut(); }
    intro(PLANET_M, ELEV, 0);
  }

  function skip() {
    if (state !== 'fly' || skipAt !== null) return;
    skipAt = performance.now();
    stopAll(0.2);
  }

  // While the menu is up nothing else hears keys: Enter starts, Escape or Space skips the sequence.
  function keys(e) {
    e.stopPropagation(); e.preventDefault();
    if (e.repeat) return;
    if (state === 'menu' && (e.key === 'Enter' || e.key === ' ')) start();
    else if (state === 'fly' && (e.key === 'Escape' || e.key === ' ' || e.key === 'Enter')) skip();
  }

  startBtn.addEventListener('click', start);
  root.addEventListener('pointerdown', (e) => { if (state === 'fly') { e.preventDefault(); skip(); } });

  return {
    // Shows the menu after the prototype notice (a click or key there, so sound is allowed): it fades in over
    // black and the theme starts at once.
    show() {
      state = 'menu';
      ensureContext();
      load('tune', './sfx/menu-music.mp3');
      root.hidden = false;
      document.body.classList.add('in-menu');
      requestAnimationFrame(() => root.classList.add('shown'));
      addEventListener('keydown', keys, true);
      startTune();
      loadSequence();
      // If the browser still holds sound back, the next key or click starts it.
      if (!tune) {
        const again = () => startTune();
        addEventListener('pointerdown', again, { once: true, capture: true });
        addEventListener('keydown', again, { once: true, capture: true });
      }
      startBtn.focus();
      cancelAnimationFrame(raf);
      raf = requestAnimationFrame(frame);
    },
    // A still of the resting view, for checking by hand.
    still() { cancelAnimationFrame(raf); state = 'still'; root.hidden = false; drawSky(performance.now(), REST_EYE, REST_AT, 0, 0); },
  };
}
