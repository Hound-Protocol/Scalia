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

process.exit(failed ? 1 : 0);
