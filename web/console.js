// Developer console, as in Source games: the key under Esc opens it as a window on the right of the screen, which
// can be dragged by its title bar and resized from its corner (both remembered). It shows the game's log (console
// output included) and takes commands; up/down walk the history, Tab completes a command.

export function installConsole(initialCommands = {}) {
  const css = document.createElement('style');
  css.textContent = `
  #devcon { position: fixed; right: 16px; top: 64px; width: min(600px, 46vw); height: 60vh; min-width: 320px; min-height: 180px; z-index: 1000;
    display: flex; flex-direction: column; resize: both; overflow: hidden; background: rgba(10, 12, 10, 0.96); border: 1px solid #44523f;
    border-top: 2px solid #e0a526; box-shadow: 0 10px 32px rgba(0,0,0,0.6); font: 12px/1.45 ui-monospace, 'Cascadia Mono', Consolas, monospace; color: #d6dfcf;
    opacity: 0; visibility: hidden; transform: translateX(12px); transition: opacity 0.1s ease-out, transform 0.1s ease-out, visibility 0s 0.1s; }
  #devcon.open { opacity: 1; visibility: visible; transform: none; transition: opacity 0.1s ease-out, transform 0.1s ease-out; }
  #devcon .sug { display: none; border-top: 1px solid #33402f; background: rgba(0,0,0,0.4); max-height: 11em; overflow-y: auto; }
  #devcon .sug.on { display: block; } #devcon .sug div { padding: 2px 12px; display: flex; gap: 12px; cursor: pointer; }
  #devcon .sug div b { color: #f2d34a; font-weight: 400; min-width: 9ch; } #devcon .sug div span { color: #8a9680; }
  #devcon .sug div em { color: #5d6b58; font-style: normal; margin-left: auto; font-size: 10px; letter-spacing: 0.08em; }
  #devcon .sug div.pick, #devcon .sug div:hover { background: rgba(224,165,38,0.1); }
  #devcon .tips { padding: 4px 12px; color: #66735f; border-top: 1px solid #262f23; font-size: 11px; background: #0d100c; }
  #devcon .bar { display: flex; align-items: center; gap: 8px; padding: 5px 6px 5px 12px; background: #161b14; border-bottom: 1px solid #33402f;
    cursor: move; user-select: none; color: #e3d6aa; letter-spacing: 0.14em; font-weight: 600; }
  #devcon .bar span { flex: 1; } #devcon .bar small { color: #6f7c68; font-weight: 400; letter-spacing: 0.04em; }
  #devcon .bar button { font: inherit; font-weight: 400; color: #b9c7a8; background: none; border: 0; padding: 0 6px; cursor: pointer; letter-spacing: 0.04em; }
  #devcon .bar button:hover, #devcon .bar button:focus-visible { color: #fff; outline: 1px solid #44523f; }
  #devcon .out { flex: 1; overflow-y: auto; padding: 6px 12px; white-space: pre-wrap; scrollbar-width: thin; scrollbar-color: #44523f transparent; }
  #devcon .out > div { display: flex; gap: 10px; } #devcon .out time { color: #4d5a48; flex: none; }
  #devcon .out .warn { color: #ffd479; } #devcon .out .err { color: #ff8a7a; } #devcon .out .cmd { color: #f2d34a; } #devcon .out .ok { color: #9ee08f; }
  #devcon .out .head { color: #e0a526; margin-top: 6px; letter-spacing: 0.1em; }
  #devcon .row { display: flex; align-items: center; border-top: 1px solid #33402f; background: rgba(0,0,0,0.45); }
  #devcon .row b { color: #e0a526; padding-left: 12px; font-weight: 400; }
  #devcon input { flex: 1; font: inherit; color: inherit; background: none; border: 0; padding: 7px 10px; outline: none; }
  @media (prefers-reduced-motion: reduce) { #devcon, #devcon.open { transition: none; } }`;
  document.head.appendChild(css);
  const el = document.createElement('div');
  el.id = 'devcon';
  el.setAttribute('role', 'dialog'); el.setAttribute('aria-label', 'Console');
  el.innerHTML = '<div class="bar"><span>CONSOLE <small>developer</small></span><button data-reset title="Back to the right side">Dock</button><button data-close title="Close (key under Esc)">✕</button></div>'
    + '<div class="out"></div><div class="sug" role="listbox"></div><div class="row"><b>&gt;</b><input spellcheck="false" autocomplete="off" aria-label="Console command" placeholder="Type a command (help lists them all)"></div>'
    + '<div class="tips">Enter runs · Tab completes · ↑ ↓ history · Esc or the key under Esc closes</div>';
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
    // Each line stamped with the clock (game lines and answers alike).
    const t = document.createElement('time');
    t.textContent = new Date().toTimeString().slice(0, 8);
    const body = document.createElement('span');
    body.textContent = text;
    line.append(t, body);
    out.appendChild(line);
    while (out.childNodes.length > 500) out.firstChild.remove();
    out.scrollTop = out.scrollHeight;
  };
  // The game's own log lands here too.
  for (const [k, cls] of [['log', ''], ['info', ''], ['warn', 'warn'], ['error', 'err']]) {
    const orig = console[k].bind(console);
    console[k] = (...a) => { orig(...a); try { const msg = a.map(String).join(' '); if (cls && window.scaliaNote && !/^\[(error|crash)\]/.test(msg)) window.scaliaNote(k, msg); print(a.map((x) => (typeof x === 'string' ? x : x instanceof Error ? x.message : JSON.stringify(x))).join(' '), cls); } catch (_) {} };
  }
  addEventListener('error', (e) => print(`error: ${e.message || e.error || 'Unknown window error'}`, 'err'));
  addEventListener('unhandledrejection', (e) => print(`unhandled rejection: ${e.reason?.stack || e.reason?.message || String(e.reason)}`, 'err'));

  // Commands by group (each command's `group`; the console's own under Console).
  const registry = { ...initialCommands };
  const all = () => ({
    help: { group: 'Console', help: 'help [group]: list commands, all or one group', run: (a) => {
      const by = {};
      for (const [n, c] of Object.entries(all())) (by[c.group || 'Other'] = by[c.group || 'Other'] || []).push([n, c.help]);
      const want = a[0] && Object.keys(by).find((g) => g.toLowerCase() === a[0].toLowerCase());
      for (const [g, list] of Object.entries(by)) {
        if (want && g !== want) continue;
        print(g.toUpperCase(), 'head');
        for (const [n, h] of list) print(`  ${n.padEnd(9)} ${h}`);
      }
    } },
    clear: { group: 'Console', help: 'clear the console (also cls)', run: () => { out.textContent = ''; } },
    cls: { group: 'Console', help: 'clear the console', run: () => { out.textContent = ''; } },
    ...registry });
  const run = (line) => {
    print(`] ${line}`, 'cmd');
    const [name, ...args] = line.trim().split(/\s+/);
    if (!name) return;
    const c = all()[name.toLowerCase()];
    if (!c) { print(`unknown command "${name}" (help lists them)`, 'warn'); return; }
    try { const r = c.run(args, print); if (r !== undefined) print(String(r), 'ok'); } catch (e) { print(String(e), 'err'); }
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
    if (e.key === 'Enter') { const v = input.value; input.value = ''; if (v.trim()) { history.push(v); hpos = history.length; run(v); } suggest(); }
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
  // As you type: the commands that match, with what they do; click one to fill it in.
  const sug = el.querySelector('.sug');
  const suggest = () => {
    const w = input.value.trim().toLowerCase();
    const word = w.split(/\s+/)[0];
    const list = w.includes(' ') ? Object.entries(all()).filter(([n]) => n === word) : Object.entries(all()).filter(([n]) => n.startsWith(word));
    sug.innerHTML = '';
    for (const [n, c] of (w ? list : Object.entries(all()))) {
      const d = document.createElement('div'); d.setAttribute('role', 'option');
      const b = document.createElement('b'); b.textContent = n; const sp = document.createElement('span'); sp.textContent = c.help;
      const em = document.createElement('em'); em.textContent = (c.group || '').toUpperCase();
      d.append(b, sp, em); d.addEventListener('click', () => { input.value = n + ' '; input.focus(); suggest(); });
      sug.appendChild(d);
    }
    sug.classList.toggle('on', sug.childNodes.length > 0 && document.activeElement === input);
  };
  input.addEventListener('input', suggest);
  input.addEventListener('focus', suggest);
  input.addEventListener('blur', () => setTimeout(() => sug.classList.remove('on'), 150));
  print('Scalia console. Start typing to see matching commands, or type help for the full list.');
  print('Groups: ' + [...new Set(Object.values(all()).map((c) => c.group || 'Other'))].join(', ') + '  (help <group> for one)');
  return { print, run, toggle, addCommands(next = {}) { Object.assign(registry, next); print(`Registered ${Object.keys(next).length} game command(s).`, 'ok'); } };
}
