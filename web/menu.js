// The main menu and its intro flight.
//
// The menu is the whole galaxy, turning slowly, with home marked. Start: a soft flash, the menu music cuts, the
// boom. The camera falls towards home, slowly at first and ever faster (the galaxy, then the engine's own star
// system, planet and field), straight down into the commander, whoosh, black. Then his eyes open (a round black
// border widening, with a blink) while the view eases back out to the widest field zoom, ending on him.
// Escape, Space or a click skips the flight. With reduced motion it is a plain fade.

const HIT_MS = 5000;          // Start to the camera entering the commander.
const GALAXY_MS = 2600;       // How long the galaxy is on screen after Start (the engine takes over from here).
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

// The galaxy, painted once into a square offscreen canvas: a barred spiral seen face-on (it is tilted when drawn).
// Returns the canvas and home's place on it (fractions of its size).
function paintGalaxy(size) {
  const c = document.createElement('canvas');
  c.width = c.height = size;
  // Painted on the CPU (a hundred thousand tiny fills are slow and, on some GPUs, lost on an accelerated canvas),
  // then handed to the GPU once as a bitmap (see createMenu).
  const g = c.getContext('2d', { willReadFrequently: true });
  const R = size * 0.46, cx = size / 2, cy = size / 2;
  const r = rng(0x5ca1a);
  // The disc's faint glow and the warm bulge.
  let grad = g.createRadialGradient(cx, cy, 0, cx, cy, R);
  grad.addColorStop(0, 'rgba(255,214,160,0.55)'); grad.addColorStop(0.08, 'rgba(250,200,150,0.30)');
  grad.addColorStop(0.35, 'rgba(150,160,210,0.08)'); grad.addColorStop(1, 'rgba(60,70,120,0)');
  g.fillStyle = grad; g.fillRect(0, 0, size, size);
  g.globalCompositeOperation = 'lighter';
  const ARMS = 4, PITCH = 0.23; // log-spiral pitch (radians of tan): about 13 degrees, like the Milky Way
  const arm = (k, rr) => k * Math.PI * 2 / ARMS + Math.log(Math.max(rr, 0.05) / 0.12) / Math.tan(PITCH + 0.0);
  // Dots sized to the canvas, so the galaxy reads the same at any resolution.
  const px = size / 1100;
  const dot = (x, y, s, col) => { g.fillStyle = col; const d = s * px; g.fillRect(x - d / 2, y - d / 2, d, d); };
  // Stars: a central bar, then four arms (two strong, two weak) loosening outwards, then a scatter of disc stars.
  for (let i = 0; i < 9000; i++) {
    const t = gauss(r) * 0.12, w = gauss(r) * 0.035;
    const a = 0.45;
    const x = cx + (Math.cos(a) * t - Math.sin(a) * w) * R, y = cy + (Math.sin(a) * t + Math.cos(a) * w) * R;
    dot(x, y, 1.2, `rgba(255,${200 + (r() * 30) | 0},${140 + (r() * 40) | 0},0.35)`);
  }
  for (let i = 0; i < 70000; i++) {
    const k = (r() * ARMS) | 0;
    const strong = k % 2 === 0 || r() < 0.35;
    const rr = 0.1 + Math.pow(r(), 0.85) * 0.92;
    const spread = (strong ? 0.16 : 0.26) * (0.6 + rr);
    const th = arm(k, rr) + 0.45 + gauss(r) * spread;
    const rad = rr + gauss(r) * 0.02;
    const x = cx + Math.cos(th) * rad * R, y = cy + Math.sin(th) * rad * R;
    const hot = r() < 0.02;
    const b = 0.18 + r() * 0.35;
    dot(x, y, hot ? 1.8 : 1, hot ? `rgba(255,120,170,${b + 0.2})` : `rgba(${190 + (r() * 50) | 0},${205 + (r() * 40) | 0},255,${b})`);
  }
  for (let i = 0; i < 26000; i++) {
    const rr = Math.min(1.05, -Math.log(Math.max(1e-6, r())) * 0.22);
    const th = r() * Math.PI * 2;
    dot(cx + Math.cos(th) * rr * R, cy + Math.sin(th) * rr * R, 1, `rgba(220,215,230,${0.08 + r() * 0.18})`);
  }
  // Dust lanes: dark along the inner edge of the strong arms.
  g.globalCompositeOperation = 'source-over';
  for (const k of [0, 2]) {
    for (let i = 0; i < 2200; i++) {
      const rr = 0.14 + r() * 0.75;
      const th = arm(k, rr) + 0.45 - 0.13 + gauss(r) * 0.05;
      const x = cx + Math.cos(th) * rr * R, y = cy + Math.sin(th) * rr * R;
      g.fillStyle = `rgba(8,8,14,${0.05 + r() * 0.08})`;
      g.beginPath(); g.arc(x, y, size * (0.002 + r() * 0.004), 0, 6.2832); g.fill();
    }
  }
  // The core's hot centre over everything.
  grad = g.createRadialGradient(cx, cy, 0, cx, cy, R * 0.12);
  grad.addColorStop(0, 'rgba(255,240,215,0.95)'); grad.addColorStop(1, 'rgba(255,210,150,0)');
  g.globalCompositeOperation = 'lighter';
  g.fillStyle = grad; g.beginPath(); g.arc(cx, cy, R * 0.12, 0, 6.2832); g.fill();
  // Home: out in an arm, about halfway to the edge (where the Sun sits in ours).
  const hr = 0.56, hth = arm(0, hr) + 0.45 + 0.05;
  return { canvas: c, home: [0.5 + Math.cos(hth) * hr * 0.46, 0.5 + Math.sin(hth) * hr * 0.46] };
}

