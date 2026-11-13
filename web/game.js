// Full-screen game page. The Rust viewer draws everything; this file forwards input and the animation frame.
import init, * as wasm from './pkg/scalia_wasm.js';

const $ = (id) => document.getElementById(id);
const canvas = $('view'), hud = $('hud');

function showError(msg) {
  const e = $('error');
  e.style.display = 'flex';
  e.textContent = msg;
}

const store = {
  get(k) { try { return localStorage.getItem(k); } catch { return null; } },
  set(k, v) { try { localStorage.setItem(k, v); } catch { /* storage unavailable */ } },
};

// Background music (placeholder track): starts at 25% once the player enters the game, loops, and can be turned
// off or turned down on the tablet's settings page. The choice is remembered.
const music = (() => {
  const el = new Audio('./sfx/music-vance.mp3');
  el.loop = true;
  el.preload = 'auto';
  const saved = parseFloat(store.get('music.volume'));
  const m = {
    on: store.get('music.on') !== '0',
    vol: Number.isFinite(saved) ? Math.min(Math.max(saved, 0), 1) : 0.25,
    started: false,
    start() { m.started = true; m.apply(); },
    apply() {
      el.volume = m.vol;
      if (m.on && m.started && m.vol > 0) el.play().catch(() => { /* blocked until a click */ });
      else el.pause();
    },
    set(on, vol) { m.on = on; m.vol = vol; store.set('music.on', on ? '1' : '0'); store.set('music.volume', String(vol)); m.apply(); },
  };
  return m;
})();

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

  // Scenes.
  let scene = 'ships';
  const setScene = (name) => {
    scene = name;
    v.set_scene(name);
    store.set('scalia.scene', name);
  };
  setScene(store.get('scalia.scene') || 'ground');

  // Toggles and actions.
  // Full resolution for now (the chunky pixel look is switched off; R brings it back).
  let fullres = true, outline = false;
  v.set_full(true);

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
    if (scene === 'ground') return; // units are commanded through the tablet's map
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

  // The commander's tablet: Tab raises it; the map tab looks through to the game view (which then zooms all the way),
  // the roster lists the units, the comms hub is a placeholder for talking to other commanders.
  let tabletUp = false, page = 'home';
  const tablet = $('tablet');
  const PAGE_NAME = { home: 'Home', map: 'Map', roster: 'Roster', comms: 'Comms hub', settings: 'Settings', manual: 'Field Manual', changelog: 'Changelog' };
  // The header: who you are and how many you lead (the name and battalion are yours to set in Settings).
  const ident = { cmdr: store.get('cmdr.name') || 'Commander', batt: store.get('batt.name') || '1st Battalion' };
  const showIdent = () => { $('pdaCmdr').textContent = ident.cmdr; $('pdaBatt').textContent = ident.batt; };
  showIdent();
  // The strength figure counts to its new value instead of jumping.
  let shownSoldiers = 0, soldierAnim = 0;
  const showSoldiers = (n) => {
    if (n === shownSoldiers) return;
    const from = shownSoldiers, t0 = performance.now();
    shownSoldiers = n;
    cancelAnimationFrame(soldierAnim);
    const step = (t) => {
      const k = Math.min((t - t0) / 650, 1), e = 1 - (1 - k) ** 3;
      $('pdaSoldiers').textContent = Math.round(from + (n - from) * e).toLocaleString('en-GB');
      if (k < 1) soldierAnim = requestAnimationFrame(step);
    };
    soldierAnim = requestAnimationFrame(step);
  };
  const soldierCount = () => { try { return JSON.parse(v.roster()).reduce((a, u) => a + u.soldiers, 0); } catch { return shownSoldiers; } };
  // Mission clock: time since the commander entered the field.
  const t0Mission = performance.now();
  const showClock = () => {
    const s = Math.floor((performance.now() - t0Mission) / 1000);
    $('pdaClock').textContent = [s / 3600, (s / 60) % 60, s % 60].map((x) => String(Math.floor(x)).padStart(2, '0')).join(':');
  };
  $('pdaClock').title = 'Mission time';
  // Each time it comes up the tablet boots: about a second of Linux kernel and systemd lines, a login, then the
  // command UI starts. Timestamps and a few figures differ every time.
  const term = $('pdaTerm');
  let bootRun = 0;
  const bootTerminal = () => {
    const run = ++bootRun;
    if (matchMedia('(prefers-reduced-motion: reduce)').matches) return Promise.resolve();
    const esc = (t) => t.replace(/&/g, '&amp;').replace(/</g, '&lt;');
    const ts = (sec) => `<span class="ts">[${sec.toFixed(6).padStart(12)}]</span> `;
    const ok = (t) => `[<span class="ok">  OK  </span>] ${esc(t)}`;
    const r = (a, b) => a + Math.random() * (b - a);
    const mem = 3900000 + Math.floor(r(0, 90000));
    const lines = [
      ts(0) + 'Linux version 6.8.12-scalia-rt (build@fob-forge) (gcc 13.2.0) #1 SMP PREEMPT_RT',
      ts(r(0.001, 0.003)) + 'Machine model: Hound Protocol CMD-7 rugged field tablet',
      ts(r(0.01, 0.02)) + `Memory: ${mem}K/4194304K available`,
      ts(r(0.2, 0.3)) + 'mmcblk0: p1 p2 p3',
      ts(r(0.5, 0.6)) + 'EXT4-fs (mmcblk0p2): mounted filesystem, read-only',
      ts(r(0.9, 1.1)) + 'radio-mesh: battalion net transceiver on wlan1, 16 channels',
      ts(r(1.2, 1.4)) + 'gps0: fix acquired, 9 satellites',
      ok('Started Journal Service.'),
      ok('Mounted /var/field.'),
      ok('Started Battalion net (radio-mesh.service).'),
      ok('Started Position service (gpsd.service).'),
      ok('Reached target Graphical Interface.'),
      '',
      `<span class="hi">CMD-OS 4.2 LTS</span>  (GNU/Linux 6.8.12-scalia-rt aarch64)`,
      `<span class="pr">${esc(ident.cmdr.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '') || 'commander')}@cmd7:~$</span> `,
    ];
    term.className = 'term on';
    term.innerHTML = '';
    return new Promise((done) => {
      const t0 = performance.now();
      const tick = (t) => {
        if (run !== bootRun || !tabletShown) { term.className = 'term'; done(); return; }
        const k = (t - t0) / 1000;
        // Every line shows, from the first: they arrive over 0.75 s, the command is typed, and the finished screen
        // holds a moment before the UI takes over.
        const n = Math.min(lines.length, Math.floor(lines.length * Math.min(k / 0.75, 1)) + 1);
        const cmd = 'startx cmdui'.slice(0, Math.max(0, Math.floor((k - 0.78) / 0.018)));
        term.innerHTML = '<div class="log">' + lines.slice(0, n).join('\n') + (n === lines.length ? esc(cmd) : '') + '<span class="cur"></span></div>';
        if (k < 1.2) { requestAnimationFrame(tick); return; }
        term.className = 'term off';
        setTimeout(() => { if (run === bootRun) term.className = 'term'; }, 170);
        done();
      };
      requestAnimationFrame(tick);
    });
  };
  // Radio traffic heard and sent, newest first, for the home page.
  const traffic = [];
  const pdaStatus = () => { $('pdaStatus').textContent = PAGE_NAME[page] || ''; };
  // A burst of glitch lines and big pixel blocks over the glass while a page changes (sound to come: onGlitch).
  let onGlitch = null;
  const glitch = () => {
    if (!tablet.classList.contains('up') || matchMedia('(prefers-reduced-motion: reduce)').matches) return;
    const disp = tablet.querySelector('.display');
    disp.querySelector('.glitch')?.remove();
    const box = document.createElement('div');
    box.className = 'glitch';
    const r = (a, b) => a + Math.random() * (b - a);
    const cols = ['rgba(224,165,38,0.55)', 'rgba(205,191,148,0.45)', 'rgba(111,207,90,0.4)', 'rgba(120,200,230,0.35)', 'rgba(255,255,255,0.5)'];
    const pick = () => cols[Math.floor(Math.random() * cols.length)];
    let html = '';
    for (let k = 0; k < 9; k++) {
      html += `<i class="ln" style="top:${r(0, 100).toFixed(1)}%;height:${Math.round(r(1, 5))}px;background:${pick()};--x:${Math.round(r(-40, 40))}px;--d:${r(0.06, 0.14).toFixed(3)}s;--w:${r(0, 0.1).toFixed(3)}s"></i>`;
    }
    for (let k = 0; k < 7; k++) {
      const sz = Math.round(r(10, 34)) * 2;
      html += `<i class="px" style="left:${r(0, 96).toFixed(1)}%;top:${r(0, 92).toFixed(1)}%;width:${sz * Math.round(r(1, 4))}px;height:${sz}px;background:${pick()};--d:${r(0.04, 0.09).toFixed(3)}s;--w:${r(0, 0.13).toFixed(3)}s"></i>`;
    }
    box.innerHTML = html;
    disp.appendChild(box);
    setTimeout(() => box.remove(), 260);
    if (onGlitch) onGlitch();
  };
  // Zooming the map glitches the glass for as long as the wheel moves (and while the new detail settles), hiding the pop-in.
  // Zooming the map: a very fast blur while the wheel moves, hiding the detail popping in. No glitch effects.
  let zoomBlurTimer = 0;
  const zoomGlitch = () => {
    const cv = $('mapCanvas');
    cv.classList.add('zoomblur');
    clearTimeout(zoomBlurTimer);
    zoomBlurTimer = setTimeout(() => cv.classList.remove('zoomblur'), 170);
  };
  const setPage = (p) => {
    // The page that comes in wipes across the glass (only when the page changes, not on a redraw).
    if (p !== page) {
      const el = { map: $('pageMap') }[p] || $(p);
      if (el) { el.classList.remove('enter'); void el.offsetWidth; el.classList.add('enter'); }
      glitch();
    }
    page = p;
    for (const b of tablet.querySelectorAll('[data-page]')) b.classList.toggle('on', b.dataset.page === p);
    $('pageMap').hidden = p !== 'map';
    $('home').hidden = p !== 'home';
    if (p === 'home') drawHome();
    if (p === 'map') sizeMap();
    $('roster').hidden = p !== 'roster';
    $('comms').hidden = p !== 'comms';
    $('settings').hidden = p !== 'settings';
    $('manual').hidden = p !== 'manual';
    $('changelog').hidden = p !== 'changelog';
    if (p === 'changelog') drawChangelog();
    if (p === 'roster') drawRoster();
    pdaStatus();
  };
  // Tab: the commander raises the tablet. The camera rises above him, holds, then dives onto his hands (the
  // viewer animates it); once it arrives the close-up takes over. Putting it away reverses that.
  let tabletShown = false;
  const whoosh = new Audio('./sfx/dive-whoosh.mp3'); whoosh.preload = 'auto';
  const sfx = (src, vol) => { const a = new Audio(src); a.preload = 'auto'; a.volume = vol; return () => { const b = a.cloneNode(); b.volume = vol; b.play().catch(() => {}); }; };
  const pdaOpen = sfx('./sfx/pda-open.mp3', 0.7), pdaClose = sfx('./sfx/pda-close.mp3', 0.7), pdaTap = sfx('./sfx/pda-tap.mp3', 0.6);
  // Every button on the tablet, on screen or on the device, taps.
  tablet.addEventListener('click', (e) => { if (e.target.closest('button')) pdaTap(); }, true);
  const DIVE_AT_MS = 550; // matches the viewer's CINE_TURN_MS + CINE_HOLD_MS: the zoom begins
  let diveSound = 0;
  const setTablet = (up) => {
    if (up === tabletUp) return;
    tabletUp = up;
    $('fieldHint').hidden = up;
    if (up) {
      // The close-up's box (CSS: 80% of the width, at most 84% of the height, 2164:1139): the dive frames the
      // tablet to cover exactly that much of the screen.
      const w = Math.min(innerWidth * 0.8, innerHeight * 0.84 * 2164 / 1139);
      v.set_tablet_fit((w * 1139 / 2164) / innerHeight);
      v.set_tablet(true);
      // The swoosh as the camera starts its dive (after the rise and the half-second hold).
      clearTimeout(diveSound);
      diveSound = setTimeout(() => { if (tabletUp) { const a = whoosh.cloneNode(); a.volume = 0.7; a.play().catch(() => {}); } }, DIVE_AT_MS);
      return;
    }
    if (tabletShown) pdaClose();
    tabletShown = false;
    document.body.classList.remove('pda');
    tablet.classList.remove('up');
    closeActions();
    setTimeout(() => { if (!tabletUp) v.set_tablet(false); }, 220);
  };
  // The commander's footsteps on grass: one step each time a foot lands (gait phase 0.25 and 0.75), a little
  // different every time.
  const stepSound = new Audio('./sfx/steps-grass.mp3'); stepSound.preload = 'auto';
  let lastPhase = -1;
  const footsteps = () => {
    const ph = v.scene_name() === 'ground' ? v.commander_phase() : -1;
    if (ph >= 0 && lastPhase >= 0) {
      const crossed = (at) => (lastPhase < at && ph >= at) || (lastPhase > ph && (lastPhase < at || ph >= at));
      if (crossed(0.25) || crossed(0.75)) {
        const a = stepSound.cloneNode();
        a.volume = (v.commander_gait() === 2 ? 0.7 : 0.5) * (0.85 + Math.random() * 0.3);
        a.playbackRate = 0.9 + Math.random() * 0.2;
        a.play().catch(() => {});
      }
    }
    lastPhase = ph;
  };
  // Called every frame: shows the close-up once the camera has reached the tablet.
  const syncTablet = () => {
    if (tabletUp && !tabletShown && v.tablet_view() === 2) {
      tabletShown = true;
      pdaOpen();
      document.body.classList.add('pda');
      tablet.classList.add('up');
      const disp = tablet.querySelector('.display');
      disp.classList.remove('boot');
      bootTerminal().then(() => { if (tabletShown) { void disp.offsetWidth; disp.classList.add('boot'); } });
      showSoldiers(soldierCount()); showClock();
      setPage(page);
      setTimeout(() => { if (tabletShown && page === 'map') sizeMap(); }, 450);
    }
  };
  for (const b of tablet.querySelectorAll('.hw')) b.addEventListener('click', () => {
    const k = b.dataset.hw;
    if (k === 'close') setTablet(false);
    else setPage(k);
  });
  // Home: the overview the tablet opens on.
  const drawHome = () => {
    const units = JSON.parse(v.roster());
    const soldiers = units.reduce((n, u) => n + u.soldiers, 0);
    const moving = units.filter((u) => u.status !== 'holding').length;
    const captains = units.filter((u) => u.captain);
    const squadsOf = (c) => units.filter((u) => u.boss === c.id).length;
    const ro = (k, val, sub, bar) => `<div class="ro bk"><div class="k">${k}</div><div class="v${val === '—' ? ' none' : ''}">${val}</div>${
      bar === undefined ? '' : `<div class="bar"><i style="width:${Math.round(bar * 100)}%"></i></div>`}<div class="s">${sub}</div></div>`;
    const capRows = captains.length ? captains.map((c) => `<div class="cap"><span class="n">${c.name}</span><span class="t">${squadsOf(c)} squad${squadsOf(c) === 1 ? '' : 's'}</span><span class="d">${c.task || 'Awaiting a goal'}</span></div>`).join('')
      : '<p class="note">No captains in the battalion.</p>';
    const rx = traffic.length ? traffic.slice(0, 6).map((m) => `<div class="rx${m.me ? ' me' : ''}"><span class="when">${m.when}</span><span><span class="who">${m.name}</span> ${m.text}</span></div>`).join('')
      : '<p class="note">The net is quiet. Orders you give and reports from the field appear here.</p>';
    $('home').innerHTML = `<div class="readouts">
        ${ro('Strength', soldiers.toLocaleString('en-GB'), `${units.length} units, ${moving} on the move`, moving / Math.max(units.length, 1))}
        ${ro('Casualties', '0', 'None wounded or lost')}
        ${ro('Supplies', '—', 'Not tracked yet')}
        ${ro('Morale', '—', 'Not tracked yet')}
      </div>
      <div class="cols">
        <div class="col bk"><h3>Chain of command</h3>${capRows}</div>
        <div class="col bk"><h3>Radio traffic</h3>${rx}</div>
      </div>`;
  };
  // The map: its own canvas, drawn by the viewer with the map's camera (picture in picture).
  const mapCanvas = $('mapCanvas');
  let mapAttached = false;
  const sizeMap = () => {
    // Layout size (offsetWidth), not the on-screen rect: the tablet's scale-in transform would shrink the latter.
    const r = { width: mapCanvas.offsetWidth, height: mapCanvas.offsetHeight };
    const w = Math.max(1, Math.round(r.width * dpr())), h = Math.max(1, Math.round(r.height * dpr()));
    if (mapCanvas.width !== w || mapCanvas.height !== h) { mapCanvas.width = w; mapCanvas.height = h; }
    if (!mapAttached && r.width > 0) { v.attach_map(mapCanvas); mapAttached = true; }
    if (mapAttached) v.resize_map(w, h);
  };
  addEventListener('resize', () => { if (tabletUp && page === 'map') sizeMap(); });
  // The map's box changes size as the tablet scales in (and with the window): keep its pixels matched to it, or the
  // map stretches and clicks and zooms land in the wrong place.
  new ResizeObserver(() => { if (tabletUp && page === 'map') sizeMap(); }).observe(mapCanvas);
  const onMap = (fn) => { v.use_map(true); try { return fn(); } finally { v.use_map(false); } };
  $('mapHome').addEventListener('click', () => v.map_center_commander());
  let mDrag = null, mMoved = 0, mBox = null;
  mapCanvas.addEventListener('contextmenu', (e) => e.preventDefault());
  mapCanvas.addEventListener('pointerdown', (e) => {
    if (e.button !== 0) return;
    mapCanvas.setPointerCapture(e.pointerId);
    if (e.shiftKey) { mBox = [e.offsetX, e.offsetY]; return; }
    mDrag = [e.clientX, e.clientY]; mMoved = 0;
  });
  mapCanvas.addEventListener('pointermove', (e) => {
    if (!mDrag) return;
    const dx = e.clientX - mDrag[0], dy = e.clientY - mDrag[1];
    mMoved += Math.abs(dx) + Math.abs(dy);
    if (mMoved > 4) onMap(() => v.pan(dx * dpr(), dy * dpr()));
    mDrag = [e.clientX, e.clientY];
  });
  mapCanvas.addEventListener('pointerup', (e) => {
    if (mBox) { const msg = onMap(() => v.select_box(mBox[0] * dpr(), mBox[1] * dpr(), e.offsetX * dpr(), e.offsetY * dpr(), performance.now())); if (msg) flash(msg); mBox = null; return; }
    if (mDrag && mMoved <= 4) mapClick(e);
    mDrag = null;
  });
  // Orders: click a unit, then click a place. An actions window asks what the unit should do there; its AI says
  // whether it can. The order then goes out over the radio, and the unit moves once it has answered.
  const actionsBox = $('actions');
  const closeActions = () => { actionsBox.hidden = true; };
  const mapClick = (e) => {
    closeActions();
    const px = e.offsetX * dpr(), py = e.offsetY * dpr();
    const unit = onMap(() => v.unit_at(px, py));
    if (unit >= 0 || !onMap(() => v.selection_name())) {
      const msg = onMap(() => v.click(px, py, performance.now())); if (msg) flash(msg); return;
    }
    const [x, y] = onMap(() => v.map_point(px, py));
    const acts = JSON.parse(v.actions_at(x, y));
    const g4 = (m) => String(((Math.round(m / 10) % 10000) + 10000) % 10000).padStart(4, '0'); // 10 m squares
    const grid = `${g4(x)} ${g4(y)}`;
    actionsBox.innerHTML = `<div class="head">${onMap(() => v.selection_name())} → grid ${grid}</div>` + acts.map((a) =>
      `<button data-id="${a.id}" ${a.ok ? '' : 'disabled'} title="${a.reason}">${a.label}${a.ok ? '' : ` <small>${a.reason}</small>`}</button>`).join('') +
      '<button class="cancel">Cancel</button>';
    const r = mapCanvas.getBoundingClientRect();
    actionsBox.style.left = `${r.left + e.offsetX + 8}px`; actionsBox.style.top = `${r.top + e.offsetY + 8}px`;
    actionsBox.hidden = false;
    for (const b of actionsBox.querySelectorAll('button[data-id]')) b.addEventListener('click', () => {
      closeActions(); radioOrder(Number(b.dataset.id), b.textContent.trim(), x, y, grid);
    });
    actionsBox.querySelector('.cancel').addEventListener('click', closeActions);
  };
  // The walkie-talkie: comes up on screen, the commander speaks the order, waits for the unit, then it moves.
  const radio = $('radio');
  const VERB = { 0: ['move to', 'moving'], 1: ['defend', 'defending'], 2: ['assault', 'assaulting'], 3: ['support at', 'supporting'], 10: ['take', 'taking'], 11: ['hold', 'holding'] };
  // The handset's spectrum: 20 bars, log-spaced from 40 Hz to 10 kHz. Every sound the game plays itself goes
  // through an analyser (radio clicks now; recorded voices later) and drives them for real. The browser's speech
  // voice cannot be listened to by the page, so while it talks the bars pulse on each word it reports, in the
  // spectral shape of a voice on a radio band.
  const BANDS = 20, F_LO = 40, F_HI = 10000;
  let actx = null, analyser = null, fbins = null;
  const audioGraph = () => {
    if (actx) return actx;
    try {
      actx = new AudioContext();
      analyser = actx.createAnalyser(); analyser.fftSize = 4096; analyser.smoothingTimeConstant = 0.55;
      analyser.connect(actx.destination);
      fbins = new Float32Array(analyser.frequencyBinCount);
    } catch (_) { actx = null; }
    return actx;
  };
  // Plays a sound through the analyser (falls back to plain playback before audio is allowed).
  const playAnalysed = (proto, vol) => new Promise((done) => {
    const a = proto.cloneNode(); a.volume = vol;
    a.onended = done; a.onerror = done; setTimeout(done, 1500);
    const ctx = audioGraph();
    if (ctx) { try { if (ctx.state === 'suspended') ctx.resume(); ctx.createMediaElementSource(a).connect(analyser); } catch (_) { /* plain */ } }
    a.play().catch(done);
  });
  const edges = Array.from({ length: BANDS + 1 }, (_, k) => F_LO * Math.pow(F_HI / F_LO, k / BANDS));
  let wordEnergy = 0, wordAt = 0;
  // A voice heard over a radio: most energy between 300 Hz and 3 kHz, little below or above.
  const voiceShape = edges.slice(0, BANDS).map((f, k) => { const g = Math.sqrt(f * edges[k + 1]); return Math.exp(-Math.pow(Math.log(g / 1100) / 1.0, 2)); });
  const waveBars = () => [...document.querySelectorAll('#radio .wave i')];
  // The green oscilloscope trace, always on: flat with a faint hiss when the net is quiet, swinging with the voice
  // when someone speaks (what the analyser hears, plus the word bursts for the synthesised voices it cannot hear).
  let scopeData = null;
  const drawScope = () => {
    const cv = document.querySelector('#radio .wave .scope');
    if (!cv) return;
    const w = cv.clientWidth, h = cv.clientHeight;
    if (!w || !h) return;
    if (cv.width !== w * 2 || cv.height !== h * 2) { cv.width = w * 2; cv.height = h * 2; }
    const g = cv.getContext('2d');
    g.clearRect(0, 0, cv.width, cv.height);
    if (analyser) { scopeData = scopeData || new Float32Array(analyser.fftSize); analyser.getFloatTimeDomainData(scopeData); }
    const t = performance.now() / 1000, n = 120;
    const e = Math.min(1, wordEnergy * Math.exp(-(performance.now() - wordAt) / 160));
    g.beginPath();
    for (let i = 0; i <= n; i++) {
      const x = (i / n) * cv.width;
      const sig = scopeData ? scopeData[Math.floor((i / n) * (scopeData.length - 1))] : 0;
      const hiss = 0.04 * Math.sin(i * 0.9 + t * 13) * Math.sin(i * 0.37 - t * 7) + 0.025 * (Math.random() - 0.5);
      // A voice-like wave: a few formant-ish partials under a syllable envelope, scrolling across the screen.
      const ph = i * 0.42 - t * 38;
      const voice = e * (0.55 * Math.sin(ph) + 0.3 * Math.sin(ph * 2.7 + 1.3) + 0.15 * Math.sin(ph * 5.3)) * (0.6 + 0.4 * Math.sin(i * 0.09 - t * 9));
      const y = cv.height / 2 - Math.max(-1, Math.min(1, sig * 3 + hiss + voice * 0.9)) * cv.height * 0.45;
      if (i === 0) g.moveTo(x, y); else g.lineTo(x, y);
    }
    g.strokeStyle = '#6dff8f'; g.lineWidth = 2; g.shadowColor = '#3dff6a'; g.shadowBlur = 6;
    g.stroke();
  };
  const drawSpectrum = () => {
    const bars = waveBars();
    if (radio.classList.contains('up')) drawScope();
    if (radio.classList.contains('up')) {
      let real = null;
      if (analyser) {
        analyser.getFloatFrequencyData(fbins);
        const hz = actx.sampleRate / 2 / fbins.length;
        real = edges.slice(0, BANDS).map((f, k) => {
          const a = Math.max(0, Math.floor(f / hz)), b = Math.max(a + 1, Math.ceil(edges[k + 1] / hz));
          let m = -200;
          for (let i = a; i < b && i < fbins.length; i++) m = Math.max(m, fbins[i]);
          return Math.min(1, Math.max(0, (m + 85) / 60)); // -85 dB silent, -25 dB full
        });
      }
      const e = wordEnergy * Math.exp(-(performance.now() - wordAt) / 160);
      bars.forEach((bar, k) => {
        const synth = e * voiceShape[k] * (0.55 + 0.45 * Math.random());
        const v = Math.max(real ? real[k] : 0, synth, 0.06 + 0.04 * Math.random()); // a little radio hiss
        bar.style.height = `${Math.round(8 + v * 92)}%`;
      });
    }
    requestAnimationFrame(drawSpectrum);
  };
  requestAnimationFrame(drawSpectrum);
  const say = (text, pitch, rate) => new Promise((done) => {
    if (!('speechSynthesis' in window)) { setTimeout(done, 400 + text.length * 45); return; }
    const u = new SpeechSynthesisUtterance(text); u.pitch = pitch; u.rate = rate;
    const voices = speechSynthesis.getVoices().filter((v) => v.lang.startsWith('en'));
    if (voices.length) u.voice = voices[pitch < 1 ? Math.min(1, voices.length - 1) : 0];
    let over = false; const end = () => { if (!over) { over = true; done(); } };
    u.onend = end; u.onerror = end; setTimeout(end, 1500 + text.length * 90);
    // Each word the voice starts: a burst on the handset's spectrum, louder for longer words.
    u.onboundary = (ev) => { wordEnergy = Math.min(1, 0.55 + (ev.charLength || 4) * 0.06); wordAt = performance.now(); };
    u.onstart = () => { wordEnergy = 0.8; wordAt = performance.now(); };
    speechSynthesis.speak(u);
  });
  const line = (who, name, text) => {
    const d = document.createElement('div'); d.className = who; d.innerHTML = `<b>${name.toUpperCase()}</b>${text}`;
    const log = $('radioLog'); log.appendChild(d); while (log.children.length > 3) log.firstChild.remove();
  };
  // The handset's header: who we are talking to (or who is calling), and what the set is doing.
  const talkingTo = (dir, who) => { $('rDir').textContent = dir; $('rWho').textContent = who; };
  const radioState = (s) => { $('rState').textContent = s; };

  // The talkie button: a click as the button is pressed and another as it is let go.
  const keyClick = new Audio('./sfx/radio-key.mp3'); keyClick.preload = 'auto';
  const click = () => playAnalysed(keyClick, 0.8);
  const clickPlain = () => new Promise((done) => {
    const a = keyClick.cloneNode(); a.volume = 0.8;
    a.onended = done; a.onerror = done; setTimeout(done, 450);
    a.play().catch(done);
  });
  // One net, one speaker at a time: the commander's orders and the units' own calls queue up; urgent calls go first.
  let onAir = false;
  const net = [];
  const voiceOf = (name) => { let h = 0; for (const c of name) h = (h * 31 + c.charCodeAt(0)) >>> 0; return { pitch: 0.6 + (h % 50) / 100, rate: 1.0 + (h % 7) / 40 }; };
  const keyUp = async () => { radio.classList.add('up'); await new Promise((r) => setTimeout(r, 450)); };
  const transmit = async (cls, who, name, text, voice) => {
    await click(); radio.classList.add(cls); radioState(cls === 'tx' ? 'TRANSMITTING' : 'RECEIVING'); line(who, name, text);
    traffic.unshift({ when: $('pdaClock').textContent.slice(0, 5), name: cls === 'tx' ? ident.cmdr : name, text, me: cls === 'tx' });
    traffic.length = Math.min(traffic.length, 20);
    await say(text.replace(/(\d)/g, '$1 '), voice.pitch, voice.rate);
    await click(); radio.classList.remove(cls); radioState('ONLINE');
  };
  const pump = async () => {
    if (onAir || !net.length) return;
    onAir = true;
    const job = net.shift();
    try { await job(); } finally {
      onAir = false;
      if (net.length) pump(); else setTimeout(() => { if (!onAir && !net.length) radio.classList.remove('up'); }, 1500);
    }
  };
  // Calls from the units (captains' reports): the commander listens without stopping.
  const hear = (ev) => {
    const job = async () => {
      if (!radio.classList.contains('up')) { $('radioLog').innerHTML = ''; await keyUp(); }
      talkingTo('INCOMING FROM', ev.from);
      await transmit('rx', 'unit', ev.from, ev.text, voiceOf(ev.from));
    };
    if (ev.urgent) net.unshift(job); else net.push(job);
    pump();
  };
  setInterval(() => { try { for (const ev of JSON.parse(v.radio_events())) hear(ev); } catch (e) { /* viewer busy */ } }, 400);
  // The roster keeps up with the units' status while it is open (not while a picker is in use).
  const radioOrder = (id, label, x, y, grid) => {
    const who = v.selection_name(), first = who.split(',')[0];
    const [verb, ing] = VERB[id] || VERB[0];
    if (onAir) flash('net busy: your order goes out next');
    net.push(async () => {
      $('radioLog').innerHTML = '';
      talkingTo('TALKING TO', who);
      v.set_radio(true); await keyUp();
      const order = `${who}, ${verb} grid ${grid}. Over.`;
      await transmit('tx', 'cmd', 'Commander', order, { pitch: 1.0, rate: 1.05 });
      await new Promise((r) => setTimeout(r, 500 + Math.random() * 700)); // the unit keys up
      const reply = `${first} copies, ${ing} grid ${grid}. Out.`;
      await transmit('rx', 'unit', first, reply, voiceOf(first));
      const msg = v.issue_at(id, x, y, performance.now()); if (msg) flash(msg);
      if (page === 'roster') drawRoster();
      await new Promise((r) => setTimeout(r, 400));
      v.set_radio(false);
    });
    pump();
  };
  mapCanvas.addEventListener('wheel', (e) => { e.preventDefault(); zoomGlitch(); onMap(() => v.wheel(Math.sign(e.deltaY))); }, { passive: false });

  const drawRoster = () => {
    const units = JSON.parse(v.roster());
    const captains = units.filter((u) => u.captain);
    const boss = (u) => u.captain ? '<span class="rank">Captain</span>' :
      `<select class="boss" data-id="${u.id}"><option value="-1"${u.boss < 0 ? ' selected' : ''}>You</option>${
        captains.map((c) => `<option value="${c.id}"${u.boss === c.id ? ' selected' : ''}>${c.name}</option>`).join('')}</select>`;
    const cap = (t) => t.charAt(0).toUpperCase() + t.slice(1);
    const row = (u) => `<tr class="unit${u.selected ? ' sel' : ''}${u.captain ? ' capt' : ''}" data-id="${u.id}"><td>${u.name}</td><td>${boss(u)}</td><td class="num">${u.soldiers}</td><td class="st-${u.status}">${u.task || cap(u.status)}</td><td>${cap(u.stance)}</td></tr>`;
    $('roster').innerHTML = `<table><tr><th>Unit</th><th>Answers to</th><th>Manpower</th><th>Status</th><th>Stance</th></tr>${
      captains.map(row).join('')}${units.filter((u) => !u.captain).map(row).join('')
    }</table><p class="note">Click a unit to find it on the map. Attach squads to a captain and give him a goal: he plans the rest and leads them.</p>`;
    for (const r of $('roster').querySelectorAll('tr.unit')) r.addEventListener('click', (e) => {
      if (e.target.tagName === 'SELECT' || e.target.tagName === 'OPTION') return;
      onMap(() => v.select_unit(Number(r.dataset.id))); setPage('map');
    });
    for (const sel of $('roster').querySelectorAll('select.boss')) sel.addEventListener('change', () => {
      flash(v.attach(Number(sel.dataset.id), Number(sel.value))); drawRoster();
    });
  };
  for (const b of tablet.querySelectorAll('[data-page]')) b.addEventListener('click', () => setPage(b.dataset.page));
  // Settings: music on/off and volume, kept between visits.
  const musicOn = $('musicOn'), musicVol = $('musicVol');
  const showMusic = () => {
    musicOn.classList.toggle('on', music.on); musicOn.textContent = music.on ? 'ON' : 'OFF';
    musicVol.value = Math.round(music.vol * 100); $('musicVolTxt').textContent = `${musicVol.value}%`;
  };
  for (const [id, key] of [['setCmdr', 'cmdr'], ['setBatt', 'batt']]) {
    const el = $(id);
    el.value = ident[key];
    el.addEventListener('input', () => { ident[key] = el.value.trim() || (key === 'cmdr' ? 'Commander' : '1st Battalion'); store.set(`${key}.name`, el.value.trim()); showIdent(); });
  }
  musicOn.addEventListener('click', () => { music.set(!music.on, music.vol); showMusic(); });
  musicVol.addEventListener('input', () => { music.set(music.on, musicVol.value / 100); showMusic(); });
  showMusic();
  // Pages keep up with the units while open: redrawn only when something changed, and never while a picker is open
  // (a redraw would close it under the mouse).
  let rosterSeen = '';
  setInterval(() => {
    if (!tabletUp) return;
    showClock(); showSoldiers(soldierCount());
    if (page === 'home') drawHome();
    if (page !== 'roster') return;
    const picking = document.activeElement && document.activeElement.tagName === 'SELECT';
    const now = v.roster();
    if (!picking && now !== rosterSeen) { rosterSeen = now; drawRoster(); }
  }, 1000);

  // Keyboard: held keys pan and rotate every frame, others act once.
  const held = new Set();
  addEventListener('keydown', (e) => {
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLSelectElement) return;
    const k = e.key.toLowerCase();
    held.add(k);
    if (k === 'tab') { e.preventDefault(); setTablet(!tabletUp); return; }
    if (k === '1') setScene('ships');
    else if (k === '2') setScene('ground');
    else if (k === '3') setScene('planet');
    else if (k === '4') setScene('system');
    else if (k === ' ') { e.preventDefault(); v.launch(performance.now()); }
    else if (k === 'c' && tabletUp && page === 'map') { v.map_center_commander(); pdaTap(); }
    else if (k === 'f') v.follow();
    else if (k === 'o') { outline = !outline; v.set_outline(outline); }
    else if (k === 'r') { fullres = !fullres; v.set_full(fullres); }
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
      if (tabletUp && page === 'map') {
        if (px || py) onMap(() => v.pan(px, py));
        v.move_commander(0, 0, false);
      } else if (tabletUp || scene !== 'ground') {
        if ((px || py) && !tabletUp) v.pan(px, py);
        v.move_commander(0, 0, false);
      } else {
        // On foot: WASD walks the commander (Shift sprints); the camera follows him.
        const mx = (held.has('d') || held.has('arrowright') ? 1 : 0) - (held.has('a') || held.has('arrowleft') ? 1 : 0);
        const my = (held.has('w') || held.has('arrowup') ? 1 : 0) - (held.has('s') || held.has('arrowdown') ? 1 : 0);
        v.move_commander(mx, my, held.has('shift'));
      }
      if (held.has('q')) v.rotate(-1.5 * dt);
      if (held.has('e')) v.rotate(1.5 * dt);
      const t0 = performance.now();
      const line = v.frame(now);
      syncTablet();
      footsteps();
      cpuMs = cpuMs * 0.9 + (performance.now() - t0) * 0.1;
      const showNote = now < noteUntil;
      hud.hidden = !showNote || tabletUp;
      if (showNote) hud.textContent = note;
      perfPanel(v.perf(), cpuMs, now, cap);
      const now_scene = v.scene_name();
      if (now_scene !== scene) { scene = now_scene; store.set('scalia.scene', scene); }
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
// This build's update number (stamped by the deploy).
const UPDATE = '__UPDATE__';
// The prototype notice: shown before play; entering also unlocks sound (browsers need a first click).
(function prototypeNotice() {
  const box = document.getElementById('proto');
  if (!box) return;
  document.getElementById('protoVer').textContent = UPDATE.startsWith('__') ? 'Local build' : UPDATE;
  const go = () => { box.hidden = true; removeEventListener('keydown', key, true); music.start(); };
  // While it is up, keys go nowhere else; Enter, Space or Escape enters.
  const key = (e) => { e.stopPropagation(); e.preventDefault(); if (e.key === 'Enter' || e.key === ' ' || e.key === 'Escape') go(); };
  document.getElementById('protoGo').addEventListener('click', go);
  addEventListener('keydown', key, true);
})();
(function watchForUpdates() {
  const mine = new URL(import.meta.url).searchParams.get('v');
  if (!mine) return; // local build
  const bar = document.getElementById('update');
  document.getElementById('update-reload').addEventListener('click', () => location.reload());
  let dismissed = null;
  document.getElementById('update-close').addEventListener('click', () => { bar.hidden = true; dismissed = bar.dataset.v; });
  const check = async () => {
    try {
      const r = await fetch('version.json', { cache: 'no-store' });
      const info = await r.json();
      const live = info.v;
      if (live && live !== mine && live !== dismissed) {
        bar.dataset.v = live;
        document.querySelector('#update .ttl').textContent = `${info.n || 'Update'} received`;
        document.getElementById('update-sub').textContent = `You run ${UPDATE.startsWith('__') ? mine.slice(0, 7) : UPDATE}`;
        if (bar.hidden) { bar.hidden = false; bar.style.animation = 'none'; void bar.offsetWidth; bar.style.animation = ''; }
      }
    } catch (_) { /* offline: try again later */ }
  };
  setInterval(check, 60_000);
  check();
})();
