// Full-screen game page. The Rust viewer draws everything; this file forwards input and the animation frame.
import init, * as wasm from './pkg/scalia_wasm.js';

const $ = (id) => document.getElementById(id);
const canvas = $('view'), hud = $('hud'), help = $('help');

function showError(msg) {
  const e = $('error');
  e.style.display = 'flex';
  e.textContent = msg;
}

const store = {
  get(k) { try { return localStorage.getItem(k); } catch { return null; } },
  set(k, v) { try { localStorage.setItem(k, v); } catch { /* storage unavailable */ } },
};

try {
  await init();
  if (!navigator.gpu) throw new Error('WebGPU is not available in this browser.');

  // Canvas in device pixels; the viewer renders at 1/scale of that and scales up with sharp pixels.
  const dpr = () => window.devicePixelRatio || 1;
  function fit() {
    const w = Math.max(1, Math.round(innerWidth * dpr())), h = Math.max(1, Math.round(innerHeight * dpr()));
    if (canvas.width !== w || canvas.height !== h) { canvas.width = w; canvas.height = h; }
    return [w, h];
  }
  fit();
  // Browsers do not tell wgpu the adapter's name; ask directly so a CPU renderer is flagged.
  let gpuNote = '';
  try {
    const a = await navigator.gpu.requestAdapter({ powerPreference: 'high-performance' });
    const info = a && a.info;
    if (!a) gpuNote = ' | no WebGPU adapter';
    else if (info && (info.isFallbackAdapter || /swiftshader|llvmpipe/i.test(`${info.architecture} ${info.description}`)))
      gpuNote = ' | SOFTWARE RENDERING: the CPU draws; enable hardware WebGPU (Vulkan) in Chrome';
    else if (info) gpuNote = ` | ${[info.vendor, info.architecture].filter(Boolean).join(' ')}`;
  } catch { /* informational only */ }
  const v = await wasm.start_viewer(canvas);
  const pixelScale = (h) => Math.max(1, Math.round(h / 300)); // about 300 internal rows on any screen
  function resize() { const [w, h] = fit(); v.resize(w, h); v.set_pixel_scale(pixelScale(h)); }
  addEventListener('resize', resize);
  resize();

  let note = '', noteUntil = 0;
  const flash = (msg) => { note = msg; noteUntil = performance.now() + 2500; };

  // Ship list.
  const sel = $('shipSel');
  for (let i = 0; wasm.shipped_ron(i) !== undefined; i++) sel.append(new Option(JSON.parse(wasm.blueprint_info(wasm.shipped_ron(i))).name, i));
  sel.append(new Option('Soldier', sel.options.length));
  sel.value = '5';
  sel.addEventListener('change', () => { v.set_ship(Number(sel.value)); showModelOpts(); });

  // Model viewer: pose, level of detail and colour variant for the soldier.
  const poseSel = $('poseSel'), detailSel = $('detailSel'), variantSel = $('variantSel');
  poseSel.append(new Option('all poses', -1));
  ['walk', 'run', 'stand', 'crouch', 'cover aim', 'prone', 'standing aim', 'sprint', 'walk while aiming'].forEach((n, i) => poseSel.append(new Option(n, i)));
  for (let i = 0; v.variant_name(i) !== undefined; i++) variantSel.append(new Option(v.variant_name(i), i));
  poseSel.addEventListener('change', () => v.preview_pose(Number(poseSel.value)));
  detailSel.addEventListener('change', () => v.set_model_detail(Number(detailSel.value)));
  variantSel.addEventListener('change', () => v.set_model_variant(Number(variantSel.value)));
  const showModelOpts = () => { $('modelOpts').hidden = !(scene === 'ships' && sel.value === String(sel.options.length - 1)); };

  // Scenes.
  let scene = 'ships';
  const setScene = (name) => {
    scene = name;
    v.set_scene(name);
    for (const b of document.querySelectorAll('[data-scene]')) b.classList.toggle('on', b.dataset.scene === name);
    showModelOpts();
    store.set('scalia.scene', name);
  };
  for (const b of document.querySelectorAll('[data-scene]')) b.addEventListener('click', () => setScene(b.dataset.scene));
  setScene(store.get('scalia.scene') || 'ground');

  // Toggles and actions.
  const fullres = $('fullres'), outline = $('outline');
  fullres.addEventListener('change', () => v.set_full(fullres.checked));
  // Full resolution for now (the chunky pixel look is switched off; R brings it back).
  fullres.checked = true; v.set_full(true);
  outline.addEventListener('change', () => v.set_outline(outline.checked));
  $('launch').addEventListener('click', () => v.launch(performance.now()));
  $('follow').addEventListener('click', () => v.follow());
  const toggleHelp = () => { help.classList.toggle('hidden'); store.set('scalia.help', help.classList.contains('hidden') ? '0' : '1'); };
  $('helpBtn').addEventListener('click', toggleHelp);
  if (store.get('scalia.help') === '0') help.classList.add('hidden');

  // Mouse: wheel zooms, drag pans.
  // One mouse-wheel notch (about 100 px) is one zoom step; trackpads send many small deltas, summed into steps.
  let wheelSum = 0, wheelAt = 0;
  canvas.addEventListener('wheel', (e) => {
    e.preventDefault();
    if (e.timeStamp - wheelAt > 300) wheelSum = 0; // a pause forgets leftovers
    wheelAt = e.timeStamp;
    const px = e.deltaMode === 1 ? e.deltaY * 33.4 : e.deltaMode === 2 ? e.deltaY * 800 : e.deltaY;
    // The first movement of a gesture steps at once, so a single notch never feels dead.
    const fresh = wheelSum === 0;
    wheelSum += px;
    const STEP = 100;
    if (fresh && Math.abs(wheelSum) < STEP) { v.wheel(Math.sign(wheelSum)); wheelSum = Math.sign(wheelSum) * -STEP + wheelSum; }
    while (Math.abs(wheelSum) >= STEP) {
      v.wheel(Math.sign(wheelSum));
      wheelSum -= Math.sign(wheelSum) * STEP;
    }
  }, { passive: false });
  // Left button: drag pans, a click without movement selects. Right button: order the selected group.
  let drag = null, moved = 0, box = null;
  const boxEl = document.createElement('div');
  boxEl.id = 'selbox'; boxEl.hidden = true;
  document.body.append(boxEl);
  const showBox = () => {
    const r = canvas.getBoundingClientRect();
    Object.assign(boxEl.style, {
      left: r.left + Math.min(box.x0, box.x1) + 'px', top: r.top + Math.min(box.y0, box.y1) + 'px',
      width: Math.abs(box.x1 - box.x0) + 'px', height: Math.abs(box.y1 - box.y0) + 'px',
    });
    boxEl.hidden = false;
  };
  canvas.addEventListener('contextmenu', (e) => e.preventDefault());
  canvas.addEventListener('pointerdown', (e) => {
    if (e.button === 2) {
      const msg = v.command(e.offsetX * dpr(), e.offsetY * dpr(), performance.now(), e.shiftKey);
      if (msg) flash(msg);
      return;
    }
    if (e.button !== 0) return;
    if (e.shiftKey && scene === 'ground') {
      // Shift + drag draws a selection box.
      box = { x0: e.offsetX, y0: e.offsetY, x1: e.offsetX, y1: e.offsetY };
      canvas.setPointerCapture(e.pointerId);
      showBox();
      return;
    }
    drag = [e.clientX, e.clientY]; moved = 0;
    canvas.setPointerCapture(e.pointerId);
    canvas.classList.add('dragging');
  });
  canvas.addEventListener('pointermove', (e) => {
    if (box) { box.x1 = e.offsetX; box.y1 = e.offsetY; showBox(); return; }
    if (!drag) return;
    const dx = e.clientX - drag[0], dy = e.clientY - drag[1];
    moved += Math.abs(dx) + Math.abs(dy);
    if (moved > 4) v.pan(dx * dpr(), dy * dpr());
    drag = [e.clientX, e.clientY];
  });
  const endDrag = (e) => {
    if (box) {
      if (e.type === 'pointerup') {
        const msg = v.select_box(box.x0 * dpr(), box.y0 * dpr(), box.x1 * dpr(), box.y1 * dpr(), performance.now());
        if (msg) flash(msg);
      }
      box = null;
      boxEl.hidden = true;
      return;
    }
    if (drag && moved <= 4 && e.type === 'pointerup') {
      const msg = v.click(e.offsetX * dpr(), e.offsetY * dpr(), performance.now());
      if (msg) flash(msg);
    }
    drag = null;
    canvas.classList.remove('dragging');
  };
  canvas.addEventListener('pointerup', endDrag);
  canvas.addEventListener('pointercancel', endDrag);

  // Keyboard: held keys pan and rotate every frame, others act once.
  const held = new Set();
  addEventListener('keydown', (e) => {
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLSelectElement) return;
    const k = e.key.toLowerCase();
    held.add(k);
    if (k === '1') setScene('ships');
    else if (k === '2') setScene('ground');
    else if (k === '3') setScene('planet');
    else if (k === '4') setScene('system');
    else if (k === ' ') { e.preventDefault(); v.launch(performance.now()); }
    else if (k === 'f') v.follow();
    else if (k === 'h') toggleHelp();
    else if (k === 'o') { outline.checked = !outline.checked; v.set_outline(outline.checked); }
    else if (k === 'r') { fullres.checked = !fullres.checked; v.set_full(fullres.checked); }
    else if (k === 't') flash(v.cycle_day_speed(performance.now()));
    else if (k === 'n') flash(v.cycle_army());
    else if (k === 'p') flash(v.cycle_stance());
    else if (k === 'l') { capIndex = (capIndex + 1) % CAPS.length; store.set('scalia.cap', String(capIndex)); flash(`frame cap: ${CAPS[capIndex] || 'off'}`); }
  });
  addEventListener('keyup', (e) => held.delete(e.key.toLowerCase()));
  addEventListener('blur', () => held.clear());

  // Performance panel: CPU and GPU bars against the frame budget, and a frame-time graph.
  const P = (id) => document.getElementById(id);
  const graph = P('pGraph').getContext('2d');
  const frameTimes = [];
  let lastFrameAt = 0, panelAt = 0;
  const level = (f) => (f < 0.5 ? '#7fb069' : f < 0.8 ? '#e9c46a' : '#e76f51');
  if (gpuNote.includes('SOFTWARE')) { P('pWarn').hidden = false; P('pWarn').textContent = 'Software rendering: the CPU is drawing. Enable Vulkan in Chrome.'; }
  function perfPanel(p, cpu, now, cap) {
    if (lastFrameAt) { frameTimes.push(now - lastFrameAt); if (frameTimes.length > 300) frameTimes.shift(); }
    lastFrameAt = now;
    if (now - panelAt < 200) return; // numbers 5 times a second, so they can be read
    panelAt = now;
    // Utilisation: the share of each second spent on the game = ms per frame x frames per second / 1000.
    const [fps, iw, ih, sh, sc, up] = p;
    const perSecond = (ms) => (ms * fps) / 1000;
    for (const b of document.querySelectorAll('#perf .budget')) b.style.left = '80%';
    const pct = (f) => `${Math.min(100, f * 100)}%`;
    const cpuUse = perSecond(cpu);
    P('sCpu').style.width = pct(cpuUse);
    P('sCpu').style.background = level(cpuUse);
    P('vCpu').textContent = `${(cpuUse * 100).toFixed(0)}%`;
    P('tCpu').title = `${cpu.toFixed(2)} ms per frame`;
    if (sh >= 0) {
      let x = 0;
      for (const [id, ms, col] of [['sSh', sh, '#5b8fd9'], ['sSc', sc, '#7fb069'], ['sUp', up, '#c08bd9']]) {
        const w = Math.min(100 - x, perSecond(ms) * 100);
        P(id).style.left = `${x}%`; P(id).style.width = `${w}%`; P(id).style.background = col;
        x += w;
      }
      const g = perSecond(sh + sc + up);
      P('vGpu').textContent = `${(g * 100).toFixed(0)}%`;
      P('vGpu').style.color = level(g);
      P('tGpu').title = `${(sh + sc + up).toFixed(2)} ms per frame (shadow ${sh.toFixed(2)}, scene ${sc.toFixed(2)}, upscale ${up.toFixed(2)})`;
    } else {
      P('vGpu').textContent = 'n/a';
    }
    P('pFps').textContent = fps.toFixed(0);
    const target = cap || 60;
    P('pFps').style.color = fps >= target * 0.95 ? '#7fb069' : fps >= target * 0.75 ? '#e9c46a' : '#e76f51';
    P('pInfo').textContent = `${iw}x${ih}${cap ? ` | cap ${cap}` : ''}${gpuNote && !gpuNote.includes('SOFTWARE') ? gpuNote : ''}`;
    // Graph: frame interval, 0-40 ms tall; the line marks 16.7 ms.
    const c = graph.canvas, W = c.width, H = c.height;
    graph.clearRect(0, 0, W, H);
    const y = (ms) => H - Math.min(H, (ms / 40) * H);
    graph.strokeStyle = '#e9c46a55'; graph.beginPath(); graph.moveTo(0, y(16.7)); graph.lineTo(W, y(16.7)); graph.stroke();
    for (let i = 0; i < frameTimes.length; i++) {
      const ms = frameTimes[i];
      graph.fillStyle = ms <= 18 ? '#7fb069' : ms <= 26 ? '#e9c46a' : '#e76f51';
      graph.fillRect(W - frameTimes.length + i, y(ms), 1, H - y(ms));
    }
  }

  // Frame pacing: optional cap (60 or 30); everything moves by time, so a lower rate only drops frames.
  const CAPS = [0, 60, 30];
  let capIndex = Number(store.get('scalia.cap') || 0) % CAPS.length;
  let cpuMs = 0, last = performance.now(), lastDraw = 0;
  function loop(now) {
    const cap = CAPS[capIndex];
    if (cap && now - lastDraw < 1000 / cap - 2) { requestAnimationFrame(loop); return; }
    lastDraw = now;
    const dt = Math.min(0.1, (now - last) / 1000);
    last = now;
    if (!document.hidden) {
      const speed = canvas.height * 0.8 * dt; // screen heights per second
      let px = 0, py = 0;
      if (held.has('a') || held.has('arrowleft')) px += speed;
      if (held.has('d') || held.has('arrowright')) px -= speed;
      if (held.has('w') || held.has('arrowup')) py += speed;
      if (held.has('s') || held.has('arrowdown')) py -= speed;
      if (px || py) v.pan(px, py);
      if (held.has('q')) v.rotate(-1.5 * dt);
      if (held.has('e')) v.rotate(1.5 * dt);
      const t0 = performance.now();
      const line = v.frame(now);
      cpuMs = cpuMs * 0.9 + (performance.now() - t0) * 0.1;
      hud.textContent = `${line}${now < noteUntil ? ` | ${note}` : ''}`;
      perfPanel(v.perf(), cpuMs, now, cap);
      const now_scene = v.scene_name();
      if (now_scene !== scene) { scene = now_scene; for (const b of document.querySelectorAll('[data-scene]')) b.classList.toggle('on', b.dataset.scene === scene); store.set('scalia.scene', scene); }
      $('launch').hidden = scene !== 'system';
      $('launch').disabled = v.flying();
      sel.hidden = scene !== 'ships';
    }
    requestAnimationFrame(loop);
  }
  requestAnimationFrame(loop);
  window.scaliaViewer = v;
} catch (e) {
  showError(`Scalia could not start: ${e.message || e}. It needs a browser with WebGPU (Chrome or Edge; on Linux, Chrome with Vulkan enabled).`);
  hud.textContent = '';
}

// A new deploy carries a new version stamp: poll for it and offer a reload.
(function watchForUpdates() {
  const mine = new URL(import.meta.url).searchParams.get('v');
  if (!mine) return; // local build
  const bar = document.getElementById('update');
  document.getElementById('update-reload').addEventListener('click', () => location.reload());
  const check = async () => {
    try {
      const r = await fetch('version.json', { cache: 'no-store' });
      const live = (await r.json()).v;
      if (live && live !== mine) bar.hidden = false;
    } catch (_) { /* offline: try again later */ }
  };
  setInterval(check, 60_000);
  check();
})();
