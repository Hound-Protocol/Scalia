import init, * as wasm from './pkg/scalia_wasm.js';
import { startViewer } from './viewer.js';

const status = document.getElementById('status');

function card(ron) {
  const info = JSON.parse(wasm.blueprint_info(ron));
  const div = document.createElement('div');
  div.className = 'card';
  const title = document.createElement('strong');
  title.textContent = `${info.name} (${info.class})`;
  const holder = document.createElement('div');
  holder.innerHTML = wasm.blueprint_svg(ron); // generated and escaped by the Rust drawing code
  const meta = document.createElement('div');
  meta.className = 'meta';
  meta.textContent = `${info.id} | ${info.triangles} triangles | mesh hash ${info.mesh_hash}`;
  div.append(title, holder, meta);
  return div;
}

function drawChunk() {
  const seed = BigInt(Math.max(0, Math.floor(Number(document.getElementById('seed').value) || 0)));
  const face = Number(document.getElementById('face').value);
  const cx = Math.trunc(Number(document.getElementById('cx').value) || 0);
  const cy = Math.trunc(Number(document.getElementById('cy').value) || 0);
  const cells = 64;
  const heights = wasm.planet_chunk(seed, face, cx, cy, cells);
  let lo = Infinity, hi = -Infinity;
  for (const h of heights) { if (h < lo) lo = h; if (h > hi) hi = h; }
  const canvas = document.getElementById('map');
  const ctx = canvas.getContext('2d');
  const img = ctx.createImageData(cells + 1, cells + 1);
  const span = Math.max(1, hi - lo);
  heights.forEach((h, i) => {
    const g = Math.round(((h - lo) / span) * 255);
    img.data.set([g, g, g, 255], i * 4);
  });
  ctx.putImageData(img, 0, 0);
  document.getElementById('mapinfo').textContent =
    `64 m by 64 m patch, 1 m samples. Height ${(lo / 1000).toFixed(2)} m to ${(hi / 1000).toFixed(2)} m (stretched to full grey). Chunk hash ${wasm.planet_chunk_hash(seed, face, cx, cy, cells)}.`;
}

// The Rust renderer: JavaScript only forwards input and the animation frame.
async function startRustViewer(ui) {
  const v = await wasm.start_viewer(ui.canvas);
  let scene = 'ships', visible = true;
  new IntersectionObserver((e) => { visible = e[0].isIntersecting; }).observe(ui.canvas);
  for (const b of document.querySelectorAll('[data-scene]')) b.addEventListener('click', () => { scene = b.dataset.scene; v.set_scene(scene); });
  ui.shipSel.addEventListener('change', () => v.set_ship(Number(ui.shipSel.value)));
  ui.fullres.addEventListener('change', () => v.set_full(ui.fullres.checked));
  const outline = document.getElementById('outline');
  outline.addEventListener('change', () => v.set_outline(outline.checked));
  ui.canvas.addEventListener('wheel', (e) => { e.preventDefault(); v.wheel(e.deltaY); }, { passive: false });
  ui.launch.addEventListener('click', () => v.launch(performance.now()));
  function loop(now) {
    if (!document.hidden && visible) {
      ui.hud.textContent = v.frame(now);
      ui.launch.hidden = scene !== 'system';
      ui.shipSel.hidden = scene !== 'ships';
      ui.launch.disabled = v.flying();
    }
    requestAnimationFrame(loop);
  }
  requestAnimationFrame(loop);
  window.scaliaRust = v;
}

try {
  await init();
  const list = document.getElementById('blueprints');
  for (let i = 0; ; i++) {
    const ron = wasm.shipped_ron(i);
    if (ron === undefined) break;
    list.append(card(ron));
  }
  for (const id of ['seed', 'face', 'cx', 'cy']) document.getElementById(id).addEventListener('input', drawChunk);
  drawChunk();
  const sel = document.getElementById('shipSel');
  for (let i = 0; wasm.shipped_ron(i) !== undefined; i++) sel.append(new Option(JSON.parse(wasm.blueprint_info(wasm.shipped_ron(i))).name, i));
  sel.value = '5';
  const ui = { canvas: document.getElementById('view'), hud: document.getElementById('hud'), launch: document.getElementById('launch'), shipSel: sel, fullres: document.getElementById('fullres') };
  try {
    if (new URLSearchParams(location.search).has('js')) await startViewer(wasm, ui);
    else await startRustViewer(ui);
  } catch (e) {
    ui.hud.textContent = `The viewer needs WebGPU: ${e.message || e}`;
  }
  status.textContent = 'Everything below is computed in WebAssembly by the same Rust core as the native build.';
} catch (e) {
  status.textContent = `Failed to start: ${e}`;
}
