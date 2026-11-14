// Full-screen game page. The Rust viewer draws everything; this file forwards input and the animation frame.
import init, * as wasm from './pkg/scalia_wasm.js';
import { AudioEngine } from './audio.js';

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

// World sounds (music, footsteps) go through one bus; with the tablet up it is muffled (low-passed and a little
// quieter), as if the commander's attention has gone to the device. The tablet's own sounds and the radio stay clear.
const worldAudio = (() => {
  let ctx = null, filt = null, gain = null;
  const routed = new WeakSet();
  const graph = () => {
    if (ctx) return ctx;
    try {
      ctx = new AudioContext();
      filt = ctx.createBiquadFilter(); filt.type = 'lowpass'; filt.frequency.value = 20000; filt.Q.value = 0.6;
      gain = ctx.createGain(); filt.connect(gain).connect(ctx.destination);
    } catch (_) { ctx = null; }
    return ctx;
  };
  return {
    // Sends an audio element through the bus (once per element).
    add(el) {
      const c = graph(); if (!c) return;
      if (c.state === 'suspended') c.resume().catch(() => {});
      if (routed.has(el)) return;
      try { c.createMediaElementSource(el).connect(filt); routed.add(el); } catch (_) { /* plays unfiltered */ }
    },
    fadeOut(sec) { const c = graph(); if (!c) return; gain.gain.cancelScheduledValues(c.currentTime); gain.gain.setTargetAtTime(0, c.currentTime, sec / 3); },
    muffle(on) {
      const c = graph(); if (!c) return;
      const t = c.currentTime;
      filt.frequency.cancelScheduledValues(t); gain.gain.cancelScheduledValues(t);
      filt.frequency.setTargetAtTime(on ? 600 : 20000, t, 0.12);
      gain.gain.setTargetAtTime(on ? 0.55 : 1, t, 0.12);
    },
  };
})();

// Ambience: wind through the trees, looping under everything (a world sound, so it is muffled with the tablet up).
const ambience = () => {
  const el = new Audio('./sfx/ambience-wind.mp3');
  el.loop = true; el.volume = 0.35;
  worldAudio.add(el);
  el.play().catch(() => {});
};

