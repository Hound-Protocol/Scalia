import init, * as wasm from './pkg/scalia_wasm.js';

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
  const cx = Math.trunc(Number(document.getElementById('cx').value) || 0);
  const cy = Math.trunc(Number(document.getElementById('cy').value) || 0);
  const cells = 64;
  const heights = wasm.planet_chunk(seed, cx, cy, cells);
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
    `64 m by 64 m patch, 1 m samples. Height ${(lo / 1000).toFixed(2)} m to ${(hi / 1000).toFixed(2)} m (stretched to full grey). Chunk hash ${wasm.planet_chunk_hash(seed, cx, cy, cells)}.`;
}

try {
  await init();
  const list = document.getElementById('blueprints');
  for (let i = 0; ; i++) {
    const ron = wasm.shipped_ron(i);
    if (ron === undefined) break;
    list.append(card(ron));
  }
  for (const id of ['seed', 'cx', 'cy']) document.getElementById(id).addEventListener('input', drawChunk);
  drawChunk();
  status.textContent = 'Everything below is computed in WebAssembly by the same Rust core as the native build.';
} catch (e) {
  status.textContent = `Failed to start: ${e}`;
}
