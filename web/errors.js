// Error handling: one failing part of the game never freezes the rest. Every part of a frame runs guarded; an
// error is logged (once per kind, not every frame), a small notice points at the console, and the last errors
// are kept for a bug report. If the engine itself has crashed (a Rust panic: the WebAssembly module is dead),
// a crash screen says so and offers a report and a reload, instead of a frozen picture.

export function createErrors(version) {
  const recent = [];
  const seen = new Map();
  let crashed = false, noticeEl = null, noticeTimer = 0;

  const notice = (text) => {
    if (!noticeEl) {
      noticeEl = document.createElement('div');
      noticeEl.style.cssText = 'position:fixed;left:50%;bottom:64px;transform:translateX(-50%);z-index:900;padding:6px 12px;border-radius:4px;background:rgba(60,14,10,0.92);color:#ffd9d2;font:12px ui-monospace,monospace;pointer-events:none';
      document.body.appendChild(noticeEl);
    }
    noticeEl.textContent = text;
    noticeEl.hidden = false;
    clearTimeout(noticeTimer);
    noticeTimer = setTimeout(() => { noticeEl.hidden = true; }, 4000);
  };

  const describe = (e) => (e && e.stack ? String(e.stack).split('\n').slice(0, 4).join('\n') : String(e));

  const crash = (where, e) => {
    if (crashed) return;
    crashed = true;
    recent.push({ at: new Date().toISOString(), where, error: describe(e), fatal: true });
    console.error(`[crash] ${where}: ${e && e.message ? e.message : e}`);
    const el = document.createElement('div');
    el.style.cssText = 'position:fixed;inset:0;z-index:2000;display:flex;align-items:center;justify-content:center;background:rgba(8,10,8,0.9);color:#e8eee0;font:14px/1.5 ui-monospace,monospace';
    el.innerHTML = `<div style="max-width:560px;padding:24px;border:1px solid #6b2a20;background:#140c0a">
      <div style="color:#ff9b8a;font-weight:600;margin-bottom:8px">The game engine stopped</div>
      <div style="margin-bottom:12px">Something went wrong inside the engine (${where}). Copy the report and send it over, then reload.</div>
      <pre style="white-space:pre-wrap;font-size:11px;max-height:160px;overflow:auto;background:#0a0606;padding:8px">${describe(e).replace(/</g, '&lt;')}</pre>
      <button data-copy style="margin-right:8px">Copy report</button><button data-reload>Reload</button></div>`;
    el.querySelector('[data-copy]').onclick = () => navigator.clipboard.writeText(report()).then(() => { el.querySelector('[data-copy]').textContent = 'Copied'; }).catch(() => {});
    el.querySelector('[data-reload]').onclick = () => location.reload();
    document.body.appendChild(el);
  };

  const handle = (where, e) => {
    // A dead WebAssembly module (panic, out of memory) cannot recover: crash screen.
    if (e instanceof WebAssembly.RuntimeError || /unreachable|memory access out of bounds/i.test(String(e && e.message))) {
      crash(where, e);
      return;
    }
    const key = `${where}: ${e && e.message ? e.message : e}`;
    const now = performance.now();
    if ((seen.get(key) || -1e9) > now - 10_000) return;
    seen.set(key, now);
    recent.push({ at: new Date().toISOString(), where, error: describe(e), fatal: false });
    while (recent.length > 30) recent.shift();
    console.warn(`[error] ${key}`);
    notice('Error, press the key under Esc for details');
  };

  /// Runs one part of the game; an error in it is handled, never thrown on.
  const guard = (where, fn) => {
    if (crashed) return undefined;
    try { return fn(); } catch (e) { handle(where, e); return undefined; }
  };

  const report = () => {
    let gpu = '';
    try { gpu = navigator.gpu ? 'WebGPU' : 'no WebGPU'; } catch (_) {}
    return [`Scalia ${version} ${location.href}`, `${navigator.userAgent}`, gpu, `screen ${innerWidth}x${innerHeight} @${devicePixelRatio}`, '',
      ...recent.map((r) => `${r.at} ${r.fatal ? 'CRASH ' : ''}${r.where}\n${r.error}`)].join('\n');
  };

  addEventListener('error', (e) => handle('uncaught', e.error || e.message));
  addEventListener('unhandledrejection', (e) => handle('promise', e.reason));
  return { guard, handle, report, recent, get crashed() { return crashed; } };
}
