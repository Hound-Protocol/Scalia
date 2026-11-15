// This module is deliberately loaded before game.js. It gives us a working console even if
// asset loading, WebAssembly initialization, or the game module itself fails.
import { installConsole } from './console.js';

const state = { phase: 'Starting bootstrap', done: 0, total: 0, label: '', ok: true, error: null, startedAt: Date.now(), failedAt: null };
const consoleUI = installConsole({
  boot: { group: 'Startup', help: 'show the current startup phase and last checked asset', run: () => JSON.stringify(state, null, 2) },
  reload: { group: 'Startup', help: 'reload Scalia', run: () => { location.reload(); return 'reloading'; } },
  copylog: { group: 'Startup', help: 'copy startup diagnostics and console log', run: () => {
    const report = ['SCALIA STARTUP REPORT — ' + new Date().toISOString(), location.href, navigator.userAgent, JSON.stringify(state, null, 2), window.scaliaLog?.() || '(no persistent game log)'].join('\n');
    navigator.clipboard?.writeText(report).catch(() => {});
    return 'diagnostic report copied (if clipboard permission is available)';
  } },
  gpu: { group: 'Startup', help: 'show browser WebGPU availability', run: async () => {
    const available = !!navigator.gpu;
    if (!available) return 'WebGPU unavailable';
    const adapter = await navigator.gpu.requestAdapter();
    return adapter ? 'WebGPU adapter available: ' + JSON.stringify(adapter.info || {}) : 'WebGPU is present, but no adapter was returned';
  } },
});
window.scaliaConsole = consoleUI;
window.scaliaBoot = {
  update(next) { Object.assign(state, next); },
  fail(error) { state.phase = 'FAILED'; state.ok = false; state.failedAt = Date.now(); state.error = error?.stack || error?.message || String(error); consoleUI.print('STARTUP FAILED: ' + state.error, 'err'); consoleUI.toggle(true); },
  state,
};
window.addEventListener('error', (event) => {
  const message = event.error?.stack || event.message || 'Unknown uncaught error';
  consoleUI.print('UNCAUGHT: ' + message, 'err');
  if (state.phase !== 'Ready') window.scaliaBoot.fail(event.error || message);
}, true);
window.addEventListener('unhandledrejection', (event) => {
  const reason = event.reason?.stack || event.reason?.message || String(event.reason);
  consoleUI.print('UNHANDLED REJECTION: ' + reason, 'err');
  if (state.phase !== 'Ready') window.scaliaBoot.fail(event.reason || reason);
}, true);
consoleUI.print('Early console ready. Startup diagnostics are available before the game engine.', 'ok');
consoleUI.print('Press the backtick key to open. Commands: help, boot, gpu, copylog, reload.');
