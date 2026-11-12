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
  const v = await wasm.start_viewer(canvas);
  const pixelScale = (h) => Math.max(1, Math.round(h / 300)); // about 300 internal rows on any screen
  function resize() { const [w, h] = fit(); v.resize(w, h); v.set_pixel_scale(pixelScale(h)); }
  addEventListener('resize', resize);
  resize();

  // Ship list.
  const sel = $('shipSel');
  for (let i = 0; wasm.shipped_ron(i) !== undefined; i++) sel.append(new Option(JSON.parse(wasm.blueprint_info(wasm.shipped_ron(i))).name, i));
  sel.value = '5';
  sel.addEventListener('change', () => v.set_ship(Number(sel.value)));

  // Scenes.
  let scene = 'ships';
  const setScene = (name) => {
    scene = name;
    v.set_scene(name);
    for (const b of document.querySelectorAll('[data-scene]')) b.classList.toggle('on', b.dataset.scene === name);
    store.set('scalia.scene', name);
  };
  for (const b of document.querySelectorAll('[data-scene]')) b.addEventListener('click', () => setScene(b.dataset.scene));
  setScene(store.get('scalia.scene') || 'ground');

  // Toggles and actions.
  const fullres = $('fullres'), outline = $('outline');
  fullres.addEventListener('change', () => v.set_full(fullres.checked));
  outline.addEventListener('change', () => v.set_outline(outline.checked));
  $('launch').addEventListener('click', () => v.launch(performance.now()));
  $('follow').addEventListener('click', () => v.follow());
  const toggleHelp = () => { help.classList.toggle('hidden'); store.set('scalia.help', help.classList.contains('hidden') ? '0' : '1'); };
  $('helpBtn').addEventListener('click', toggleHelp);
  if (store.get('scalia.help') === '0') help.classList.add('hidden');

  // Mouse: wheel zooms, drag pans.
  canvas.addEventListener('wheel', (e) => { e.preventDefault(); v.wheel(e.deltaY); }, { passive: false });
  let drag = null;
  canvas.addEventListener('pointerdown', (e) => { drag = [e.clientX, e.clientY]; canvas.setPointerCapture(e.pointerId); canvas.classList.add('dragging'); });
  canvas.addEventListener('pointermove', (e) => {
    if (!drag) return;
    v.pan((e.clientX - drag[0]) * dpr(), (e.clientY - drag[1]) * dpr());
    drag = [e.clientX, e.clientY];
  });
  const endDrag = () => { drag = null; canvas.classList.remove('dragging'); };
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
    else if (k === '3') setScene('system');
    else if (k === ' ') { e.preventDefault(); v.launch(performance.now()); }
    else if (k === 'f') v.follow();
    else if (k === 'h') toggleHelp();
    else if (k === 'o') { outline.checked = !outline.checked; v.set_outline(outline.checked); }
    else if (k === 'r') { fullres.checked = !fullres.checked; v.set_full(fullres.checked); }
  });
  addEventListener('keyup', (e) => held.delete(e.key.toLowerCase()));
  addEventListener('blur', () => held.clear());

  let last = performance.now();
  function loop(now) {
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
      hud.textContent = v.frame(now);
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
