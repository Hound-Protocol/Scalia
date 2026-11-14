// Developer console, as in Source games: the key under Esc drops it down from the top. It shows the game's log
// (console output included) and takes commands; up/down walk the history, Tab completes a command.

export function installConsole(commands) {
  const css = document.createElement('style');
  css.textContent = `
  #devcon { position: fixed; left: 0; right: 0; top: 0; height: 42vh; z-index: 1000; display: flex; flex-direction: column;
    background: rgba(12, 14, 12, 0.94); border-bottom: 1px solid #4b5a45; font: 12px/1.4 ui-monospace, monospace; color: #d6dfcf;
    transform: translateY(-100%); transition: transform 0.12s ease-out; }
  #devcon.open { transform: none; }
  #devcon .out { flex: 1; overflow-y: auto; padding: 6px 10px; white-space: pre-wrap; }
  #devcon .out .warn { color: #ffd479; } #devcon .out .err { color: #ff8a7a; } #devcon .out .cmd { color: #9fd0ff; }
  #devcon input { font: inherit; color: inherit; background: rgba(0,0,0,0.4); border: 0; border-top: 1px solid #3c4a38; padding: 6px 10px; outline: none; }
  @media (prefers-reduced-motion: reduce) { #devcon { transition: none; } }`;
  document.head.appendChild(css);
  const el = document.createElement('div');
  el.id = 'devcon';
  el.innerHTML = '<div class="out"></div><input spellcheck="false" autocomplete="off" aria-label="Console command">';
  document.body.appendChild(el);
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
