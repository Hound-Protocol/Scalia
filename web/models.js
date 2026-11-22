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
  ['walk (low ready)', 'combat run (rush)', 'stand (safe hang)', 'crouch', 'cover aim (ready up)', 'prone', 'standing aim (ready up)', 'sprint', 'cover (high ready)', 'walk while aiming', 'strafe left while aiming', 'strafe right while aiming', 'backpedal while aiming', 'crouched walk', 'lean left (corner)', 'lean right (corner)', 'crouched aim (over a wall)', 'stacked at a door', 'room sweep', 'grenade throw', 'reload', 'hand signal', 'drone launch', 'checking a corner (crouched sidestep, aiming)', 'dynamic entry (running, aiming)', 'wounded (down, on his back)', 'dragging a casualty (backwards)', 'first aid', 'killed (face down)', 'shot, falling back', 'killed, crumpling forward', 'killed on his back'].forEach((n, i) => pose.append(new Option(n, i)));
  ['prone aim', 'crawl forward while aiming', 'crawl left while aiming', 'crawl right while aiming', 'crawl backward while aiming']
    .forEach((n, i) => pose.append(new Option(n, 32 + i)));
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

  // Commander mode: the soldier driven by the keys through the game's own animation. Keys move him relative to
  // where he faces; he stays on the plinth while his legs do the moving.
  const held = new Set();
  let stance = 0, walk = false, aim = false;
  const send = () => {
    const x = (held.has('KeyD') ? 1 : 0) - (held.has('KeyA') ? 1 : 0);
    const y = (held.has('KeyW') ? 1 : 0) - (held.has('KeyS') ? 1 : 0);
    v.commander_keys(x, y, held.has('ShiftLeft') || held.has('ShiftRight'), walk, aim);
  };
  const commander = $('commander'), freeze = $('freeze'), seams = $('seams');
  commander.addEventListener('change', () => {
    v.set_commander(commander.checked);
    $('keys').hidden = !commander.checked;
    stance = 0; walk = false; aim = false; held.clear();
    if (commander.checked) { $('spin').checked = false; v.set_turntable(false); }
  });
  freeze.addEventListener('change', () => v.set_frozen(freeze.checked));
  seams.addEventListener('change', () => v.set_seams(seams.checked));
  const toggle = (box, f) => { box.checked = !box.checked; f(box.checked); };
  addEventListener('keydown', (e) => {
    if (e.target.tagName === 'SELECT' || e.target.tagName === 'INPUT' && e.target.type !== 'checkbox') return;
    if (e.code === 'Space') { e.preventDefault(); toggle(freeze, (on) => v.set_frozen(on)); return; }
    if (e.code === 'KeyJ') { toggle(seams, (on) => v.set_seams(on)); return; }
    if (!commander.checked) return;
    if (e.repeat) return;
    if (e.code === 'KeyC') { stance = stance === 1 ? 0 : 1; v.viewer_stance(stance); }
    else if (e.code === 'KeyZ') { stance = stance === 2 ? 0 : 2; v.viewer_stance(stance); }
    else if (e.code === 'KeyG') walk = !walk;
    else if (e.code === 'KeyQ') aim = !aim;
    else held.add(e.code);
    send();
  });
  addEventListener('keyup', (e) => { held.delete(e.code); if (commander.checked) send(); });
  addEventListener('blur', () => { held.clear(); if (commander.checked) send(); });

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
