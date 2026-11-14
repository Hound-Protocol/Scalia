// Developer console, as in Source games: the key under Esc opens it as a window on the right of the screen, which
// can be dragged by its title bar and resized from its corner (both remembered). It shows the game's log (console
// output included) and takes commands; up/down walk the history, Tab completes a command.

export function installConsole(commands) {
  const css = document.createElement('style');
  css.textContent = `
  #devcon { position: fixed; right: 16px; top: 64px; width: min(560px, 46vw); height: 58vh; min-width: 300px; min-height: 160px; z-index: 1000;
    display: flex; flex-direction: column; resize: both; overflow: hidden; background: rgba(12, 14, 12, 0.95); border: 1px solid #4b5a45;
    box-shadow: 0 8px 28px rgba(0,0,0,0.55); font: 12px/1.4 ui-monospace, monospace; color: #d6dfcf;
    opacity: 0; visibility: hidden; transform: translateX(12px); transition: opacity 0.1s ease-out, transform 0.1s ease-out, visibility 0s 0.1s; }
  #devcon.open { opacity: 1; visibility: visible; transform: none; transition: opacity 0.1s ease-out, transform 0.1s ease-out; }
  #devcon .bar { display: flex; align-items: center; gap: 8px; padding: 4px 6px 4px 10px; background: #1f261c; border-bottom: 1px solid #3c4a38;
    cursor: move; user-select: none; color: #b9c7a8; letter-spacing: 0.08em; }
  #devcon .bar span { flex: 1; } #devcon .bar button { font: inherit; color: inherit; background: none; border: 0; padding: 0 6px; cursor: pointer; }
  #devcon .bar button:hover, #devcon .bar button:focus-visible { color: #fff; outline: 1px solid #4b5a45; }
  #devcon .out { flex: 1; overflow-y: auto; padding: 6px 10px; white-space: pre-wrap; }
  #devcon .out .warn { color: #ffd479; } #devcon .out .err { color: #ff8a7a; } #devcon .out .cmd { color: #9fd0ff; }
  #devcon input { font: inherit; color: inherit; background: rgba(0,0,0,0.4); border: 0; border-top: 1px solid #3c4a38; padding: 6px 10px; outline: none; }
  @media (prefers-reduced-motion: reduce) { #devcon, #devcon.open { transition: none; } }`;
  document.head.appendChild(css);
  const el = document.createElement('div');
  el.id = 'devcon';
  el.setAttribute('role', 'dialog'); el.setAttribute('aria-label', 'Console');
  el.innerHTML = '<div class="bar"><span>CONSOLE</span><button data-reset title="Back to the right side">Dock</button><button data-close title="Close (key under Esc)">✕</button></div>'
    + '<div class="out"></div><input spellcheck="false" autocomplete="off" aria-label="Console command">';
  document.body.appendChild(el);
  // Where the player left it: position and size, kept between visits.
  const KEY = 'scalia.console';
  const save = () => { try { const r = el.getBoundingClientRect(); localStorage.setItem(KEY, JSON.stringify({ x: r.left, y: r.top, w: r.width, h: r.height })); } catch (_) {} };
  const place = (g) => {
    if (!g) { Object.assign(el.style, { left: '', top: '', right: '', width: '', height: '' }); return; }
    const w = Math.min(g.w, innerWidth), h = Math.min(g.h, innerHeight);
    Object.assign(el.style, { right: 'auto', width: w + 'px', height: h + 'px',
      left: Math.max(0, Math.min(innerWidth - 80, g.x)) + 'px', top: Math.max(0, Math.min(innerHeight - 32, g.y)) + 'px' });
  };
  try { place(JSON.parse(localStorage.getItem(KEY) || 'null')); } catch (_) {}
  const bar = el.querySelector('.bar');
  bar.addEventListener('pointerdown', (e) => {
    if (e.target.closest('button')) return;
    const r = el.getBoundingClientRect(), dx = e.clientX - r.left, dy = e.clientY - r.top;
    bar.setPointerCapture(e.pointerId);
    const move = (m) => place({ x: m.clientX - dx, y: m.clientY - dy, w: r.width, h: r.height });
    const up = () => { bar.removeEventListener('pointermove', move); bar.removeEventListener('pointerup', up); save(); };
    bar.addEventListener('pointermove', move); bar.addEventListener('pointerup', up);
  });
  // Resizing from the corner: remembered once the mouse lets go.
  new ResizeObserver(() => { if (el.classList.contains('open')) { clearTimeout(el._rs); el._rs = setTimeout(save, 300); } }).observe(el);
  el.querySelector('[data-close]').addEventListener('click', () => toggle(false));
  el.querySelector('[data-reset]').addEventListener('click', () => { try { localStorage.removeItem(KEY); } catch (_) {} place(null); input.focus(); });
  // Clicks inside the window stay with it (the game does not select or move underneath).
  for (const t of ['pointerdown', 'wheel', 'contextmenu']) el.addEventListener(t, (e) => e.stopPropagation());
  const out = el.querySelector('.out'), input = el.querySelector('input');
  const history = [];
  let hpos = 0;

  const print = (text, cls = '') => {
    const line = document.createElement('div');
    if (cls) line.className = cls;
    line.textContent = text;
    out.appendChild(line);
    while (out.childNodes.length > 500) out.firstChild.remove();
    out.scrollTop = out.scrollHeight;
  };
  // The game's own log lands here too.
  for (const [k, cls] of [['log', ''], ['info', ''], ['warn', 'warn'], ['error', 'err']]) {
    const orig = console[k].bind(console);
    console[k] = (...a) => { orig(...a); try { const msg = a.map(String).join(' '); if (cls && window.scaliaNote && !/^\[(error|crash)\]/.test(msg)) window.scaliaNote(k, msg); print(a.map((x) => (typeof x === 'string' ? x : x instanceof Error ? x.message : JSON.stringify(x))).join(' '), cls); } catch (_) {} };
  }
  addEventListener('error', (e) => print(`error: ${e.message}`, 'err'));

  const all = () => ({ help: { help: 'list commands', run: () => Object.entries(all()).forEach(([n, c]) => print(`  ${n.padEnd(10)} ${c.help}`)) },
    clear: { help: 'clear the console', run: () => { out.textContent = ''; } }, ...commands });
  const run = (line) => {
    print(`] ${line}`, 'cmd');
    const [name, ...args] = line.trim().split(/\s+/);
    if (!name) return;
    const c = all()[name.toLowerCase()];
    if (!c) { print(`unknown command "${name}" (help lists them)`, 'warn'); return; }
    try { const r = c.run(args, print); if (r !== undefined) print(String(r)); } catch (e) { print(String(e), 'err'); }
  };

  const toggle = (open = !el.classList.contains('open')) => {
    el.classList.toggle('open', open);
    if (open) setTimeout(() => input.focus(), 0); else input.blur();
  };
  // The key under Esc, wherever the layout puts its character; caught before the game sees it.
  addEventListener('keydown', (e) => {
    if (e.code === 'Backquote' || e.key === '`' || e.key === '~') {
      e.preventDefault(); e.stopImmediatePropagation();
      toggle();
    }
  }, true);
  input.addEventListener('keydown', (e) => {
    e.stopPropagation();
    if (e.key === 'Enter') { const v = input.value; input.value = ''; if (v.trim()) { history.push(v); hpos = history.length; run(v); } }
    else if (e.key === 'Escape') toggle(false);
    else if (e.key === 'ArrowUp') { e.preventDefault(); hpos = Math.max(0, hpos - 1); input.value = history[hpos] || ''; }
    else if (e.key === 'ArrowDown') { e.preventDefault(); hpos = Math.min(history.length, hpos + 1); input.value = history[hpos] || ''; }
    else if (e.key === 'Tab') {
      e.preventDefault();
      const m = Object.keys(all()).filter((n) => n.startsWith(input.value.trim().toLowerCase()));
      if (m.length === 1) input.value = m[0] + ' '; else if (m.length) print(m.join('  '));
    }
  });
  input.addEventListener('keyup', (e) => e.stopPropagation());
  print('Scalia console. help lists commands; the key under Esc closes it.');
  return { print, run, toggle };
}