export function createMenu({ viewer, store, onEnter }) {
  const root = document.getElementById('menu');
  const sky = document.getElementById('menuSky');
  const startBtn = document.getElementById('menuStart');
  const flash = document.getElementById('menuFlash');
  const lids = document.getElementById('menuLids');
  const ctx = sky.getContext('2d');
  const reduced = matchMedia('(prefers-reduced-motion: reduce)').matches;
  const galaxy = paintGalaxy(Math.min(3072, Math.max(1600, Math.round(Math.min(innerWidth, innerHeight) * (window.devicePixelRatio || 1) * 1.6))));
  let galaxyImg = galaxy.canvas;
  if (window.createImageBitmap) createImageBitmap(galaxy.canvas).then((b) => { galaxyImg = b; }).catch(() => {});
  // Far stars behind the galaxy, and the streaks of the fall.
  const r = rng(77);
  const field = Array.from({ length: 900 }, () => [r(), r(), 0.2 + r() * 0.8, r()]);
  const streaks = Array.from({ length: 260 }, () => [r() * 6.2832, r(), 0.4 + r() * 0.6]);

  const sound = (src, vol) => { const a = new Audio(src); a.preload = 'auto'; a.volume = vol; return a; };
  const tune = sound('./sfx/menu-music.mp3', 0.55); tune.loop = true;
  const boom = sound('./sfx/menu-boom.mp3', 0.9);
  const travel = sound('./sfx/menu-travel.mp3', 0.8);
  const enter = sound('./sfx/menu-enter.mp3', 0.85);
  const musicOn = () => store.get('music.on') !== '0';

  // 'menu' (waiting for Start), 'fly' (the intro), 'done'.
  let state = 'hidden', t0 = 0, skipAt = null, raf = 0;
  const v = () => viewer();
  const intro = (m, e, f) => { try { v()?.set_intro(m, e, f); } catch (_) { /* engine down: the overlays still play */ } };

  const fit = () => {
    const d = window.devicePixelRatio || 1;
    const w = Math.round(innerWidth * d), h = Math.round(innerHeight * d);
    if (sky.width !== w || sky.height !== h) { sky.width = w; sky.height = h; }
  };

  // Draws the galaxy at `zoom` (1 = whole galaxy) closing on home, `warp` 0..1 for the streaks of the fall.
  function drawSky(now, zoom, warp) {
    fit();
    const W = sky.width, H = sky.height, S = Math.min(W, H);
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.fillStyle = '#04050a'; ctx.fillRect(0, 0, W, H);
    for (const [x, y, b, tw] of field) {
      const a = b * (0.55 + 0.45 * Math.sin(now / 900 + tw * 40)) * (1 - warp);
      ctx.fillStyle = `rgba(210,220,255,${a * 0.6})`;
      ctx.fillRect(x * W, y * H, 1.2, 1.2);
    }
    // Tilted disc turning slowly; while falling, home is pulled to the centre of the screen.
    const spin = now / 1000 * 0.012;
    const tilt = 0.52;
    const size = S * 1.25 * zoom;
    const [hx, hy] = galaxy.home;
    // Where home lands on screen with no pull: rotate, tilt.
    const ox = (hx - 0.5) * size, oy = (hy - 0.5) * size;
    const rx = ox * Math.cos(spin) - oy * Math.sin(spin), ry = (ox * Math.sin(spin) + oy * Math.cos(spin)) * tilt;
    const pull = smooth(Math.log(zoom) / Math.log(4));
    // Resting, the galaxy sits a little right of centre (the menu is on the left).
    const rest = [W * 0.58, H * 0.5];
    const cx = rest[0] + (W / 2 - rest[0] - rx) * pull, cy = rest[1] + (H / 2 - rest[1] - ry) * pull;
    ctx.save();
    ctx.translate(cx, cy); ctx.scale(1, tilt); ctx.rotate(spin);
    ctx.globalCompositeOperation = 'lighter';
    ctx.drawImage(galaxyImg, -size / 2, -size / 2, size, size);
    ctx.restore();
    ctx.globalCompositeOperation = 'source-over';
    const home = [cx + rx, cy + ry];
    // Home's mark: a thin amber ring, breathing; it shrinks away as the fall starts.
    if (state === 'menu' || warp < 0.05) {
      const k = state === 'menu' ? 1 : 1 - warp * 20;
      const d = window.devicePixelRatio || 1;
      const rr = (11 + 2.5 * Math.sin(now / 700)) * d;
      ctx.strokeStyle = `rgba(224,165,38,${0.85 * k})`; ctx.lineWidth = 1.2 * d;
      ctx.beginPath(); ctx.arc(home[0], home[1], rr, 0, 6.2832); ctx.stroke();
      ctx.beginPath(); ctx.moveTo(home[0] + rr + 4 * d, home[1]); ctx.lineTo(home[0] + rr + 46 * d, home[1] - 26 * d); ctx.lineTo(home[0] + rr + 130 * d, home[1] - 26 * d); ctx.stroke();
      ctx.fillStyle = `rgba(232,220,190,${0.9 * k})`;
      ctx.font = `${12 * d}px 'Saira Condensed', sans-serif`;
      ctx.fillText('HOME SYSTEM', home[0] + rr + 50 * d, home[1] - 32 * d);
      ctx.fillStyle = `rgba(160,150,120,${0.9 * k})`;
      ctx.fillText('1st Battalion deployed', home[0] + rr + 50 * d, home[1] - 12 * d);
    }
    // The fall: streaks rushing out from the centre, longer and brighter as it speeds up.
    if (warp > 0) {
      ctx.globalCompositeOperation = 'lighter';
      const far = Math.hypot(W, H) / 2;
      for (const [a, p, b] of streaks) {
        const q = (p + now / 1000 * (0.3 + 2.4 * warp)) % 1;
        const r0 = far * q * q, r1 = r0 + far * (0.02 + 0.25 * warp) * q;
        ctx.strokeStyle = `rgba(200,215,255,${b * warp * q})`;
        ctx.lineWidth = 1 + 1.5 * q;
        ctx.beginPath(); ctx.moveTo(W / 2 + Math.cos(a) * r0, H / 2 + Math.sin(a) * r0); ctx.lineTo(W / 2 + Math.cos(a) * r1, H / 2 + Math.sin(a) * r1); ctx.stroke();
      }
      ctx.globalCompositeOperation = 'source-over';
    }
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
    if (state === 'menu') { drawSky(now, 1, 0); return; }
    if (state !== 'fly') return;
    const e = now - t0;
    if (skipAt !== null) return skipFrame(now);
    if (reduced) return gentle(e);
    // The galaxy: the zoom creeps, then runs (ease-in), streaks building with it.
    if (e < GALAXY_MS) {
      const u = e / GALAXY_MS;
      drawSky(now, Math.exp(Math.log(90) * Math.pow(u, 2.4)), Math.pow(u, 1.6));
      sky.style.opacity = String(1 - smooth((e - (GALAXY_MS - 700)) / 700));
    } else if (sky.style.display !== 'none') {
      sky.style.display = 'none';
    }
    // The engine: from the star system down to his head, on a log path that keeps speeding up.
    if (e < HIT_MS) {
      const u = clamp01((e - 1400) / (HIT_MS - 1400));
      const m = logLerp(FAR_M, 1.0, Math.pow(u, 1.7));
      // Dropping straight down onto him over the last few hundred metres; he dissolves as the camera goes in.
      const steep = smooth((Math.log(400) - Math.log(m)) / (Math.log(400) - Math.log(1.0)));
      intro(m, ELEV + (1.5 - ELEV) * steep, smooth((Math.log(8) - Math.log(m)) / (Math.log(8) - Math.log(1.0))));
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
    if (e >= 700 && sky.style.display !== 'none') { sky.style.display = 'none'; intro(FIELD_M, ELEV, 0); }
    if (e > 1600) finish();
  }

  function skipFrame(now) {
    const s = now - skipAt;
    if (s < 220) { lids.style.background = `rgba(0,0,0,${smooth(s / 220)})`; return; }
    sky.style.display = 'none';
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
    for (const a of [travel]) { a.pause(); }
    document.body.style.removeProperty('--blur');
    root.remove();
    document.body.classList.remove('in-menu');
    removeEventListener('keydown', keys, true);
    onEnter();
  }

  function start() {
    if (state !== 'menu') return;
    state = 'fly';
    t0 = performance.now();
    root.classList.add('flying');
    // The menu music stops dead; the boom; a soft flash.
    tune.pause();
    boom.play().catch(() => {});
    if (!reduced) {
      travel.play().catch(() => {});
      setTimeout(() => { if (state === 'fly' && skipAt === null) enter.play().catch(() => {}); }, HIT_MS - 300);
      flash.classList.add('go');
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
      if (musicOn()) tune.play().catch(() => {});
      startBtn.focus();
      cancelAnimationFrame(raf);
      raf = requestAnimationFrame(frame);
    },
    // The galaxy behind the loading screen and the notice, before sound is allowed.
    preview() { state = 'menu'; root.hidden = false; document.body.classList.add('in-menu'); raf = requestAnimationFrame(frame); },
  };
}
