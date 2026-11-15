import test from 'node:test';
import assert from 'node:assert/strict';
import { CrosshairController } from '../crosshair.js';

const style = () => ({ values: {}, setProperty(key, value) { this.values[key] = value; } });
const element = () => ({
  hidden: true, style: style(), attrs: {}, children: [], dataset: {},
  setAttribute(key, value) { this.attrs[key] = value; },
  appendChild(node) { this.children.push(node); },
  removeChild(node) { this.children.splice(this.children.indexOf(node), 1); },
  get firstChild() { return this.children[0] ?? null; },
});
test('keeps a solid centre reference and follows the character muzzle aim point', () => {
  const canvas = { clientWidth: 1000, clientHeight: 500, style: {} };
  const group = element(), line = { hidden: true, querySelector: () => group, ownerDocument: { createElementNS: () => element() } };
  const referenceDot = element(), aimDot = element();
  const crosshair = new CrosshairController({ canvas, line, referenceDot, aimDot });
  crosshair.update({ state: { line: [0.2, 0.4, 0.7, 0.6], recoil: 0.04 } });
  assert.equal(referenceDot.hidden, false);
  assert.equal(aimDot.style.transform, 'translate(700.0px, 300.0px)');
  assert.equal(aimDot.dataset.recoil, '0.04');
  assert.equal(group.children[0].attrs.stroke, '#ffffff');
  assert.equal(group.children[0].attrs['stroke-dasharray'], undefined);
});

test('renders penetration states with the specified line semantics', () => {
  const canvas = { clientWidth: 800, clientHeight: 600, style: {} };
  const group = element(), line = { hidden: true, querySelector: () => group, ownerDocument: { createElementNS: () => element() } };
  const crosshair = new CrosshairController({ canvas, line, referenceDot: element(), aimDot: element() });
  crosshair.update({ state: { trajectory: { segments: [
    { line: [0, 0, 0.25, 0.25], status: 'clear' },
    { line: [0.25, 0.25, 0.5, 0.5], status: 'penetrates' },
    { line: [0.5, 0.5, 0.75, 0.75], status: 'caveat' },
    { line: [0.75, 0.75, 1, 1], status: 'blocked' },
  ] } } });
  assert.deepEqual(group.children.map((node) => node.attrs.stroke), ['#ffffff', '#ffffff', '#ffd43b', '#ff3b30']);
  assert.deepEqual(group.children.map((node) => node.attrs['stroke-dasharray']), [undefined, '3 4', '2 6', '1 6']);
});

test('custom hooks adapt to a changed weapon-state schema', () => {
  const canvas = { clientWidth: 800, clientHeight: 600, style: {} };
  const group = element(), line = { hidden: true, querySelector: () => group, ownerDocument: { createElementNS: () => element() } };
  const crosshair = new CrosshairController({
    canvas, line, referenceDot: element(), aimDot: element(),
    hooks: { trajectory: (state) => state.axis, recoil: (state) => state.kick },
  });
  crosshair.update({ state: { axis: [{ line: [0, 0, 0.25, 0.75], status: 'caveat' }], kick: 0.1 } });
  assert.equal(group.children[0].attrs.stroke, '#ffd43b');
});

test('invalid aim data hides all indicators rather than showing stale positions', () => {
  const canvas = { clientWidth: 800, clientHeight: 600, style: {} };
  const group = element(), line = { hidden: true, querySelector: () => group, ownerDocument: { createElementNS: () => element() } };
  const referenceDot = element(), aimDot = element();
  const crosshair = new CrosshairController({ canvas, line, referenceDot, aimDot });
  crosshair.update({ state: { line: null } });
  assert.equal(referenceDot.hidden, true);
  assert.equal(aimDot.hidden, true);
  assert.equal(line.hidden, true);
});
