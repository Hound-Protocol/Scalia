// The main menu and its intro flight.
//
// The menu is the galaxy seen low from the side, a few degrees above its disc, turning slowly, with home marked.
// Start: a soft flash, the menu music cuts, the boom; a beat of stillness; then the camera heads straight for home,
// slowly at first and ever faster, through the stars, into the engine's own star system, planet and field, and
// down into the commander: whoosh, black. Then his eyes open (a round black border widening, with a blink) while
// the view eases out to the widest field zoom, ending on him. Escape, Space or a click skips the flight; with
// reduced motion it is a plain fade.

const PAUSE_MS = 1000;        // Start to the camera moving: the beat after the boom.
const GALAXY_MS = 2600;       // The flight through the galaxy (from the end of the pause).
const HIT_MS = 6000;          // Start to the camera entering the commander.
const BLACK_MS = 450;         // Inside him, in the dark.
const OPEN_MS = 3600;         // Eyes opening and the view easing out to the field.
const ELEV = 0.6154797;       // The game camera's elevation (scalia-render camera.rs ELEV).
const FAR_M = 1.5e11;         // The widest zoom the engine has (the star system).
const FIELD_M = 50;           // The widest field zoom (viewer.rs LADDER[STEP_FIELD_MAX]).

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
function galaxyRenderer(canvas, g) {
  const gl = canvas.getContext('webgl', { alpha: false, antialias: false });
  if (!gl) return null;
  const sh = (type, src) => { const s = gl.createShader(type); gl.shaderSource(s, src); gl.compileShader(s); if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(s)); return s; };
  const prog = gl.createProgram();
  gl.attachShader(prog, sh(gl.VERTEX_SHADER, VS)); gl.attachShader(prog, sh(gl.FRAGMENT_SHADER, FS)); gl.linkProgram(prog);
  if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(prog));
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

