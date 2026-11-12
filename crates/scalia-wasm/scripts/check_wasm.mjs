// Native-versus-WebAssembly check: the same scenario must hash identically in both builds.
import { createRequire } from 'node:module';
import { execSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, '../../..');
const require = createRequire(import.meta.url);
const wasm = require(path.join(here, '../pkg/scalia_wasm.js'));

let failed = false;
const check = (name, ok, detail = '') => {
  console.log(`${ok ? 'OK' : 'FAIL'} ${name}${detail ? ` (${detail})` : ''}`);
  if (!ok) failed = true;
};

// (a) the hash from WebAssembly equals the hash from the native binary.
const native = BigInt(
  execSync('cargo run -q --release -p scalia-bench --bin hash_scenario -- 42', { cwd: root }).toString().trim(),
);
const fromWasm = wasm.scenario_hash(42n);
check('native and wasm scenario hash match', native === fromWasm, `native=${native} wasm=${fromWasm}`);

// (b) a ticked Sim derives a sane, finite set of individuals.
const sim = new wasm.Sim(42n, 200, 150);
for (let s = 1; s <= 10; s++) sim.tick(s * 1000);
const cap = 5000;
const n = sim.derive(10_000, 1, -1e6, 6e6, -1e6, 6e6, cap);
const xs = sim.xs();
const finite = [xs, sim.ys(), sim.zs()].every((a) => a.length === n && a.every(Number.isFinite));
check('derive count in range and buffers finite', n >= 1 && n <= cap && finite && sim.phases().length === n, `count=${n}`);

// (c) a view in another region sees nobody.
check('other region returns zero', sim.derive(10_000, 2, -1e6, 6e6, -1e6, 6e6, cap) === 0);

// (d) blueprints: garbage is an error, not a trap; shipped files draw and report.
let threw = false;
try { wasm.blueprint_svg('garbage'); } catch (e) { threw = !String(e).includes('not a function'); }
check('malformed blueprint throws instead of trapping', threw);
const ox = wasm.shipped_ron(5);
check('shipped_ron(5) returns text, shipped_ron(99) is undefined', typeof ox === 'string' && wasm.shipped_ron(99) === undefined);
check('blueprint_svg draws a shipped blueprint', wasm.blueprint_svg(ox).startsWith('<svg'));
const info = JSON.parse(wasm.blueprint_info(ox));
check('blueprint_info matches the native golden mesh hash', info.id === 'transport_ox' && info.mesh_hash === '10854958835410261074' && info.triangles === 148, JSON.stringify(info));

// (e) planet: chunk size and the native golden chunk hash.
check('planet_chunk(1,0,0,8) has 81 samples', wasm.planet_chunk(1n, 0, 0, 8).length === 81);
check('planet chunk hash equals the native golden', wasm.planet_chunk_hash(7n, 3, 1000, 16) === 9283842558404932274n);

// (f) absurd sizes are an error to JS and leave the module usable.
let bad = false;
try { new wasm.Sim(1n, 1, 4_000_000_000); } catch (e) { bad = !String(e).includes('unreachable'); }
check('absurd group size throws a readable error', bad);
const again = new wasm.Sim(42n, 10, 10);
check('module still works after the error', again.derive(0, 1, -1e6, 6e6, -1e6, 6e6, 100) >= 0);

// (g) orbits and flight match the native core.
check('two bodies named Aster and Brine', wasm.system_body_count() === 2 && wasm.system_body_name(0) === 'Aster' && wasm.system_body_name(1) === 'Brine');
check('body 0 starts on the x axis', JSON.stringify(Array.from(wasm.system_body_pos(0, 0))) === '[40000000000,0]');
check('body position equals the native value', JSON.stringify(Array.from(wasm.system_body_pos(0, 123456))) === '[39998382022,359114063]');
check('unknown body has no position', wasm.system_body_pos(5, 0).length === 0 && wasm.system_body_name(5) === undefined);
const fl = new wasm.Flight(0, 1, 1000, 2e9);
const dur = fl.duration_ms();
check('flight takes a sensible time', dur >= 10_000 && dur <= 600_000, `dur=${dur}`);
check('flight starts at its origin and ends at its target', JSON.stringify(Array.from(fl.pos(1000))) === JSON.stringify(Array.from(fl.from())) && JSON.stringify(Array.from(fl.pos(1000 + dur))) === JSON.stringify(Array.from(fl.to())));
const tgt = Array.from(wasm.system_body_pos(1, 1000 + dur));
const to = Array.from(fl.to());
check('flight ends where the body is, at the parking offset', to[0] === tgt[0] && to[1] === tgt[1] + 50_000_000, `to=${to} body=${tgt}`);
let e1 = false, e2 = false;
try { new wasm.Flight(0, 9, 0, 2e9); } catch (e) { e1 = String(e).length > 0 && !String(e).includes('unreachable'); }
try { new wasm.Flight(0, 1, 0, 0); } catch (e) { e2 = String(e).length > 0 && !String(e).includes('unreachable'); }
check('bad flights throw readable errors', e1 && e2);

// (h) mesh buffer for drawing: 148 triangles, 4 floats per vertex, finite.
const mb = wasm.blueprint_mesh(ox);
check('blueprint_mesh has 148 triangles of finite vertices', mb.length === 148 * 3 * 4 && mb.every(Number.isFinite), `len=${mb.length}`);

process.exit(failed ? 1 : 0);
