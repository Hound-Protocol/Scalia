// Three scenes drawn by the WebGPU renderer: ships, ground with soldiers, and the star system with a flight.
import { createRenderer, Mesh } from './render.js';

export async function startViewer(wasm, ui) {
  const r = await createRenderer(ui.canvas);
  const state = { scene: 'ships', full: false, shipIndex: 5, frames: 0, last: performance.now(), fps: 0 };
  const t0 = performance.now();
  const simMs = () => performance.now() - t0;

  // ---- ships ----
  const shipCache = new Map();
  function shipMesh(i) {
    if (!shipCache.has(i)) {
      const ron = wasm.shipped_ron(i);
      const v = wasm.blueprint_mesh(ron);
      const info = JSON.parse(wasm.blueprint_info(ron));
      let lo = [1e9, 1e9, 1e9], hi = [-1e9, -1e9, -1e9];
      for (let k = 0; k < v.length; k += 4) for (let a = 0; a < 3; a++) { lo[a] = Math.min(lo[a], v[k + a]); hi[a] = Math.max(hi[a], v[k + a]); }
      r.upload('ship' + i, v);
      shipCache.set(i, { info, lo, hi });
    }
    return shipCache.get(i);
  }

  // ---- ground ----
  // One company-sized group that marches 500 m; drawn as instances of one low-poly figure.
  const SOLDIERS = 2000;
  const sim = new wasm.Sim(42n, 1, SOLDIERS);
  {
    const f = new Mesh();
    f.box(-0.18, -0.12, 0.0, -0.03, 0.12, 0.85, 3); // left leg
    f.box(0.03, -0.12, 0.0, 0.18, 0.12, 0.85, 3); // right leg
    f.box(-0.24, -0.15, 0.85, 0.24, 0.15, 1.45, -1); // torso (instance colour)
    f.box(-0.11, -0.11, 1.48, 0.11, 0.11, 1.75, 8); // head
    f.box(0.26, 0.05, 1.0, 0.33, 0.85, 1.08, 0); // rifle
    r.upload('figure', f.array());
  }
  let groundView = 60;
  const inst = new Float32Array(SOLDIERS * 4);
  const CELLS = 64;
  const chunks = new Map(); // "cx,cy" -> heights (metres)
  function chunk(cx, cy) {
    const k = cx + ',' + cy;
    if (!chunks.has(k)) {
      const h = wasm.planet_chunk(42n, 0, cx, cy, CELLS);
      const n = CELLS + 1, m = new Mesh(), hm = new Float32Array(h.length);
      for (let i = 0; i < h.length; i++) hm[i] = h[i] / 1000;
      const P = (i, j) => [cx * CELLS + i, cy * CELLS + j, hm[j * n + i]];
      const col = (z) => 10 + (((Math.floor(z * 8) % 4) + 4) % 4);
      for (let j = 0; j < CELLS; j++) for (let i = 0; i < CELLS; i++) {
        const a = P(i, j), b = P(i + 1, j), c = P(i + 1, j + 1), d = P(i, j + 1);
        m.tri(a, b, c, col(a[2])); m.tri(a, c, d, col(a[2]));
      }
      r.upload('terrain' + k, m.array());
      chunks.set(k, hm);
    }
    return chunks.get(k);
  }
  function evictChunks(gx, gy, keep) {
    for (const k of chunks.keys()) {
      const [x, y] = k.split(',').map(Number);
      if (Math.abs(x - gx) > keep || Math.abs(y - gy) > keep) { chunks.delete(k); r.drop('terrain' + k); }
    }
  }
  function heightAt(x, y) {
    const cx = Math.floor(x / CELLS), cy = Math.floor(y / CELLS);
    const hm = chunk(cx, cy);
    const i = Math.min(CELLS, Math.max(0, Math.round(x - cx * CELLS)));
    const j = Math.min(CELLS, Math.max(0, Math.round(y - cy * CELLS)));
    return hm[j * (CELLS + 1) + i];
  }

  // ---- system ----
  const bodies = [0, 1].map((i) => wasm.system_body_name(i));
  let docked = 0, flight = null, flightStart = 0;
  const trail = [];
  const KM = 1e6; // millimetres per kilometre
  const shipAt = (t) => {
    if (flight) {
      const el = t - flightStart, dur = flight.duration_ms();
      if (el < dur) { const p = flight.pos(flightStart + el); return [p[0] / KM, p[1] / KM, dur, el]; }
      docked = flight.dest; flight = null;
    }
    const b = wasm.system_body_pos(docked, t);
    return [b[0] / KM, b[1] / KM + 50, 0, 0];
  };
  let sysScale = 260000, yaw = Math.PI / 4;
  ui.canvas.addEventListener('wheel', (e) => {
    e.preventDefault();
    const k = e.deltaY > 0 ? 1.15 : 1 / 1.15;
    if (state.scene === 'system') sysScale = Math.min(500000, Math.max(30, sysScale * k));
    else if (state.scene === 'ground') groundView = Math.min(90, Math.max(8, groundView * k));
  }, { passive: false });
  ui.launch.addEventListener('click', () => {
    if (flight) return;
    const to = 1 - docked, t = simMs();
    flight = new wasm.Flight(docked, to, t, 2e9);
    flight.dest = to; flightStart = t;
  });

  for (const b of document.querySelectorAll('[data-scene]')) b.addEventListener('click', () => { state.scene = b.dataset.scene; });
  ui.fullres.addEventListener('change', () => { state.full = ui.fullres.checked; });
  const outlineBox = document.getElementById('outline');
  state.outline = outlineBox ? outlineBox.checked : true;
  outlineBox?.addEventListener('change', () => { state.outline = outlineBox.checked; });
  ui.shipSel.addEventListener('change', () => { state.shipIndex = Number(ui.shipSel.value); });

  function tickFps(now) {
    state.frames++;
    if (now - state.last > 500) { state.fps = (state.frames * 1000) / (now - state.last); state.frames = 0; state.last = now; }
  }

  // Software WebGPU runs on the CPU: cap it at 20 fps. Draw nothing while the tab or the viewer is hidden.
  const minFrameMs = r.software ? 50 : 0;
  let lastDraw = 0, visible = true;
  new IntersectionObserver((e) => { visible = e[0].isIntersecting; }).observe(ui.canvas);
  const gpuNote = r.software
    ? `SOFTWARE RENDERING (${r.adapterName}): the CPU is drawing, enable hardware WebGPU in Chrome`
    : r.adapterName;
  function render(now) {
    if (document.hidden || !visible || now - lastDraw < minFrameMs) { requestAnimationFrame(render); return; }
    lastDraw = now;
    try { draw(now); } catch (e) { ui.hud.textContent = `Render error: ${e.message}`; console.error(e); return; }
    requestAnimationFrame(render);
  }

  function draw(now) {
    const w = ui.canvas.width, h = ui.canvas.height;
    const internal = state.full ? [w, h] : [Math.round(w / 2), Math.round(h / 2)];
    const t = simMs();
    let text = '', cam;
    if (state.scene === 'ships') {
      const s = shipMesh(state.shipIndex);
      const size = Math.max(s.hi[0] - s.lo[0], s.hi[1] - s.lo[1], s.hi[2] - s.lo[2]);
      cam = { keys: ['ship' + state.shipIndex], target: [(s.lo[0] + s.hi[0]) / 2, (s.lo[1] + s.hi[1]) / 2, (s.lo[2] + s.hi[2]) / 2],
        yaw: t / 2500, viewHeight: size * 1.0, range: size * 3 };
      text = `${s.info.name}: ${s.info.triangles} triangles, ${(s.hi[1] - s.lo[1]).toFixed(1)} m long`;
    } else if (state.scene === 'ground') {
      sim.tick(t);
      const n = sim.derive(t, 1, -1e9, 1e9, -1e9, 1e9, SOLDIERS);
      const xs = sim.xs(), ys = sim.ys(), ph = sim.phases();
      let cx = 0, cy = 0;
      for (let i = 0; i < n; i++) { cx += xs[i]; cy += ys[i]; }
      if (n) { cx /= n; cy /= n; }
      for (let i = 0; i < n; i++) {
        inst.set([xs[i], ys[i], heightAt(xs[i], ys[i]), 6 + (ph[i] === 1 ? 1 : 0)], i * 4);
      }
      r.upload('soldierInst', inst.subarray(0, n * 4));
      const gx = Math.floor(cx / CELLS), gy = Math.floor(cy / CELLS);
      const rad = Math.ceil((groundView * 1.9) / CELLS / 2) + 1;
      const keys = [];
      for (let dy = -rad; dy <= rad; dy++) for (let dx = -rad; dx <= rad; dx++) {
        chunk(gx + dx, gy + dy); keys.push(`terrain${gx + dx},${gy + dy}`);
      }
      evictChunks(gx, gy, rad + 2);
      cam = { keys, instanced: [{ mesh: 'figure', instances: 'soldierInst' }], target: [cx, cy, heightAt(cx, cy)],
        yaw: Math.PI / 4, viewHeight: groundView, range: 600 };
      text = `${n} soldiers (one group, marching) drawn as instances of one figure; wheel zooms (${groundView.toFixed(0)} m tall view)`;
    } else {
      const m = new Mesh();
      const pix = sysScale / internal[1]; // km per internal pixel
      for (let i = 0; i < 2; i++) {
        const p = wasm.system_body_pos(i, t);
        m.ring(0, 0, (i === 0 ? 40000 : 90000), Math.max(pix * 0.6, 20), 128, 3);
        m.diamond(p[0] / KM, p[1] / KM, 0, Math.max(800, pix * 5), i === 0 ? 6 : 5);
      }
      m.diamond(0, 0, 0, Math.max(2500, pix * 9), 14);
      const [sx, sy, dur, el] = shipAt(t);
      if (flight && trail.length === 0 || (flight && t - (trail.at(-1)?.t ?? -1e9) > 700)) trail.push({ t, x: sx, y: sy });
      if (!flight && trail.length > 0 && t - trail.at(-1).t > 5000) trail.length = 0;
      for (const q of trail) m.diamond(q.x, q.y, 0, Math.max(120, pix * 1.2), 8);
      m.box(sx - pix * 2.5, sy - pix * 2.5, 0, sx + pix * 2.5, sy + pix * 2.5, pix * 5, 15);
      r.upload('system', m.array());
      cam = { keys: ['system'], target: [0, 0, 0], yaw, viewHeight: sysScale, range: sysScale * 4 };
      text = flight ? `Flying ${bodies[docked]} to ${bodies[flight.dest]}: ${(el / 1000).toFixed(0)} s of ${(dur / 1000).toFixed(0)} s`
        : `Docked at ${bodies[docked]}. Wheel zooms (${(sysScale / 1000).toFixed(0)} thousand km across).`;
      ui.launch.disabled = !!flight;
    }
    ui.launch.hidden = state.scene !== 'system';
    ui.shipSel.hidden = state.scene !== 'ships';
    r.frame({ ...cam, internal, outline: state.outline });
    if (pendingShot) {
      const c2 = document.createElement('canvas');
      c2.width = internal[0]; c2.height = internal[1];
      const g = c2.getContext('2d'); g.imageSmoothingEnabled = false;
      g.drawImage(ui.canvas, 0, 0, c2.width, c2.height);
      pendingShot(c2.toDataURL('image/png')); pendingShot = null;
    }
    tickFps(now);
    ui.hud.textContent = r.gpuError
      ? `GPU ERROR: ${r.gpuError.slice(0, 600)}`
      : `${text} | ${internal[0]}x${internal[1]} | ${state.fps.toFixed(0)} fps | ${gpuNote}`;
  }
  requestAnimationFrame(render);
  // Frame capture for remote checks: returns a PNG data URL of the next frame at the internal size.
  let pendingShot = null;
  window.scaliaShot = () => new Promise((res) => { pendingShot = res; });
  window.scaliaViewer = { state, simMs, get flight() { return flight; } };
}
