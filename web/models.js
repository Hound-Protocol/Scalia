// Model viewer: one model at a time, orbit with the mouse, zoom with the wheel. Uses the game's own renderer.
import init, * as wasm from './pkg/scalia_wasm.js';

const $ = (id) => document.getElementById(id);
const canvas = $('view');

try {
  await init();
  if (!navigator.gpu) throw new Error('WebGPU is not available in this browser.');
  const dpr = () => window.devicePixelRatio || 1;
  const fit = () => {
    const w = Math.max(1, Math.round(innerWidth * dpr())), h = Math.max(1, Math.round(innerHeight * dpr()));
    if (canvas.width !== w || canvas.height !== h) { canvas.width = w; canvas.height = h; }
    return [w, h];
  };
  fit();
  const v = await wasm.start_viewer(canvas);
  const resize = () => { const [w, h] = fit(); v.resize(w, h); };
  addEventListener('resize', resize);
  resize();
  v.set_full(true);
  v.set_outline(false);
  v.set_scene('ships');

  // Models: the shipped blueprints, then the soldier.
  const model = $('model');
  for (let i = 0; wasm.shipped_ron(i) !== undefined; i++) model.append(new Option(JSON.parse(wasm.blueprint_info(wasm.shipped_ron(i))).name, i));
  model.append(new Option('Soldier', model.options.length));
  const soldier = () => model.value === String(model.options.length - 1);
  const pose = $('pose'), detail = $('detail'), variant = $('variant');
  pose.append(new Option('all poses', -1));
  ['walk (low ready)', 'combat run (rush)', 'stand (safe hang)', 'crouch', 'cover aim (ready up)', 'prone', 'standing aim (ready up)', 'sprint', 'cover (high ready)', 'walk while aiming', 'strafe left while aiming', 'strafe right while aiming', 'backpedal while aiming'].forEach((n, i) => pose.append(new Option(n, i)));
  for (let i = 0; v.variant_name(i) !== undefined; i++) variant.append(new Option(v.variant_name(i), i));
  const pick = () => { v.set_ship(Number(model.value)); $('soldierOpts').hidden = !soldier(); location.hash = model.value; };
  model.addEventListener('change', pick);
  pose.addEventListener('change', () => v.preview_pose(Number(pose.value)));
  detail.addEventListener('change', () => v.set_model_detail(Number(detail.value)));
  variant.addEventListener('change', () => v.set_model_variant(Number(variant.value)));
  $('spin').addEventListener('change', (e) => v.set_turntable(e.target.checked));
  $('light').addEventListener('input', (e) => v.set_model_light(Number(e.target.value)));
  $('outline').addEventListener('change', (e) => v.set_outline(e.target.checked));
  $('reset').addEventListener('click', () => { v.reset_model_view(); $('spin').checked = true; });
  model.value = location.hash ? location.hash.slice(1) : String(model.options.length - 1);
  pick();

  // Drag orbits (and stops the turntable), the wheel zooms.
  let drag = null;
  canvas.addEventListener('pointerdown', (e) => {
    drag = [e.clientX, e.clientY];
    canvas.setPointerCapture(e.pointerId);
    canvas.classList.add('dragging');
    $('spin').checked = false;
  });
  canvas.addEventListener('pointermove', (e) => {
    if (!drag) return;
    v.pan((e.clientX - drag[0]) * dpr(), (e.clientY - drag[1]) * dpr());
    drag = [e.clientX, e.clientY];
  });
  const end = () => { drag = null; canvas.classList.remove('dragging'); };
  canvas.addEventListener('pointerup', end);
  canvas.addEventListener('pointercancel', end);
  canvas.addEventListener('wheel', (e) => { e.preventDefault(); v.wheel(Math.sign(e.deltaY)); }, { passive: false });

  const info = $('info');
  const loop = (now) => {
    info.textContent = v.frame(now);
    requestAnimationFrame(loop);
  };
  requestAnimationFrame(loop);
} catch (err) {
  $('error').style.display = 'flex';
  $('error').textContent = String(err && err.message || err);
}