// Reloading (F5, Ctrl+R, Ctrl+Shift+R, or the update notice's button) is never instant: a red "Rebooting
// session" window drops in, then the game fades out (picture and sound) and only then the page reloads.
// The browser's own reload button cannot be intercepted by a page.
let rebooting = false;
const reboot = () => {
  if (rebooting) return;
  rebooting = true;
  const box = document.getElementById('reboot');
  box.hidden = false;
  setTimeout(() => { document.body.classList.add('unloading'); worldAudio.fadeOut(0.6); }, 1000);
  setTimeout(() => location.reload(), 1700);
};
addEventListener('keydown', (e) => {
  if (e.key === 'F5' || ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'r')) { e.preventDefault(); e.stopPropagation(); reboot(); }
}, true);

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
      if (m.on && m.started && m.vol > 0) worldAudio.add(el);
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
  };
  // The game always opens on the ground: the model viewer and the space views are tools, not where play starts.
  setScene('ground');

  // Toggles and actions.
  // Full resolution for now (the chunky pixel look is switched off; R brings it back).
  let fullres = true, outline = false;
  v.set_full(true);

  // Mouse: wheel zooms, drag pans.
  // One mouse-wheel notch (about 100 px) is one zoom step; trackpads send many small deltas, summed into steps.
  // Turns wheel events into whole zoom steps: step(sign) is called once per notch (~100 px of scroll).
  const notches = (step) => {
    let sum = 0, at = 0;
    return (e) => {
      e.preventDefault();
      if (e.timeStamp - at > 300) sum = 0; // a pause forgets leftovers
      at = e.timeStamp;
      const px = e.deltaMode === 1 ? e.deltaY * 33.4 : e.deltaMode === 2 ? e.deltaY * 800 : e.deltaY;
      // A mouse wheel notch (a big delta, however big) is exactly one step; only small trackpad deltas add up.
      if (Math.abs(px) >= 40) { sum = 0; step(Math.sign(px)); return; }
      // The first movement of a gesture steps at once, so a single notch never feels dead.
      const fresh = sum === 0, STEP = 100;
      sum += px;
      if (fresh && Math.abs(sum) < STEP) { step(Math.sign(sum)); sum = Math.sign(sum) * -STEP + sum; }
      while (Math.abs(sum) >= STEP) { step(Math.sign(sum)); sum -= Math.sign(sum) * STEP; }
    };
  };
  canvas.addEventListener('wheel', notches((d) => v.wheel(d)), { passive: false });
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
  // Zooming the map: a very fast blur while the wheel moves, hiding the detail popping in. No glitch effects.
  let zoomBlurTimer = 0;
  const zoomGlitch = () => {
    const cv = $('mapCanvas');
    cv.classList.add('zoomblur');
    clearTimeout(zoomBlurTimer);
    zoomBlurTimer = setTimeout(() => cv.classList.remove('zoomblur'), 170);
  };
  // The changelog is written at deploy time from the commit history (changelog.json).
  let chgLoaded = false;
  if (store.get('scalia.controls') === '0') $('controls').hidden = true;
  const drawChangelog = async () => {
    if (chgLoaded) return;
    try {
      const list = await (await fetch('./changelog.json', { cache: 'no-cache' })).json();
      $('chgList').innerHTML = list.map((e) => `<div class="ver"><b>${e.n}</b><span>${e.text}${e.body ? `<span class="body">${e.body.replace(/\n/g, '<br>')}</span>` : ''}<small>${e.date}</small></span></div>`).join('') || '<p class="note">Nothing recorded yet.</p>';
      chgLoaded = true;
    } catch { $('chgList').innerHTML = '<p class="note">The changelog is not available right now.</p>'; }
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
  const unitSelect = sfx('./sfx/unit-select.mp3', 0.6), menuOpen = sfx('./sfx/context-menu.mp3', 0.6);
  const pdaOpen = sfx('./sfx/pda-open.mp3', 0.7), pdaClose = sfx('./sfx/pda-leave.mp3', 0.3), pdaTap = sfx('./sfx/pda-tap.mp3', 0.6);
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
    worldAudio.muffle(false);
    tablet.classList.remove('up');
    setTimeout(() => { if (!tabletShown) { dockRadio(false); walkieUp(); } }, 400); // the handset goes down with the hand first
    closeActions();
    setTimeout(() => { if (!tabletUp) v.set_tablet(false); }, 380); // slide down first, then zoom out
  };
  // ---- Hands on the tablet ----
  // The walkie-talkie: while the tablet is up, the handset lives in the left hand beside it (scaled from its
  // field size to the hand's), and the left hand leaves the tablet to raise it whenever the radio is up.
  const radioEl = $('radio');
  const wkSlot = tablet.querySelector('.wkslot'), radioHome = radioEl.parentNode, radioNext = radioEl.nextSibling;
  const fitRadio = () => { if (radioEl.parentNode === wkSlot) radioEl.style.transform = `scale(${wkSlot.clientHeight / radioEl.offsetHeight})`; };
  const dockRadio = (inHand) => {
    if (inHand && radioEl.parentNode !== wkSlot) { wkSlot.appendChild(radioEl); fitRadio(); }
    if (!inHand && radioEl.parentNode === wkSlot) { radioEl.style.transform = ''; radioHome.insertBefore(radioEl, radioNext); }
  };
  addEventListener('resize', fitRadio);
  // On a narrow screen the tablet eases back (around its right edge) to make room for the handset.
  let frameS = 1, frameGoal = 1, frameRaf = 0;
  const applyFrame = () => {
    tablet.style.scale = frameS === 1 ? '' : frameS.toFixed(4);
  };
  const stepFrame = (t0, from) => (t) => {
    const k = Math.min(1, (t - t0) / 400), e = k * k * k * (k * (k * 6 - 15) + 10);
    frameS = from + (frameGoal - from) * e; applyFrame();
    frameRaf = k < 1 ? requestAnimationFrame(stepFrame(t0, from)) : 0;
  };
  // Equip and put-away sounds for the handset, whichever view it comes up in.
  const equipWalkie = sfx('./sfx/walkie-up.mp3', 0.6), stowWalkie = sfx('./sfx/walkie-down.mp3', 0.6);
  let radioWasUp = false;
  const walkieUp = () => {
    const isUp = radioEl.classList.contains('up');
    if (isUp !== radioWasUp) { radioWasUp = isUp; (isUp ? equipWalkie : stowWalkie)(); }
    const on = isUp && tablet.classList.contains('up');
    tablet.classList.toggle('walkie', on);
    // Room for the handset left of the tablet (it reaches 24.5% of the tablet's width past its left edge).
    const w = tablet.offsetWidth, right = (innerWidth + w) / 2, need = w * 1.245 + innerWidth * 0.015;
    frameGoal = on && need > right ? right / need : 1;
    if (frameGoal === frameS) return;
    cancelAnimationFrame(frameRaf);
    const t0 = performance.now(), from = frameS;
    frameRaf = requestAnimationFrame(stepFrame(t0, from));
  };
  addEventListener('resize', walkieUp);
  new MutationObserver(walkieUp).observe(radioEl, { attributes: true, attributeFilter: ['class'] });
  // The right thumb rests on the frame below the buttons, follows the one under the mouse and presses it.
  const thumb = tablet.querySelector('.thumb'), armR = tablet.querySelector('.arm-r');
  // The thumb rests just right of a button; the whole right hand slides along the edge to the one under the mouse.
  const ASPECT = 2164 / 1139, TIP_Y = 54.036;
  const handTo = (y) => {
    const dy = `0 ${((y - TIP_Y) / ASPECT).toFixed(2)}cqw`; // percent of the tablet's height, in cqw
    thumb.style.translate = dy; armR.style.translate = dy;
  };
  const btnY = (b) => parseFloat(b.style.top) + 6.15 / 2;
  handTo(btnY(tablet.querySelector('.hw[data-hw="map"]')));
  for (const b of tablet.querySelectorAll('.hw')) {
    if (parseFloat(b.style.left || '91.99') < 50) continue; // the right-hand buttons only
    b.addEventListener('pointerenter', () => handTo(btnY(b)));
    b.addEventListener('pointerdown', () => {
      handTo(btnY(b)); thumb.classList.add('press');
      tablet.style.rotate = '0.2deg'; setTimeout(() => { tablet.style.rotate = ''; }, 120);
    });
  }
  // The left hand: the same, mirrored, for the buttons on the left side (Field Manual, Changelog).
  const thumbL = tablet.querySelector('.thumb-l'), armL = tablet.querySelector('img.fp.arm-l');
  const leftTo = (y) => { const dy = `${((y - TIP_Y) / ASPECT).toFixed(2)}cqw`; thumbL.style.setProperty('--ly', dy); armL.style.setProperty('--ly', dy); };
  leftTo(btnY(tablet.querySelector('.hw[data-hw="manual"]')));
  for (const b of tablet.querySelectorAll('.hw')) {
    if (parseFloat(b.style.left || '91.99') >= 50) continue;
    b.addEventListener('pointerenter', () => leftTo(btnY(b)));
    b.addEventListener('pointerdown', () => {
      leftTo(btnY(b)); thumbL.classList.add('press');
      tablet.style.rotate = '-0.2deg'; setTimeout(() => { tablet.style.rotate = ''; }, 120);
    });
  }
  addEventListener('pointerup', () => thumb.classList.remove('press'));
  addEventListener('pointerup', () => thumbL.classList.remove('press'));
  // Putting the tablet away with Tab: the thumb goes to the power button and presses it first, then it goes down.
  let pressingOff = false;
  const pressOff = () => {
    if (pressingOff) return;
    pressingOff = true;
    handTo(btnY(tablet.querySelector('.hw[data-hw="close"]')));
    setTimeout(() => { thumb.classList.add('press'); pdaTap(); }, 330);
    setTimeout(() => { thumb.classList.remove('press'); }, 450);
    setTimeout(() => { pressingOff = false; setTablet(false); }, 520);
  };
  // The commander's footsteps on grass: one step each time a heel strikes (gait phase 0 and 0.5), a little
  // different every time.
  const stepSound = new Audio('./sfx/steps-grass.mp3'); stepSound.preload = 'auto';
  let lastPhase = -1;
  const footsteps = () => {
    const ph = v.scene_name() === 'ground' ? v.commander_phase() : -1;
    if (ph >= 0 && lastPhase >= 0) {
      const crossed = (at) => (lastPhase < at && ph >= at) || (lastPhase > ph && (lastPhase < at || ph >= at));
      if (crossed(0.0) || crossed(0.5)) {
        const a = stepSound.cloneNode(); worldAudio.add(a);
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
      worldAudio.muffle(true);
      tablet.classList.add('up');
      dockRadio(true); walkieUp();
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
  let mDrag = null, mMoved = 0, mBox = null, hoverId = null;
  const mapBoxEl = $('mapBox');
  mapCanvas.addEventListener('contextmenu', (e) => e.preventDefault());
  // Right button: a short click opens the commands menu; held, it opens the about panel for what is under it.
  let rHold = null;
  const HOLD_MS = 450;
  mapCanvas.addEventListener('pointerdown', (e) => {
    mapCanvas.setPointerCapture(e.pointerId);
    if (e.button === 2) {
      closeActions();
      const at = { offsetX: e.offsetX, offsetY: e.offsetY };
      holdRing(e.offsetX, e.offsetY, true);
      rHold = { at, timer: setTimeout(() => { rHold = null; holdRing(0, 0, false); openAbout(at); }, HOLD_MS) };
      return;
    }
    if (e.button === 1) { e.preventDefault(); mDrag = [e.clientX, e.clientY]; return; }   // middle button: move the camera
    if (e.button !== 0) return;
    closeActions(); closeAbout();
    mBox = [e.offsetX, e.offsetY]; mMoved = 0;                                            // left: select, or drag a square
  });
  mapCanvas.addEventListener('pointermove', (e) => {
    if (mDrag) {
      const dx = e.clientX - mDrag[0], dy = e.clientY - mDrag[1];
      onMap(() => v.pan(dx * dpr(), dy * dpr()));
      mDrag = [e.clientX, e.clientY];
      return;
    }
    if (mBox) {
      mMoved = Math.max(mMoved, Math.abs(e.offsetX - mBox[0]) + Math.abs(e.offsetY - mBox[1]));
      if (mMoved > 6) {
        Object.assign(mapBoxEl.style, { left: Math.min(mBox[0], e.offsetX) + 'px', top: Math.min(mBox[1], e.offsetY) + 'px',
          width: Math.abs(e.offsetX - mBox[0]) + 'px', height: Math.abs(e.offsetY - mBox[1]) + 'px' });
        mapBoxEl.hidden = false;
      }
      return;
    }
    // Hover: the details panel shows the unit under the mouse.
    const h = mapCanvas.clientHeight, w = mapCanvas.clientWidth;
    let hit = null, best = 1e9;
    for (const m of lastMarks) {
      const d = Math.hypot(m.x * w - e.offsetX, m.y * h - e.offsetY);
      if (d <= Math.max(14, m.r * h) && d < best) { best = d; hit = m.id; }
    }
    hoverId = hit;
    showTip(e);
  });
  mapCanvas.addEventListener('pointerleave', () => { hoverId = null; mapTip.hidden = true; });
  mapCanvas.addEventListener('pointerup', (e) => {
    if (e.button === 1) { mDrag = null; return; }
    if (e.button === 2) {
      if (rHold) { clearTimeout(rHold.timer); holdRing(0, 0, false); const at = rHold.at; rHold = null; mapMenu({ offsetX: at.offsetX, offsetY: at.offsetY }); }
      return;
    }
    if (e.button !== 0 || !mBox) return;
    mapBoxEl.hidden = true;
    if (mMoved > 6) {
      commanderPicked = false;
      const msg = onMap(() => v.select_box(mBox[0] * dpr(), mBox[1] * dpr(), e.offsetX * dpr(), e.offsetY * dpr(), performance.now())); if (msg) flash(msg);
      if (msg && msg !== 'nothing selected') unitSelect();
    } else mapSelect(e);
    mBox = null;
  });
  // ---- Cursor helper and about panels ----
  // A small helper by the cursor names whatever it is over and says what the mouse does there; holding the right
  // button opens an about panel for it (what it is, what it does, what it does not). Buildings are scanned: their
  // doors, windows and rooms are counted and shown over the building on the map.
  const mapTip = $('mapTip'), about = $('aboutPanel'), scanSvg = $('scanOverlay'), ring = $('holdRing');
  const holdRing = (x, y, on) => {
    ring.hidden = !on;
    if (on) { ring.style.left = `${x}px`; ring.style.top = `${y}px`; ring.classList.remove('go'); void ring.offsetWidth; ring.classList.add('go'); }
  };
  const ABOUT = {
    unit: { type: 'Infantry squad', what: 'Riflemen under one call sign: they move, take cover and report as a team.',
      does: 'Moves, defends, assaults or supports where you order; reports by radio when it gets there.',
      not: 'Acts only on orders (yours or its captain\'s). No vehicles or heavy weapons yet.' },
    captain: { type: 'Captain', what: 'An officer leading his own small team.',
      does: 'Give him a goal (take or hold a grid) and attach squads in the roster: he plans the steps and leads them.',
      not: 'Fights poorly alone; without squads attached he only reports.' },
    commander: { type: 'You · battalion commander', what: 'The commander on foot. Command goes out over the radio; the PDA is how you see the battalion.',
      does: 'Calls units up, gives orders, takes reports.', not: 'Not a fighter: the rifle stays slung. A unit only acts on what you tell it over the radio.' },
    barracks: { type: 'Structure · barracks block', what: 'Single-storey troop quarters, 24 × 8 m, flat concrete roof.',
      does: 'Hard cover and a strongpoint for a squad.', not: 'Units do not go inside yet.', scan: true, rooms: '4 and a corridor',
      built: 'Reinforced concrete blocks, 25 cm walls, concrete roof slab', caliber: '12.7 mm (.50 cal). Not 30 mm cannon, RPGs or tank guns.' },
    killhouse: { type: 'Structure · training house', what: 'Roofless plywood house, 20 × 14 m: four rooms, two outside doors, windows on every side.',
      does: 'Close-quarters training ground. The enemy squad (red helmets and vests) is stationed inside.', not: 'Walls stop sight but not bullets.', scan: true, rooms: '4, joined by inner doorways',
      built: 'Plywood panels on timber posts, 2.4 m high, no roof', caliber: 'None: every round goes through plywood.' },
    tower: { type: 'Structure · guard tower', what: 'Roofed lookout on timber legs, platform 4.5 m up.',
      does: 'Sees over the walls; the sandbag lining protects one sentry.', not: 'The legs give no cover.', scan: true, rooms: '1 platform',
      built: 'Timber frame, sandbag-lined platform, sheet roof', caliber: '7.62 mm at the platform; pistol rounds only at the legs.' },
    wall: { type: 'Concrete wall', what: 'Perimeter wall, 0.6 m thick, 2.7 m high.', does: 'Blocks movement and sight; stops up to 14.5 mm.', not: 'Cannot be breached yet (engineers later).' },
    sandbags: { type: 'Sandbag line', what: 'Two courses of sandbags, about 1 m high.', does: 'Cover for crouching or prone troops; stops rifle fire up to 7.62 mm.', not: 'No protection against explosives or sustained heavy machine-gun fire.' },
    container: { type: 'Shipping container', what: 'A 12 m steel container in the depot.', does: 'Blocks movement and sight: concealment more than cover.', not: 'Thin steel: stops pistol rounds and fragments only.' },
    crate: { type: 'Supply crate', what: 'A wooden crate by the depot.', does: 'Low concealment.', not: 'Stops fragments only. Supplies are not simulated yet.' },
    rock: { type: 'Rock', what: 'A boulder in the obstacle lane.', does: 'Natural hard cover against small arms.', not: 'Low: cover only when prone.' },
    ground: { type: 'Ground', what: 'Open ground.', does: 'Units can move here.', not: 'No cover.' },
    tank: { type: 'Armored vehicle · main battle tank', what: 'Tracked, armoured, a 120 mm gun.', does: 'Breaks strongpoints, leads assaults.', not: 'Not in the battalion yet.' },
    truck: { type: 'Vehicle · utility truck', what: 'Carries troops and supplies.', does: 'Moves a squad fast along roads.', not: 'Unarmoured. Not in the battalion yet.' },
  };
  const nameOf = (o) => o.kind === 'unit' ? `${o.cs} ${pad2(o.n)}` : o.kind === 'commander' ? 'Actual 00' :
    o.kind === 'ground' ? (o.inside ? 'Testing ground' : 'Open ground') : ABOUT[o.kind]?.type.split(' · ').pop().replace(/^./, (c) => c.toUpperCase()) || o.kind;
  const infoOf = (o) => o.kind === 'unit' && o.captain ? ABOUT.captain : ABOUT[o.kind] || ABOUT.ground;
  const objectAt = (x, y) => { try { return JSON.parse(onMap(() => v.object_at(x * dpr(), y * dpr()))); } catch (_) { return { kind: 'none' }; } };
  // The helper hangs off the cursor on a soft spring: it trails the mouse and settles, never jumps.
  let tipAt = 0, tipObj = null, tipGoal = null, tipPos = null, mouseAt = { offsetX: 0, offsetY: 0 };
  const tipLoop = () => {
    if (tipGoal && !mapTip.hidden) {
      tipPos = tipPos || { ...tipGoal };
      tipPos.x += (tipGoal.x - tipPos.x) * 0.22; tipPos.y += (tipGoal.y - tipPos.y) * 0.22;
      mapTip.style.transform = `translate(${tipPos.x.toFixed(1)}px, ${tipPos.y.toFixed(1)}px) translate(-100%, -100%)`;
    }
    requestAnimationFrame(tipLoop);
  };
  requestAnimationFrame(tipLoop);
  const showTip = (e) => {
    mouseAt = { offsetX: e.offsetX, offsetY: e.offsetY };
    tipGoal = { x: e.offsetX - 14, y: e.offsetY - 14 };
    if (performance.now() - tipAt < 60) return; tipAt = performance.now();
    const o = objectAt(e.offsetX, e.offsetY);
    tipObj = o;
    if (o.kind === 'none') { mapTip.hidden = true; tipPos = null; return; }
    const sel = onMap(() => v.selection_name());
    const hints = [];
    if (o.kind === 'unit') hints.push(`Click: select ${o.captain ? 'captain' : 'squad'}`);
    else if (o.kind === 'commander') hints.push('Click: select yourself');
    else if (o.kind === 'tank' || o.kind === 'truck') hints.push('Click: select armored vehicle');
    else if (!sel) hints.push('Click a unit to select it');
    if (sel) hints.push(`Right-click: order ${sel.split(',')[0]}${sel.includes(',') ? ' +' : ''} here`);
    if (o.kind === 'killhouse') hints.push('K: run the training assault');
    hints.push(infoOf(o).scan ? 'Hold S: scan structure' : 'Hold right-click: about');
    const sub = o.kind === 'unit' ? `${infoOf(o).type} · ${o.soldiers}` : infoOf(o).type;
    const html = `<b>${nameOf(o)}</b><span>${sub} · grid ${o.grid}</span>${hints.map((h) => `<i>${h}</i>`).join('')}`;
    if (mapTip.innerHTML !== html) mapTip.innerHTML = html;
    if (mapTip.hidden) { mapTip.hidden = false; tipPos = null; }
  };
  // Hold S over a building: scan it (S does not pan while it is held for a scan).
  let sHold = null;
  const scanHolding = () => !!sHold;
  addEventListener('keydown', (e) => {
    if (e.key.toLowerCase() !== 's' || e.repeat || !(tabletUp && page === 'map') || !tipObj || !infoOf(tipObj).scan || mapTip.hidden) return;
    const at = { ...mouseAt };
    holdRing(at.offsetX, at.offsetY, true);
    sHold = setTimeout(() => { sHold = null; holdRing(0, 0, false); openAbout(at); }, HOLD_MS);
  });
  addEventListener('keyup', (e) => {
    if (e.key.toLowerCase() === 's' && sHold) { clearTimeout(sHold); sHold = null; holdRing(0, 0, false); }
  });
  let scanId = null, scanStart = 0;
  const closeAbout = () => { about.hidden = true; scanId = null; scanSvg.innerHTML = ''; scanSvg.hidden = true; };
  const openAbout = (at) => {
    const o = objectAt(at.offsetX, at.offsetY);
    if (o.kind === 'none') return;
    const i = infoOf(o);
    menuOpen();
    let html = `<button class="x" title="Close (Esc)">×</button><h4>${nameOf(o)}</h4><div class="role">${i.type}</div>
      <dl><dt>What it is</dt><dd>${i.what}</dd><dt>What it does</dt><dd>${i.does}</dd><dt>What it does not</dt><dd>${i.not}</dd></dl>`;
    if (i.scan) {
      const ov = JSON.parse(onMap(() => v.structure_overlay(o.id)));
      html += `<div class="scaninfo"><div class="scanning"><span>Scanning structure</span><i class="bar"><i></i></i></div>
        <dl class="result"><dt>Doors</dt><dd>${ov.doors.length}</dd><dt>Windows</dt><dd>${ov.windows.length}</dd><dt>Rooms</dt><dd>${i.rooms}</dd>
        <dt>Built from</dt><dd>${i.built}</dd><dt>Stops up to</dt><dd>${i.caliber}</dd></dl></div>`;
      scanId = o.id; scanStart = performance.now(); scanSvg.hidden = false;
    } else { scanId = null; scanSvg.hidden = true; scanSvg.innerHTML = ''; }
    about.innerHTML = html; about.hidden = false;
    about.querySelector('.x').addEventListener('click', closeAbout);
  };
  addEventListener('keydown', (e) => { if (e.key === 'Escape' && !about.hidden) closeAbout(); });
  const SCAN_MS = 1600;
  // The scan overlay, redrawn every frame while the panel is open (the map can move under it).
  const drawScan = () => {
    if (scanId === null || about.hidden) return;
    const ov = JSON.parse(onMap(() => v.structure_overlay(scanId)));
    const pts = (q) => q.map((n, k) => n.toFixed(4)).join(' ');
    const p = Math.min(1, (performance.now() - scanStart) / SCAN_MS);
    const all = ov.faces.flat(), ys = all.filter((_, k) => k % 2), y0 = Math.min(...ys), y1 = Math.max(...ys);
    const reveal = y1 - (y1 - y0) * p; // the scan sweeps up the building
    scanSvg.innerHTML = `<g style="mask-image:none">${ov.faces.map((q) => `<polygon class="face" points="${pts(q)}"/>`).join('')}
      ${p >= 1 ? ov.rooms.map((q) => `<polyline class="room" points="${pts(q)}"/>`).join('') : ''}
      ${p >= 1 ? ov.windows.map((q) => `<polygon class="win" points="${pts(q)}"/>`).join('') : ''}
      ${p >= 1 ? ov.doors.map((q) => `<polygon class="door" points="${pts(q)}"/>`).join('') : ''}</g>
      ${p < 1 ? `<line class="sweep" x1="0" x2="1" y1="${reveal.toFixed(4)}" y2="${reveal.toFixed(4)}"/>` : ''}`;
    scanSvg.style.maskImage = p < 1 ? `linear-gradient(to bottom, transparent ${(reveal * 100).toFixed(2)}%, #000 ${(reveal * 100).toFixed(2)}%)` : '';
    about.classList.toggle('scanned', p >= 1);
  };
  // Orders: click a unit, then click a place. An actions window asks what the unit should do there; its AI says
  // whether it can. The order then goes out over the radio, and the unit moves once it has answered.
  const actionsBox = $('actions');
  const closeActions = () => { actionsBox.hidden = true; };
  // ---- Unit recognition ----
  // Field view: call sign and number over each unit, nothing else. PDA map: an open-corner square around each unit,
  // yellow, green when selected, with a details panel on the left of the screen.
  const fieldEls = new Map();
  const drawFieldLabels = () => {
    const box = $('fieldLabels');
    const on = !tabletUp && !tabletShown && scene === 'ground';
    box.hidden = !on;
    if (!on) return;
    const w = canvas.clientWidth, h = canvas.clientHeight, seen = new Set();
    for (const m of JSON.parse(v.unit_marks())) {
      let el = fieldEls.get(m.id);
      if (!el) { el = document.createElement('i'); el.textContent = `${m.cs} ${String(m.n).padStart(2, '0')}`; if (m.you) el.className = 'you'; box.appendChild(el); fieldEls.set(m.id, el); }
      const up = m.r * h + 12;
      el.style.transform = `translate(${(m.x * w).toFixed(1)}px, ${(m.y * h - up).toFixed(1)}px) translate(-50%, -100%)`;
      seen.add(m.id);
    }
    for (const [id, el] of fieldEls) if (!seen.has(id)) { el.remove(); fieldEls.delete(id); }
  };
  // ---- AI lab (?lab in the address) ----
  // Fixed scenarios on the testing ground to watch the battle AI together: slow motion and pause, what every
  // man is doing written over his head, and the leaders' steps and shouts in a feed. Keys: - pause, , . speed,
  // Backspace restart.
  const lab = new URLSearchParams(location.search).has('lab') ? (() => {
    const SCEN = [['clear', 'Clear house'], ['contact', 'Contact'], ['wounded', 'Casualty'], ['standoff', 'Standoff']];
    const SPEEDS = [0, 0.25, 0.5, 1, 2];
    let cur = new URLSearchParams(location.search).get('lab') || 'clear', speed = 1, labels = true;
    const panel = document.createElement('div'); panel.id = 'labPanel';
    panel.innerHTML = `<h5>AI LAB</h5><div class="scen">${SCEN.map(([k, n]) => `<button data-s="${k}">${n}</button>`).join('')}</div>
      <div class="spd">${SPEEDS.map((x) => `<button data-v="${x}">${x === 0 ? 'Pause' : x + '×'}</button>`).join('')}<button data-l>Labels</button><button data-r>Restart</button></div>
      <div class="note"></div><div class="feed"></div><div style="color:#8a9680;margin-top:4px">− pause · , . slower/faster · Backspace restart</div>`;
    document.body.appendChild(panel);
    const marks = $('labMarks'), feed = panel.querySelector('.feed'), els = new Map();
    const paint = () => {
      panel.querySelectorAll('[data-s]').forEach((b) => b.classList.toggle('on', b.dataset.s === cur));
      panel.querySelectorAll('[data-v]').forEach((b) => b.classList.toggle('on', Number(b.dataset.v) === speed));
      panel.querySelector('[data-l]').classList.toggle('on', labels);
    };
    const setSpeed = (x) => { speed = x; v.lab_set_speed(x); paint(); };
    const start = (k) => {
      cur = k; feed.textContent = '';
      panel.querySelector('.note').textContent = v.lab_start(k);
      const u = new URL(location.href); u.searchParams.set('lab', k); history.replaceState(null, '', u);
      paint();
    };
    panel.addEventListener('click', (e) => {
      const b = e.target.closest('button'); if (!b) return;
      if (b.dataset.s) start(b.dataset.s);
      else if (b.dataset.v !== undefined) setSpeed(Number(b.dataset.v));
      else if (b.hasAttribute('data-l')) { labels = !labels; paint(); }
      else if (b.hasAttribute('data-r')) start(cur);
    });
    addEventListener('keydown', (e) => {
      if (e.target.closest && e.target.closest('input,textarea')) return;
      const k = e.key.toLowerCase();
      if (k === '-') setSpeed(speed === 0 ? 1 : 0);
      else if (k === ',') setSpeed(SPEEDS[Math.max(1, SPEEDS.indexOf(speed) - 1)]);
      else if (k === '.') setSpeed(SPEEDS[Math.min(SPEEDS.length - 1, SPEEDS.indexOf(speed) + 1)]);
      else if (k === 'backspace') { e.preventDefault(); start(cur); }
    });
    let started = false;
    return {
      draw() {
        if (!started) { started = true; start(cur); setSpeed(1); }
        const f = v.lab_feed();
        if (f) { feed.textContent += (feed.textContent ? '\n' : '') + f; feed.scrollTop = feed.scrollHeight; }
        // The tablet comes first: the lab panel and labels step aside while it is up.
        panel.hidden = tabletUp || tabletShown;
        marks.hidden = !labels || tabletUp || tabletShown;
        if (marks.hidden) return;
        const w = canvas.clientWidth, h = canvas.clientHeight, seen = new Set();
        for (const m of JSON.parse(v.lab_marks())) {
          let el = els.get(m.i);
          if (!el) { el = document.createElement('i'); marks.appendChild(el); els.set(m.i, el); }
          el.className = m.t ? 'red' : '';
          if (el.textContent !== m.txt) el.textContent = m.txt;
          el.style.transform = `translate(${(m.x * w).toFixed(1)}px, ${(m.y * h).toFixed(1)}px) translate(-50%, -100%)`;
          seen.add(m.i);
        }
        for (const [i, el] of els) if (!seen.has(i)) { el.remove(); els.delete(i); }
      },
    };
  })() : null;
  const markEls = new Map();
  let lastMarks = [], commanderPicked = false, panelKey = '';
  const SPEC = { weapon: 'Assault rifle', armor: 'Standard fatigues', gear: 'Radio, first-aid kit' };
  const drawUnitPanel = (units, marks) => {
    const cap = (t) => t.charAt(0).toUpperCase() + t.slice(1);
    const panel = $('unitPanel');
    const sel = marks.find((m) => m.id === hoverId && m.id >= 0);
    let key = '', html = '';
    if (sel) {
      const u = units.find((x) => x.id === sel.id);
      if (u) {
        const boss = u.boss >= 0 ? (units.find((x) => x.id === u.boss) || {}).name : 'Battalion command';
        key = `${u.id}|${u.soldiers}|${u.status}|${u.stance}|${u.task}`;
        html = `<h4>${sel.cs} ${String(sel.n).padStart(2, '0')}</h4><div class="role">${u.captain ? 'Captain' : 'Squad'}</div><dl>
          <dt>Manpower</dt><dd>${u.soldiers}</dd><dt>Status</dt><dd>${cap(u.status)}${u.task ? ' · ' + u.task : ''}</dd><dt>Stance</dt><dd>${cap(u.stance)}</dd>
          <dt>Answers to</dt><dd>${boss}</dd><dt>Weapon</dt><dd>${SPEC.weapon}</dd><dt>Armor</dt><dd>${SPEC.armor}</dd><dt>Equipment</dt><dd>${SPEC.gear}</dd></dl>`;
      }
    } else if (hoverId === -1) {
      key = 'cmd';
      html = `<h4>Actual 00</h4><div class="role">Commander</div><dl><dt>Name</dt><dd>${ident.cmdr}</dd><dt>Manpower</dt><dd>1</dd><dt>Weapon</dt><dd>Rifle, slung</dd>
        <dt>Armor</dt><dd>Black uniform</dd><dt>Equipment</dt><dd>Command PDA, handheld radio</dd></dl>`;
    }
    panel.hidden = !html;
    if (key !== panelKey) { panelKey = key; if (html) panel.innerHTML = html + '<p class="note">Placeholder details until equipment is simulated.</p>'; }
  };
  const MARK_SVG = '<svg viewBox="0 0 100 100" preserveAspectRatio="none"><path d="M10 0 H100 V90"/><path d="M0 10 V100 H90"/></svg>';
  const drawMapMarks = () => {
    if (!(tabletUp && tabletShown && page === 'map')) return;
    const box = $('mapMarks'), w = mapCanvas.clientWidth, h = mapCanvas.clientHeight;
    const marks = onMap(() => JSON.parse(v.unit_marks()));
    lastMarks = marks;
    const seen = new Set(), placed = [];
    const order = [...marks].sort((a, b) => (b.sel || b.you) - (a.sel || a.you));
    // Zoomed out, units that would sit on top of each other merge into one marker labelled with the count.
    const heads = [], under = new Map();
    for (const m of order) {
      if (m.you) { heads.push(m); continue; }
      const hd = heads.find((q) => !q.you && Math.hypot((q.x - m.x) * w, (q.y - m.y) * h) < 22);
      if (hd) under.set(hd.id, (under.get(hd.id) || 1) + 1); else heads.push(m);
    }
    const hidden = new Set(order.filter((m) => !heads.includes(m)).map((m) => m.id));
    for (const m of order) {
      let el = markEls.get(m.id);
      if (!el) {
        el = document.createElement('div'); el.innerHTML = MARK_SVG; el.firstChild.style.position = 'absolute';
        const b = document.createElement('b'); b.textContent = `${m.cs} ${String(m.n).padStart(2, '0')}`;
        el.style.cssText = 'position:absolute;left:0;top:0;will-change:transform'; el.appendChild(b); box.appendChild(el); markEls.set(m.id, el);
      }
      el.style.display = hidden.has(m.id) ? 'none' : '';
      const n = under.get(m.id) || 1;
      el.lastChild.textContent = n > 1 ? `${n} units` : (m.you ? 'Actual 00' : `${m.cs} ${String(m.n).padStart(2, '0')}`);
      const size = Math.max(26, m.r * h * 2);
      const svg = el.firstChild;
      svg.style.width = svg.style.height = `${size}px`; svg.style.left = svg.style.top = `${-size / 2}px`;
      svg.classList.toggle('sel', m.sel || (m.you && commanderPicked));
      el.style.transform = `translate(${(m.x * w).toFixed(1)}px, ${(m.y * h).toFixed(1)}px)`;
      const lx = m.x * w, ly = m.y * h - size / 2 - 8;
      const clash = placed.some((q) => Math.abs(q[0] - lx) < 84 && Math.abs(q[1] - ly) < 12);
      if (!clash) placed.push([lx, ly]);
      el.lastChild.style.visibility = clash ? 'hidden' : 'visible';
      el.lastChild.style.bottom = `${size / 2}px`; el.lastChild.style.left = '0'; el.lastChild.style.marginBottom = '3px';
      seen.add(m.id);
    }
    for (const [id, el] of markEls) if (!seen.has(id)) { el.remove(); markEls.delete(id); }
    drawUnitPanel(JSON.parse(v.roster()), marks);
  };
  // Left click: select the unit under the cursor (the commander too), or clear the selection.
  const mapSelect = (e) => {
    const px = e.offsetX * dpr(), py = e.offsetY * dpr();
    const me = lastMarks.find((m) => m.you);
    if (me && Math.hypot(me.x * mapCanvas.clientWidth - e.offsetX, me.y * mapCanvas.clientHeight - e.offsetY) <= Math.max(16, me.r * mapCanvas.clientHeight)) {
      commanderPicked = true; onMap(() => v.select_only(-1)); unitSelect(); return;
    }
    commanderPicked = false;
    const msg = onMap(() => v.click(px, py, performance.now())); if (msg) flash(msg);
    if (msg && msg !== 'nothing selected') unitSelect();
  };
  // Right click: the commands menu for the selected unit, at the clicked place.
  const mapMenu = (e) => {
    closeActions();
    const px = e.offsetX * dpr(), py = e.offsetY * dpr();
    if (!onMap(() => v.selection_name())) { flash('Select a unit first (left click)'); return; }
    const [x, y] = onMap(() => v.map_point(px, py));
    const acts = JSON.parse(v.actions_at(x, y));
    const g4 = (m) => String(((Math.round(m / 10) % 10000) + 10000) % 10000).padStart(4, '0'); // 10 m squares
    const grid = `${g4(x)} ${g4(y)}`;
    actionsBox.innerHTML = `<div class="head">${onMap(() => v.selection_name())} → grid ${grid}</div>` + acts.map((a) =>
      `<button data-id="${a.id}" ${a.ok ? '' : 'disabled'} title="${a.reason}">${a.label}${a.ok ? '' : ` <small>${a.reason}</small>`}</button>`).join('') +
      '<button class="sitrep">Radio check · sitrep</button><button class="cancel">Cancel</button>';
    const r = mapCanvas.getBoundingClientRect();
    actionsBox.style.left = `${r.left + e.offsetX + 8}px`; actionsBox.style.top = `${r.top + e.offsetY + 8}px`;
    actionsBox.hidden = false;
    menuOpen();
    for (const b of actionsBox.querySelectorAll('button[data-id]')) b.addEventListener('click', () => {
      closeActions();
      const id = Number(b.dataset.id), label = b.textContent.trim();
      // A move: the walkie-talkie comes up first, with the decision of how they go.
      if (id === 0 || id === 20) decide((mode) => radioOrder(id, label, x, y, grid, mode));
      else radioOrder(id, label, x, y, grid, id === 22 ? 2 : 0);
    });
    actionsBox.querySelector('.cancel').addEventListener('click', closeActions);
    actionsBox.querySelector('.sitrep').addEventListener('click', () => { closeActions(); sitrep(JSON.parse(v.selection_ids())); });
  };
  // The walkie-talkie: comes up on screen, the commander speaks the order, waits for the unit, then it moves.
  const radio = $('radio');
  const VERB = { 20: ['move into the building at', 'moving into the building'], 21: ['clear the building at', 'clearing the building'], 22: ['garrison the building at', 'garrisoning the building'], 0: ['move to', 'moving to the coordinates'], 1: ['defend', 'defending the coordinates'], 2: ['assault', 'assaulting the coordinates'], 3: ['support from', 'supporting from the coordinates'], 10: ['take', 'taking the coordinates'], 11: ['hold', 'holding the coordinates'] };
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
  // The browser's speech voice cannot be listened to, so its loudness is followed from the text instead: where it
  // is in the sentence (re-synced on every word the voice reports, when it reports them), syllables as peaks,
  // spaces and punctuation as dips.
  let speech = null; // { text, t0, base, msPerChar }
  const speechEnergy = (now) => {
    if (!speech) return 0;
    const pos = speech.base + (now - speech.t0) / speech.msPerChar;
    if (pos >= speech.text.length) return 0.12;
    const ch = speech.text[Math.floor(pos)];
    if (/[.,!?]/.test(ch)) return 0.03;
    if (ch === ' ') return 0.18;
    return Math.min(1, 0.4 + 0.6 * Math.abs(Math.sin(pos * Math.PI / 2.6)) * (0.8 + 0.2 * Math.random()));
  };
  // While a set transmits, the net hiss runs through the analyser for real (band-limited noise), and a squelch tail
  // closes each transmission.
  let bed = null;
  const startBed = () => {
    const c = audioGraph(); if (!c || bed) return;
    if (c.state === 'suspended') c.resume().catch(() => {});
    const buf = c.createBuffer(1, c.sampleRate * 2, c.sampleRate), d = buf.getChannelData(0);
    for (let i = 0; i < d.length; i++) d[i] = (Math.random() * 2 - 1) * (Math.random() < 0.002 ? 3 : 1); // hiss with crackle
    const src = c.createBufferSource(); src.buffer = buf; src.loop = true;
    const bp = c.createBiquadFilter(); bp.type = 'bandpass'; bp.frequency.value = 1700; bp.Q.value = 0.7;
    const g = c.createGain(); g.gain.value = 0; g.gain.setTargetAtTime(0.06, c.currentTime, 0.03);
    src.connect(bp).connect(g).connect(analyser); src.start();
    bed = { src, g, c };
  };
  const stopBed = () => {
    if (!bed) return;
    const { src, g, c } = bed; bed = null;
    const t = c.currentTime;
    g.gain.cancelScheduledValues(t); g.gain.setValueAtTime(0.11, t); g.gain.exponentialRampToValueAtTime(0.0004, t + 0.2);
    src.stop(t + 0.22);
  };
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
    const now = performance.now();
    const e = Math.min(1, Math.max(speechEnergy(now), wordEnergy * Math.exp(-(now - wordAt) / 160)));
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
      const e = Math.max(speechEnergy(performance.now()), wordEnergy * Math.exp(-(performance.now() - wordAt) / 160));
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
    let over = false; const end = () => { if (!over) { over = true; speech = null; done(); } };
    u.onend = end; u.onerror = end; setTimeout(end, 1500 + text.length * 90);
    u.onboundary = (ev) => { if (speech) { speech.base = ev.charIndex; speech.t0 = performance.now(); } };
    u.onstart = () => { speech = { text, t0: performance.now(), base: 0, msPerChar: 66 / rate }; };
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
  // Who speaks with which recorded voice (scripts/radio_voices.py): four squad voices, one per captain.
  const VOICE_ID = { Alpha: 's1', Echo: 's1', Bravo: 's2', Foxtrot: 's2', Charlie: 's3', Golf: 's3', Delta: 's4', Hotel: 's4', Reyes: 'c1', Okafor: 'c2' };
  // For testing, every unit speaks with Hotel 08's voice (no variations); VOICE_ID keeps the per-squad table.
  const ONE_VOICE = 'Hotel 08';
  const voiceOf = (name) => { name = ONE_VOICE; let h = 0; for (const c of name) h = (h * 31 + c.charCodeAt(0)) >>> 0; return { id: VOICE_ID[name.split(' ')[0]], pitch: 0.6 + (h % 50) / 100, rate: 1.0 + (h % 7) / 40 }; };
  // Recorded lines: the manifest maps each voice's exact line text to its clip. Clips load on first use and stay
  // decoded; they play through the handset's radio chain (narrow band, overdriven speaker, compression) into the
  // analyser, so the trace follows the real voice. A line with no clip falls back to the browser's speech voice.
  let voiceManifest = null;
  fetch('./voice/manifest.json', { cache: 'no-cache' }).then((r) => r.json()).then((m) => { voiceManifest = m; }).catch(() => {});
  const clipCache = new Map();
  const clipFor = (vid, text) => {
    const f = voiceManifest && voiceManifest[vid] && voiceManifest[vid][text];
    const c = audioGraph();
    if (!f || !c) return Promise.resolve(null);
    if (!clipCache.has(f)) {
      clipCache.set(f, fetch(`./voice/${f}`).then((r) => { if (!r.ok) throw new Error(r.status); return r.arrayBuffer(); })
        .then((b) => c.decodeAudioData(b)).catch(() => { clipCache.delete(f); return null; }));
    }
    return clipCache.get(f);
  };
  let chainIn = null;
  const radioChain = () => {
    if (chainIn) return chainIn;
    const c = audioGraph();
    const hp = c.createBiquadFilter(); hp.type = 'highpass'; hp.frequency.value = 480; hp.Q.value = 0.9;
    const lp = c.createBiquadFilter(); lp.type = 'lowpass'; lp.frequency.value = 2500; lp.Q.value = 1.2;
    const mid = c.createBiquadFilter(); mid.type = 'peaking'; mid.frequency.value = 1500; mid.gain.value = 8; mid.Q.value = 1.1;
    const drive = c.createWaveShaper(); const curve = new Float32Array(1024);
    for (let i = 0; i < curve.length; i++) { const x = i / 511.5 - 1; curve[i] = Math.tanh(x * 5) / Math.tanh(5); }
    drive.curve = curve; drive.oversample = '2x';
    const comp = c.createDynamicsCompressor(); comp.threshold.value = -24; comp.ratio.value = 6; comp.attack.value = 0.003; comp.release.value = 0.12;
    const out = c.createGain(); out.gain.value = 0.85;
    hp.connect(lp).connect(mid).connect(drive).connect(comp).connect(out).connect(analyser);
    chainIn = hp;
    return chainIn;
  };
  // The other side comes through the heavy radio chain; the commander's own voice is heard clean.
  const playClip = (buf, clean) => new Promise((done) => {
    const c = audioGraph();
    if (c.state === 'suspended') c.resume().catch(() => {});
    const src = c.createBufferSource(); src.buffer = buf; src.connect(clean ? analyser : radioChain());
    let over = false; const end = () => { if (!over) { over = true; done(); } };
    src.onended = end; setTimeout(end, buf.duration * 1000 + 600);
    src.start();
  });
  const keyUp = async () => { radio.classList.add('up'); await new Promise((r) => setTimeout(r, 450)); };
  const transmit = async (cls, who, name, text, voice) => {
    await click(); radio.classList.add(cls); radioState(cls === 'tx' ? 'TRANSMITTING' : 'RECEIVING'); line(who, name, text);
    traffic.unshift({ when: $('pdaClock').textContent.slice(0, 5), name: cls === 'tx' ? ident.cmdr : name, text, me: cls === 'tx' });
    traffic.length = Math.min(traffic.length, 20);
    if (cls === 'rx') startBed();
    const clip = voice.id ? await clipFor(voice.id, text) : null;
    if (clip) await playClip(clip, cls === 'tx');
    else { if (voiceManifest && voice.id) console.debug('radio: no recorded clip for', voice.id, JSON.stringify(text)); await say(text.replace(/(\d)/g, '$1 '), voice.pitch, voice.rate); }
    stopBed();
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
  // ---- Radio procedure ----
  // Nothing is said without a call-up first, as on a real net. A unit that wants to report calls "Actual" and
  // waits: the call shows as an Answer Transmission button (E) and the handset stays down until it is answered.
  // To give an order or ask for a sitrep the commander calls the unit and waits for "send it"; a unit that is busy
  // or misses the call gets a second call, then the commander gives up on it.
  const ACTUAL = 'Commander'; // how the net names the commander
  const pad2 = (n) => String(n).padStart(2, '0');
  const csOf = (u) => `${u.cs} ${pad2(u.n)}`;
  const wait = (ms) => new Promise((r) => setTimeout(r, ms));
  const ME = { id: 'cmd', pitch: 1.0, rate: 1.05 };
  const sayTx = (text) => transmit('tx', 'cmd', 'Commander', text, ME);
  const sayRx = (name, text) => transmit('rx', 'unit', name, text, voiceOf(name));
  const busy = (u) => /Assault/.test(u.task || '');
  // Call a unit up; true once it answers "send it".
  const callUp = async (u) => {
    const name = csOf(u);
    talkingTo('CALLING', name);
    await sayTx(`${name}, this is ${ACTUAL}. Over.`);
    for (let attempt = 0; attempt < 2; attempt++) {
      await wait(900 + Math.random() * 800);
      if (!busy(u) && (attempt > 0 || Math.random() > 0.12)) {
        talkingTo('TALKING TO', name);
        await sayRx(name, `${ACTUAL}, ${name}. Send it. Over.`);
        return true;
      }
      radioState('NO REPLY');
      if (attempt === 0) { await wait(1300); await sayTx(`${name}, ${name}, this is ${ACTUAL}. Over.`); }
    }
    await wait(1200); radioState('ONLINE');
    flash(`${name} does not answer`);
    return false;
  };
  // Radio check and situation report from each selected unit (or one unit from the roster).
  const sitrep = (ids) => {
    net.push(async () => {
      $('radioLog').innerHTML = ''; v.set_radio(true); await keyUp();
      for (const id of ids) {
        const u = JSON.parse(v.roster()).find((x) => x.id === id);
        if (!u || !(await callUp(u))) continue;
        await sayTx('Send sitrep. Over.');
        await wait(600 + Math.random() * 600);
        const doing = u.status === 'holding' ? 'holding, no contact' : u.status === 'moving' || u.status === 'marching' ? 'on the move' : u.status;
        await sayRx(csOf(u), `At position, ${u.soldiers} effective, ${doing}. Over.`);
        await sayTx(`${ACTUAL} copies. Out.`);
      }
      await wait(300); v.set_radio(false);
    });
    pump();
  };
  // Incoming calls: wait for the commander to answer (urgent ones first, at most a few waiting).
  const incoming = [];
  const answerBox = $('answerCall');
  const ringTone = sfx('./sfx/walkie-ring.mp3', 0.5);
  // Urgent calls (under fire, casualties, supplies gone: things that cannot wait) add an alarm over the ring.
  const urgentTone = sfx('./sfx/urgent-alert.mp3', 0.6);
  let ringTimer = 0;
  const showCalls = () => {
    answerBox.hidden = !incoming.length;
    if (!incoming.length) { clearInterval(ringTimer); ringTimer = 0; return; }
    const c = incoming[0];
    answerBox.querySelector('small').textContent = `${c.from} ${pad2(c.n)} calling${c.urgent ? ' · URGENT' : ''}${incoming.length > 1 ? ` · ${incoming.length - 1} more waiting` : ''}`;
    answerBox.classList.toggle('urgent', incoming.some((x) => x.urgent));
    const ring = () => { ringTone(); if (incoming.some((x) => x.urgent)) urgentTone(); };
    if (!ringTimer) { ring(); ringTimer = setInterval(ring, 4000); } // the 1.6 s ring, then a pause
    else if (incoming[0].urgent && !incoming[0].alerted) urgentTone(); // a new urgent call alarms at once
    incoming.forEach((x) => { x.alerted = true; });
  };
  const hear = (ev) => {
    const same = incoming.findIndex((x) => x.id === ev.id);
    if (same >= 0) incoming.splice(same, 1); // a newer report from the same unit replaces the old one
    if (ev.urgent) incoming.unshift(ev); else incoming.push(ev);
    while (incoming.length > 5) incoming.splice(incoming.findLastIndex((x) => !x.urgent), 1);
    showCalls();
  };
  const answer = () => {
    const ev = incoming.shift(); showCalls();
    if (!ev) return;
    const name = `${ev.from} ${pad2(ev.n)}`;
    const body = ev.text.replace(/^(Actual|Commander),\s*[^.!]*\.\s*/, '').replace(/\s*(Over|Out)[.!]?\s*$/, '').replace(/[.!]\s*$/, '');
    net.unshift(async () => {
      $('radioLog').innerHTML = ''; v.set_radio(true); await keyUp();
      talkingTo('INCOMING FROM', name);
      const end = ev.urgent ? '! Over!' : '. Over.';
      await sayRx(name, `${ACTUAL}, this is ${name}${end}`);
      await wait(ev.urgent ? 150 : 400 + Math.random() * 400);
      await sayTx(ev.urgent ? `${name}, ${ACTUAL}. Send it! Over!` : `${name}, ${ACTUAL}. Send it. Over.`);
      await wait(ev.urgent ? 200 : 500 + Math.random() * 500);
      await sayRx(name, `${body}${end}`);
      await sayTx(`${ACTUAL} copies. Out.`);
      await wait(300); v.set_radio(false);
    });
    pump();
  };
  answerBox.querySelector('button').addEventListener('click', answer);
  setInterval(() => { try { for (const ev of JSON.parse(v.radio_events())) hear(ev); } catch (e) { /* viewer busy */ } }, 400);
  // How a move is made: decided with the walkie-talkie up, before the call goes out.
  const MODES = [
    { m: 0, label: 'Move there', say: '', ack: '' },
    { m: 1, label: 'Proceed with caution · possible hostiles', say: ' Proceed with caution, possible hostile presence.', ack: ', moving with caution' },
    { m: 2, label: 'Garrison this area', say: ' Garrison the area.', ack: ', will garrison' },
    { m: 3, label: 'Secure the area · hostiles confirmed', say: ' Secure the area, hostile presence confirmed.', ack: ', expecting contact' },
  ];
  const decide = (then) => {
    radio.classList.add('up');
    const box = document.createElement('div');
    box.className = 'decide';
    box.innerHTML = '<div class="head">How do they go?</div>' + MODES.map((o) => `<button data-m="${o.m}">${o.label}</button>`).join('') + '<button class="cancel">Cancel</button>';
    document.body.appendChild(box);
    // Beside the walkie-talkie once it has come up (it slides in), kept on screen.
    const place = () => {
      const r = radio.getBoundingClientRect();
      const left = r.left > innerWidth / 2 ? r.left - box.offsetWidth - 12 : r.right + 12;
      const top = Math.min(r.top + r.height * 0.25, innerHeight - box.offsetHeight - 8);
      Object.assign(box.style, { left: `${Math.max(8, Math.min(left, innerWidth - box.offsetWidth - 8))}px`, top: `${Math.max(8, top)}px` });
    };
    box.style.visibility = 'hidden';
    setTimeout(() => { place(); box.style.visibility = ''; }, 470);
    const done = (m) => {
      box.remove();
      if (m == null) { if (!onAir && !net.length) radio.classList.remove('up'); return; }
      then(m);
    };
    for (const b of box.querySelectorAll('button[data-m]')) b.addEventListener('click', () => done(Number(b.dataset.m)));
    box.querySelector('.cancel').addEventListener('click', () => done(null));
  };
  // Orders: call each selected unit up; the ones that answer get the order and read it back.
  const radioOrder = (id, label, x, y, grid, mode = 0) => {
    const how = MODES.find((o) => o.m === mode) || MODES[0];
    const [verb, ing] = VERB[id] || VERB[0];
    const ids = JSON.parse(v.selection_ids());
    if (onAir) flash('net busy: your call goes out next');
    net.push(async () => {
      $('radioLog').innerHTML = ''; v.set_radio(true); await keyUp();
      for (const uid of ids) {
        const u = JSON.parse(v.roster()).find((q) => q.id === uid);
        if (!u || !(await callUp(u))) continue;
        const name = csOf(u);
        await sayTx(`${verb.charAt(0).toUpperCase() + verb.slice(1)} the coordinates transmitted.${how.say} Over.`);
        await wait(500 + Math.random() * 700);
        await sayRx(name, `Roger, coordinates received, ${ing}${how.ack}. Over.`);
        await sayTx(`${ACTUAL}, out.`);
        const keep = new Uint32Array(JSON.parse(v.selection_ids()));
        v.set_selection(new Uint32Array([uid]));
        v.set_order_mode(mode);
        const msg = v.issue_at(id, x, y, performance.now()); if (msg) flash(msg);
        v.set_selection(keep);
        if (page === 'roster') drawRoster();
      }
      await wait(400); v.set_radio(false);
    });
    pump();
  };
  // The blur only runs while the view really changes: at the end of the zoom range it stays sharp.
  const zoomSound = sfx('./sfx/map-zoom.mp3', 0.5);
  let zoomSoundAt = 0;
  mapCanvas.addEventListener('wheel', notches((d) => {
    if (!onMap(() => v.wheel(d))) return;
    zoomGlitch();
    if (performance.now() - zoomSoundAt > 140) { zoomSoundAt = performance.now(); zoomSound(); } // one whoosh per step, not a pile-up
  }), { passive: false });

  const drawRoster = () => {
    const units = JSON.parse(v.roster());
    const captains = units.filter((u) => u.captain);
    const boss = (u) => u.captain ? '<span class="rank">Captain</span>' :
      `<select class="boss" data-id="${u.id}"><option value="-1"${u.boss < 0 ? ' selected' : ''}>You</option>${
        captains.map((c) => `<option value="${c.id}"${u.boss === c.id ? ' selected' : ''}>${c.name}</option>`).join('')}</select>`;
    const cap = (t) => t.charAt(0).toUpperCase() + t.slice(1);
    const row = (u) => `<tr class="unit${u.selected ? ' sel' : ''}${u.captain ? ' capt' : ''}" data-id="${u.id}"><td>${u.name}</td><td>${boss(u)}</td><td class="num">${u.soldiers}</td><td class="st-${u.status}">${u.task || cap(u.status)}</td><td>${cap(u.stance)}</td><td><button class="cmdu call" data-id="${u.id}">Call</button> <button class="cmdu" data-id="${u.id}">Command Unit</button></td></tr>`;
    $('roster').innerHTML = `<table><tr><th>Unit</th><th>Answers to</th><th>Manpower</th><th>Status</th><th>Stance</th><th></th></tr>${
      captains.map(row).join('')}${units.filter((u) => !u.captain).map(row).join('')
    }</table><p class="note">Click a unit to find it on the map. Attach squads to a captain and give him a goal: he plans the rest and leads them.</p>`;
    for (const r of $('roster').querySelectorAll('tr.unit')) r.addEventListener('click', (e) => {
      if (e.target.tagName === 'SELECT' || e.target.tagName === 'OPTION' || e.target.closest('button')) return;
      onMap(() => v.select_unit(Number(r.dataset.id))); setPage('map');
    });
    for (const b of $('roster').querySelectorAll('button.cmdu.call')) b.addEventListener('click', () => sitrep([Number(b.dataset.id)]));
    for (const b of $('roster').querySelectorAll('button.cmdu:not(.call)')) b.addEventListener('click', () => {
      onMap(() => v.select_unit(Number(b.dataset.id))); unitSelect(); setPage('map');
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
  // ---- Inventory (Q): the field around the commander blurs and his kit floats in a ring around him. Pick a slot
  // with a small move of the mouse from the middle (its direction), or keys 1 to 5; click or Enter to use it.
  const DEBUG = new URLSearchParams(location.search).has('debug');
  window.scaliaField = () => v.field_debug();
  const SLOTS = [
    { key: 'main', name: 'Main weapon', item: 'M4 carbine', note: '5.56 mm, 30-round magazine', icon: '<path d="M6 30h34l4-4h10v6h-6l-2 4H28l-2 6h-6l2-6H6z"/>' },
    { key: 'belt', name: 'Belt', item: '6 magazines, 2 frag grenades', note: 'Within reach while moving', icon: '<path d="M8 26h48v12H8z M14 22h8v4h-8z M28 22h8v4h-8z M42 22h8v4h-8z"/>' },
    { key: 'pda', name: 'PDA', item: 'Command tablet', note: 'Map, roster, orders (Tab)', icon: '<path d="M14 14h36v36H14z M18 18v24h28V18z M28 45h8v2h-8z"/>' },
    { key: 'pack', name: 'Backpack', item: 'Water, rations, radio battery', note: 'Slow to reach', icon: '<path d="M20 18c0-6 24-6 24 0v32H20z M24 32h16v10H24z M26 14h12v4H26z"/>' },
    { key: 'side', name: 'Secondary weapon', item: 'Sidearm', note: '9 mm pistol', icon: '<path d="M10 26h30l4 4h6v6h-6l-4 4h-8l-2 8h-7l2-8h-5z"/>' },
  ];
  let invOpen = false, invSel = -1, invVec = [0, 0];
  const invEl = document.createElement('div');
  invEl.id = 'inventory';
  invEl.hidden = true;
  invEl.innerHTML = '<div class="blur"></div>' + SLOTS.map((sl, k) => `<div class="slot" data-k="${k}"><svg viewBox="0 0 64 64">${sl.icon}</svg><b>${k + 1}</b><span>${sl.name}</span><small>${sl.item}</small></div>`).join('') + '<div class="cursor"></div><div class="hint">Mouse or 1-5 to pick · click or Enter to use · Q or Esc to close</div>';
  document.body.appendChild(invEl);
  const R_RING = Math.min(innerWidth, innerHeight) * 0.2, R_PICK = 22, R_MAX = 70;
  const invLayout = () => {
    const c = v.commander_screen();
    const cx = c[0] * innerWidth, cy = c[1] * innerHeight;
    invEl.style.setProperty('--cx', `${cx}px`); invEl.style.setProperty('--cy', `${cy}px`);
    invEl.querySelectorAll('.slot').forEach((el, k) => {
      const a = -Math.PI / 2 + k * 2 * Math.PI / SLOTS.length;
      el.style.left = `${cx + Math.cos(a) * R_RING}px`; el.style.top = `${cy + Math.sin(a) * R_RING}px`;
    });
    const cur = invEl.querySelector('.cursor');
    cur.style.left = `${cx + invVec[0]}px`; cur.style.top = `${cy + invVec[1]}px`;
  };
  const invPick = (k) => {
    invSel = k;
    invEl.querySelectorAll('.slot').forEach((el, j) => el.classList.toggle('on', j === k));
  };
  const openInv = () => {
    invOpen = true; invSel = -1; invVec = [0, 0];
    invEl.hidden = false; invLayout(); invPick(-1);
    requestAnimationFrame(() => invEl.classList.add('in'));
  };
  const closeInv = () => { invOpen = false; invEl.classList.remove('in'); setTimeout(() => { if (!invOpen) invEl.hidden = true; }, 180); };
  const useSlot = (k) => {
    const sl = SLOTS[k]; if (!sl) return;
    closeInv();
    if (sl.key === 'pda') { if (!tabletUp && v.tablet_view() === 0) setTablet(true); return; }
    flash(`${sl.name}: ${sl.item} (${sl.note})`);
  };
  const invKey = (k, e) => {
    if (k === 'q' || k === 'escape') { if (!e.repeat) closeInv(); return; }
    if (k >= '1' && k <= '5') { invPick(Number(k) - 1); return; }
    if (k === 'enter' && invSel >= 0) useSlot(invSel);
  };
  // Mouse: a small cursor moves from the middle, held within a short reach; its direction picks the slot.
  addEventListener('mousemove', (e) => {
    if (!invOpen) return;
    invVec = [invVec[0] + e.movementX, invVec[1] + e.movementY];
    const d = Math.hypot(invVec[0], invVec[1]);
    if (d > R_MAX) invVec = [invVec[0] * R_MAX / d, invVec[1] * R_MAX / d];
    if (d > R_PICK) {
      const a = Math.atan2(invVec[1], invVec[0]) + Math.PI / 2;
      invPick(((Math.round(a / (2 * Math.PI / SLOTS.length)) % SLOTS.length) + SLOTS.length) % SLOTS.length);
    }
    invLayout();
  });
  invEl.addEventListener('pointerdown', (e) => { e.preventDefault(); if (invSel >= 0) useSlot(invSel); });
  // ---- Training assault (K): a squad takes the killhouse from the enemy inside, on the battle AI ----
  // Battle sounds go through the Pack Engine audio graph (audio.js): the acoustics place every shot, blast and
  // footfall (walls, distance, zoom, the listener's space, headphones 3D), the graph plays it.
  const audio = new AudioEngine();
  const SHOT = './sfx/m4-shot.mp3', STEPS = './sfx/steps-grass.mp3';
  const fireGraph = () => { const c = audio.ensure(); if (c) { audio.buffer(SHOT); audio.buffer(STEPS); } return c; };
  let envAt = 0;
  const STEP_SAY = { approach: 'moving up on the house', drone: 'drone up, looking inside', stack: 'stacked on the door', grenade: 'frag out',
    storm: 'going in', 'slow entry': 'entering slow', clear: 'clearing rooms', 'room clear': 'room clear', consolidate: 'consolidating', holding: 'house is ours, holding' };
  let assaultOn = false, assaultPoll = 0, lastAlive = null;
  const assaultTick = (now) => {
    // The field battle's sounds (gunfire, blasts, footfalls), whoever is fighting: tablet orders or the K demo.
    const s = v.take_sounds();
    if (s.length) fireGraph();
    if (audio.ctx && now - envAt > 100) { envAt = now; audio.setEnv(v.audio_env()); }
    for (let i = 0; i + 10 < s.length; i += 11) {
      const [kind, loud, delay, gain, lowpass, room, far, rx, ry] = s.subarray(i, i + 9);
      if (kind === 2) audio.play(STEPS, { kind: 'step', gain: gain * (0.25 + 0.35 * loud), lowpass, room: room * 0.6, far: far * 0.5, rx, ry, delay, rate: 0.9 + Math.random() * 0.2 });
      else if (kind === 1) audio.play(SHOT, { kind: 'blast', gain: Math.min(4, gain * 6), lowpass, room, far, rx, ry, delay, rate: 0.45 });
      else audio.play(SHOT, { kind: 'shot', gain: Math.min(2.5, gain * 2.2), lowpass, room, far, rx, ry, delay, rate: 0.94 + Math.random() * 0.12 });
    }
    if (!assaultOn) return;
    if (now - assaultPoll < 250) return;
    assaultPoll = now;
    let st; try { st = JSON.parse(v.assault_state()); } catch (_) { return; }
    if (!st.running) { assaultOn = false; return; }
    // Casualty calls: a man of ours down, or an enemy down (from the alive counts).
    if (lastAlive && !st.over) {
      if (st.attackers < lastAlive.a) flash(`Alpha 1: man down! (${st.attackers} up)`);
      else if (st.defenders < lastAlive.d) flash(`Alpha 1: enemy down, ${st.defenders} left`);
    }
    lastAlive = { a: st.attackers, d: st.defenders };
    for (const step of st.steps) {
      if (step.squad === 0 && step.step.startsWith('bark:')) flash(`Alpha 1: "${step.step.slice(5).replace(/^./, (c) => c.toUpperCase())}!"`);
      else if (step.squad === 0 && STEP_SAY[step.step]) flash(`Alpha 1: ${STEP_SAY[step.step]} (${st.attackers} up)`);
      else if (step.squad === 1 && step.step === 'fall back') flash('Enemy falling back inside the house');
    }
    if (st.over) {
      assaultOn = false;
      flash(st.defenders === 0 ? `Killhouse taken: ${st.attackers} of 12 still up. K to run it again.` : `Assault failed: ${st.defenders} enemy left. K to run it again.`);
    }
  };
  const toggleAssault = () => {
    if (assaultOn) { v.stop_assault(); assaultOn = false; flash('Training assault called off'); return; }
    fireGraph();
    v.start_assault((Math.random() * 1e9) >>> 0);
    assaultOn = true; assaultPoll = 0; lastAlive = null;
    flash('Training assault: Alpha 1 takes the killhouse');
  };
  addEventListener('keydown', (e) => {
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLSelectElement) return;
    const k = e.key.toLowerCase();
    held.add(k);
    if (k === 'k' && !e.repeat && !tabletUp) { toggleAssault(); return; }
    if (k === 'h' && !e.repeat) { const c = $('controls'); c.hidden = !c.hidden; store.set('scalia.controls', c.hidden ? '0' : '1'); return; }
    if (k === 'tab') {
      e.preventDefault();
      if (e.repeat) return;
      // One thing at a time: the tablet finishes opening before it can close, and is fully put away (camera
      // back) before it can open again, however fast Tab is pressed.
      if (tabletUp && tabletShown) pressOff();
      else if (!tabletUp && v.tablet_view() === 0 && !pressingOff) setTablet(true);
      return;
    }
    if (k === 'e' && !$('answerCall').hidden) { answer(); return; }
    if (invOpen) { invKey(k, e); return; }
    if (k === 'q' && !e.repeat && !tabletUp) { openInv(); return; }
    if (DEBUG && k === '1') setScene('ships');
    else if (DEBUG && k === '2') setScene('ground');
    else if (DEBUG && k === '3') setScene('planet');
    else if (DEBUG && k === '4') setScene('system');
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

  // Benchmark: ~1,000 soldiers around the commander, the camera turns one full circle in 20 s
  // after a 3 s warm-up; reports frame, CPU and GPU times. Start with ?bench in the URL or scaliaBench().
  let bench = null;
  function runBench(squads = 84) {
    if (bench) return;
    if (scene !== 'ground') { v.set_scene('ground'); scene = 'ground'; }
    const soldiers = v.bench_army(squads);
    const box = document.getElementById('benchBox') || Object.assign(document.createElement('div'), { id: 'benchBox' });
    box.style.cssText = 'position:fixed;left:50%;top:14px;transform:translateX(-50%);z-index:100000;background:#0d1410e6;color:#d8e6d0;border:1px solid #5d7a52;padding:10px 14px;font:13px/1.5 system-ui,sans-serif;white-space:pre;pointer-events:auto';
    document.body.appendChild(box);
    box.textContent = `Benchmark: ${soldiers} soldiers, warming up…`;
    bench = { start: performance.now(), warm: 3000, run: 20000, soldiers, cap: capIndex, box, frames: [], cpu: [], gpu: [], prev: 0 };
    capIndex = 0; // uncapped while measuring
  }
  function pct(a, q) { const s = [...a].sort((x, y) => x - y); return s[Math.min(s.length - 1, Math.floor(q * s.length))]; }
  function benchTick(now, cpu) {
    const t = now - bench.start;
    if (t < bench.warm) { bench.prev = now; return; }
    if (t < bench.warm + bench.run) {
      bench.frames.push(now - bench.prev); bench.prev = now;
      bench.cpu.push(cpu);
      const p = v.perf();
      if (p[3] >= 0) bench.gpu.push(p[3] + p[4] + p[5]);
      bench.box.textContent = `Benchmark: ${bench.soldiers} soldiers, measuring ${((t - bench.warm) / 1000).toFixed(0)} / ${bench.run / 1000} s`;
      return;
    }
    const avg = (a) => a.reduce((x, y) => x + y, 0) / Math.max(1, a.length);
    const f = bench.frames, c = bench.cpu, g = bench.gpu;
    const p = v.perf();
    const r = {
      soldiers: bench.soldiers, frames: f.length, resolution: `${p[1]}x${p[2]}`,
      fps: 1000 / avg(f), frame_avg: avg(f), frame_p95: pct(f, 0.95), frame_worst: Math.max(...f),
      cpu_avg: avg(c), cpu_p95: pct(c, 0.95), cpu_worst: Math.max(...c),
      gpu_avg: g.length ? avg(g) : null, gpu_p95: g.length ? pct(g, 0.95) : null, gpu_worst: g.length ? Math.max(...g) : null, ua: navigator.userAgent,
    };
    console.log('Scalia benchmark', JSON.stringify(r));
    const ms = (x) => (x == null ? 'n/a' : `${x.toFixed(2)} ms`);
    bench.box.textContent = `Benchmark result: ${r.soldiers} soldiers, ${r.resolution} internal, ${r.frames} frames
Average ${r.fps.toFixed(1)} fps
Frame   avg ${ms(r.frame_avg)}   95% ${ms(r.frame_p95)}   worst ${ms(r.frame_worst)}
CPU     avg ${ms(r.cpu_avg)}   95% ${ms(r.cpu_p95)}   worst ${ms(r.cpu_worst)}
GPU     avg ${ms(r.gpu_avg)}   95% ${ms(r.gpu_p95)}   worst ${ms(r.gpu_worst)}
Click to copy and close.`;
    bench.box.onclick = () => { navigator.clipboard?.writeText(JSON.stringify(r)).catch(() => {}); bench.box.remove(); };
    window.scaliaBenchResult = r;
    capIndex = bench.cap;
    bench = null;
  }
  window.scaliaBench = runBench;
  if (new URLSearchParams(location.search).has('bench')) setTimeout(() => runBench(), 1500);

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
      if ((held.has('s') && !scanHolding()) || held.has('arrowdown')) py -= speed;
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
      if (held.has(',') || held.has('<')) v.rotate(-1.5 * dt);
      if (held.has('.') || held.has('>')) v.rotate(1.5 * dt);
      if (bench) v.rotate((Math.PI * 2 / 20) * dt);
      const t0 = performance.now();
      const line = v.frame(now);
      syncTablet();
      drawFieldLabels();
      if (lab) lab.draw();
      drawMapMarks(); drawScan();
      footsteps();
      assaultTick(now);
      const cpuFrame = performance.now() - t0;
      cpuMs = cpuMs * 0.9 + cpuFrame * 0.1;
      if (bench) benchTick(now, cpuFrame);
      const showNote = now < noteUntil;
      hud.hidden = !showNote || tabletUp;
      if (showNote) hud.textContent = note;
      perfPanel(v.perf(), cpuMs, now, cap);
      const now_scene = v.scene_name();
      if (now_scene !== scene) scene = now_scene;
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
  const go = () => { box.hidden = true; removeEventListener('keydown', key, true); music.start(); ambience(); };
  // While it is up, keys go nowhere else; Enter, Space or Escape enters.
  const key = (e) => { e.stopPropagation(); e.preventDefault(); if (e.key === 'Enter' || e.key === ' ' || e.key === 'Escape') go(); };
  document.getElementById('protoGo').addEventListener('click', go);
  addEventListener('keydown', key, true);
})();
(function watchForUpdates() {
  const mine = new URL(import.meta.url).searchParams.get('v');
  if (!mine) return; // local build
  const bar = document.getElementById('update');
  const alert = new Audio('./sfx/update-alert.mp3'); alert.preload = 'auto'; alert.volume = 0.6;
  document.getElementById('update-reload').addEventListener('click', reboot);
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
        if (bar.hidden) { alert.currentTime = 0; alert.play().catch(() => {}); bar.hidden = false; bar.style.animation = 'none'; void bar.offsetWidth; bar.style.animation = ''; }
      }
    } catch (_) { /* offline: try again later */ }
  };
  setInterval(check, 60_000);
  check();
})();
