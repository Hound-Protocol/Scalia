import test from 'node:test';
import assert from 'node:assert/strict';
import { CrosshairController } from '../crosshair.js';

const style = () => ({ values: {}, setProperty(key, value) { this.values[key] = value; } });
const element = () => ({ hidden: true, style: style(), attrs: {}, setAttribute(key, value) { this.attrs[key] = value; } });

test('ring follows the character aim point while the dot follows pointer input', () => {
  const canvas = { clientWidth: 1000, clientHeight: 500, style: {} };
  const linePath = element(), line = { ...element(), firstElementChild: linePath };
  const ring = element(), dot = element();
  const crosshair = new CrosshairController({ canvas, line, ring, dot });
  crosshair.update({ state: { line: [0.2, 0.4, 0.7, 0.6], spread: 0.5 }, pointer: { x: 300, y: 220 } });
  assert.equal(ring.style.transform, 'translate(700.0px, 300.0px)');
  assert.equal(dot.style.transform, 'translate(300.0px, 220.0px)');
  assert.equal(ring.style.values['--r'], '15.0px');
  assert.equal(linePath.attrs.x2, '700.0');
  assert.equal(line.hidden, false);
});

test('custom hooks adapt to a changed weapon-state schema', () => {
  const canvas = { clientWidth: 800, clientHeight: 600, style: {} };
  const ring = element(), dot = element();
  const crosshair = new CrosshairController({
    canvas, ring, dot,
    hooks: {
      aimLine: (state) => state.ballisticAxis,
      spread: (state) => state.stabilityRadius,
    },
  });
  crosshair.update({ state: { ballisticAxis: [0, 0, 0.25, 0.75], stabilityRadius: 0.25 }, pointer: { x: 12, y: 24 } });
  assert.equal(ring.style.transform, 'translate(200.0px, 450.0px)');
  assert.equal(ring.style.values['--r'], '11.0px');
});

test('invalid aim data hides both indicators rather than showing stale positions', () => {
  const canvas = { clientWidth: 800, clientHeight: 600, style: {} };
  const ring = element(), dot = element(), line = element();
  const crosshair = new CrosshairController({ canvas, ring, dot, line });
  crosshair.update({ state: { line: null, spread: 0.2 }, pointer: { x: 1, y: 2 } });
  assert.equal(ring.hidden, true);
  assert.equal(dot.hidden, true);
  assert.equal(line.hidden, true);
});