export function createMenu({ viewer, onEnter }) {
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
  let draw = null;
  const makeRenderer = () => { try { draw = galaxyRenderer(sky, galaxy); } catch (e) { draw = null; console.warn('galaxy:', e); } };
  makeRenderer();
  // A lost GPU context (driver reset, the engine's GPU work crowding it out) comes back and is rebuilt.
  sky.addEventListener('webglcontextlost', (e) => { e.preventDefault(); draw = null; });
  sky.addEventListener('webglcontextrestored', makeRenderer);

  const sound = (src, vol) => { const a = new Audio(src); a.preload = 'auto'; a.volume = vol; return a; };
  const tune = sound('../../sfx/menu-music.mp3', 0.55); tune.loop = true;
  const boom = sound('../../sfx/menu-boom.mp3', 0.9);
  const travel = sound('../../sfx/menu-travel.mp3', 0.8);
  const enter = sound('../../sfx/menu-enter.mp3', 0.85);

  // 'menu' (waiting for Start), 'fly' (the intro), 'done'.
  let state = 'hidden', t0 = 0, skipAt = null, raf = 0;
  const v = () => viewer();
  const intro = (m, e, f) => { try { v()?.set_intro(m, e, f); } catch (_) { /* engine down: the overlays still play */ } };

  const fit = () => {
    const d = Math.min(2, window.devicePixelRatio || 1);
    const w = Math.round(innerWidth * d), h = Math.round(innerHeight * d);
    for (const c of [sky, marks]) if (c.width !== w || c.height !== h) { c.width = w; c.height = h; }
  };

  // The resting camera: low over the disc (about 6 degrees up), close enough that it fills the width; aimed a
  // little above the core so the galaxy lies across the lower middle and the title rides above its bulge.
  const REST_EYE = [0.22, -2.05, 0.21];
  const REST_AT = [0.0, 0.0, 0.075];
  const FOV = 0.62;
  const spinAt = (now) => now / 1000 * 0.018;
  let flightSpin = 0;

  // The camera at `fly` (0 resting .. 1 at home) and `look` (0 at the core .. 1 at home): during the pause the
  // gaze swings onto home; then the eye heads straight for it, the distance closing ever faster.
  function camera(now, fly, look) {
    const spin = state === 'fly' ? flightSpin : spinAt(now);
    const c = Math.cos(spin), s = Math.sin(spin);
    const h = [galaxy.home[0] * c - galaxy.home[1] * s, galaxy.home[0] * s + galaxy.home[1] * c, galaxy.home[2]];
    const at = lerp3(REST_AT, h, smooth(look));
    const left = Math.exp(Math.log(0.0006) * Math.pow(fly, 2.1));
    return { eye: lerp3(h, REST_EYE, fly > 0 ? left : 1), at, spin };
  }

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

  function drawSky(now, fly, look, markAlpha) {
    fit();
    const cam = camera(now, fly, look);
    const project = draw ? draw(cam.eye, cam.at, cam.spin, FOV) : null;
    drawMarks(project, now, markAlpha);
  }

  // The round black border of opening eyes: `open` 0 shut .. 1 gone off the corners.
  function setLids(open) {
    const rad = Math.max(0, open) * 78; // % of the farthest corner
    lids.style.background = open >= 1 ? 'none'
      : `radial-gradient(ellipse ${rad * 1.25}% ${rad}% at 50% 50%, transparent 0%, transparent 62%, rgba(0,0,0,0.85) 86%, #000 100%)`;
  }
  // Two lids blink open: half open, a heavy half-close, then wide.
  const eyes = (x) => {
    x = clamp01(x);
    if (x < 0.28) return 0.55 * smooth(x / 0.28);
    if (x < 0.42) return 0.55 - 0.25 * smooth((x - 0.28) / 0.14);
    return 0.3 + 1.0 * smooth((x - 0.42) / 0.58);
  };

  function frame(now) {
    raf = requestAnimationFrame(frame);
    if (state === 'menu') { drawSky(now, 0, 0, 1); return; }
    if (state !== 'fly') return;
    const e = now - t0;
    if (skipAt !== null) return skipFrame(now);
    if (reduced) return gentle(e);
    // The pause: everything holds; the gaze settles on home and its mark fades. Then the flight through the stars.
    const g = (e - PAUSE_MS) / GALAXY_MS;
    if (g < 1) {
      drawSky(now, clamp01(g), clamp01(e / PAUSE_MS), 1 - smooth(e / (PAUSE_MS * 0.8)));
      // Arriving: home's sun swells to white, and the engine's star system takes over behind it.
      const fade = String(1 - smooth((g - 0.8) / 0.2));
      sky.style.opacity = fade; marks.style.opacity = fade;
      flash.style.opacity = String(0.5 * smooth((g - 0.72) / 0.16) * (1 - smooth((g - 0.9) / 0.1)));
    } else if (sky.style.display !== 'none') {
      sky.style.display = 'none'; marks.style.display = 'none'; flash.style.opacity = '0';
    }
    // The engine: from the star system down to his head, on a log path that keeps speeding up.
    if (e < HIT_MS) {
      const start = PAUSE_MS + GALAXY_MS * 0.8;
      const u = clamp01((e - start) / (HIT_MS - start));
      const m = logLerp(FAR_M, 1.0, Math.pow(u, 1.6));
      // Dropping straight down onto him over the last few hundred metres; he dissolves as the camera goes in.
      const steep = smooth((Math.log(400) - Math.log(m)) / Math.log(400));
      intro(m, ELEV + (1.5 - ELEV) * steep, smooth((Math.log(8) - Math.log(m)) / Math.log(8)));
      lids.style.background = e > HIT_MS - 260 ? `rgba(0,0,0,${smooth((e - HIT_MS + 260) / 220)})` : 'none';
      return;
    }
    // Inside: dark. Then the eyes open on him from close behind, and the view eases out to the field.
    const o = e - HIT_MS - BLACK_MS;
    if (o < 0) { lids.style.background = '#000'; intro(2.5, ELEV, 0); return; }
    const k = o / OPEN_MS;
    setLids(eyes(k * 1.15));
    intro(logLerp(2.5, FIELD_M, smooth(k)), ELEV, 0);
    document.body.style.setProperty('--blur', `${(1 - smooth(k * 1.6)) * 6}px`);
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
    travel.pause();
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
    t0 = performance.now();
    root.classList.add('flying');
    if (foot) foot.style.opacity = '0';
    // The menu music stops dead; the boom; a soft flash. The travel sound rises with the flight.
    tune.pause();
    boom.play().catch(() => {});
    if (!reduced) {
      setTimeout(() => { if (state === 'fly' && skipAt === null) travel.play().catch(() => {}); }, PAUSE_MS - 300);
      setTimeout(() => { if (state === 'fly' && skipAt === null) enter.play().catch(() => {}); }, HIT_MS - 300);
      flash.classList.add('go');
      setTimeout(() => flash.classList.remove('go'), 800);
    }
    intro(FAR_M, ELEV, 0);
  }

  function skip() {
    if (state !== 'fly' || skipAt !== null) return;
    skipAt = performance.now();
    travel.pause(); enter.pause();
  }

  // While the menu is up nothing else hears keys: Enter starts, Escape or Space skips the flight.
  function keys(e) {
    e.stopPropagation(); e.preventDefault();
    if (e.repeat) return;
    if (state === 'menu' && (e.key === 'Enter' || e.key === ' ')) start();
    else if (state === 'fly' && (e.key === 'Escape' || e.key === ' ' || e.key === 'Enter')) skip();
  }

  startBtn.addEventListener('click', start);
  root.addEventListener('pointerdown', (e) => { if (state === 'fly') { e.preventDefault(); skip(); } });

  return {
    // Shows the menu (after the prototype notice, so sound is allowed).
    show() {
      state = 'menu';
      root.hidden = false;
      document.body.classList.add('in-menu');
      addEventListener('keydown', keys, true);
      // The menu theme plays whatever the in-game music setting; if the browser holds it back, the next key or
      // click starts it.
      tune.play().catch(() => {
        const again = () => { if (state === 'menu') tune.play().catch(() => {}); };
        addEventListener('pointerdown', again, { once: true, capture: true });
        addEventListener('keydown', again, { once: true, capture: true });
      });
      startBtn.focus();
      cancelAnimationFrame(raf);
      raf = requestAnimationFrame(frame);
    },
    // A still of the flight (`fly` 0..1, `look` 0..1), for checking the camera by hand.
    still(fly, look) { cancelAnimationFrame(raf); state = 'still'; drawSky(performance.now(), fly, look, 0); },
    // The galaxy behind the loading screen and the notice, before sound is allowed.
    preview() { state = 'menu'; root.hidden = false; document.body.classList.add('in-menu'); raf = requestAnimationFrame(frame); },
  };
}
