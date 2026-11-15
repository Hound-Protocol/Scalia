import test from 'node:test';
import assert from 'node:assert/strict';
import { AudioEngine, spatialPosition } from '../audio.js';

const connectedElements = new WeakSet();
let sourceCreations = 0;

class FakeNode {
  constructor() {
    this.gain = { value: 1, setTargetAtTime() {} };
    this.frequency = { value: 1, setTargetAtTime() {} };
    this.Q = { value: 0 };
    this.positionX = { value: 0, setTargetAtTime() {} };
    this.positionY = { value: 0, setTargetAtTime() {} };
    this.positionZ = { value: 0, setTargetAtTime() {} };
  }
  connect() { return this; }
}

class FakeAudioContext {
  currentTime = 0;
  createMediaElementSource(element) {
    sourceCreations++;
    if (connectedElements.has(element)) {
      const error = new Error('already connected');
      error.name = 'InvalidStateError';
      throw error;
    }
    connectedElements.add(element);
    return new FakeNode();
  }
  createGain() { return new FakeNode(); }
  createBiquadFilter() { return new FakeNode(); }
  createPanner() { return new FakeNode(); }
}

function engine() {
  const audio = Object.create(AudioEngine.prototype);
  audio.ensure = () => new FakeAudioContext();
  audio.world = new FakeNode();
  audio.rev = { small: new FakeNode(), large: new FakeNode(), corridor: new FakeNode() };
  return audio;
}

test('stream() reuses one source and stream across repeated calls and engine instances', () => {
  const element = {};
  const firstEngine = engine();
  const first = firstEngine.stream(element);
  assert.ok(first);
  assert.equal(firstEngine.stream(element), first);
  assert.equal(engine().stream(element), first);
  assert.equal(sourceCreations, 1);
});

test('stream() contains InvalidStateError when another audio path already owns the element', () => {
  const element = {};
  connectedElements.add(element);
  assert.doesNotThrow(() => assert.equal(engine().stream(element), null));
});

test('jukebox replaces an already-routed media element and obtains a controllable stream', async () => {
  const { createRoutedMediaStream } = await import('../jukebox.js');
  const original = {};
  const replacement = {};
  const calls = [];
  const result = createRoutedMediaStream({
    element: original,
    createElement: () => replacement,
    stream: (element) => {
      calls.push(element);
      return element === replacement ? { volume() {}, tone() {} } : null;
    },
  });
  assert.equal(result.element, replacement);
  assert.ok(result.voice);
  assert.deepEqual(calls, [original, replacement]);
});
test('routeMedia() reuses one media source and preserves its selected bus', () => {
  const before = sourceCreations;
  const element = {};
  const context = new FakeAudioContext();
  const audio = engine();
  audio.ensure = () => context;
  audio.mediaFilter = new FakeNode();
  audio.ui = new FakeNode();

  const first = audio.routeMedia(element, 'world');
  assert.ok(first);
  assert.equal(first.bus, 'world');
  assert.equal(audio.routeMedia(element, 'world'), first);
  assert.equal(audio.routeMedia(element, 'ui'), null);
  const secondEngine = engine();
  let secondEnsures = 0;
  secondEngine.ensure = () => { secondEnsures++; return new FakeAudioContext(); };
  assert.equal(secondEngine.routeMedia(element, 'world'), first);
  assert.equal(secondEnsures, 0);
  assert.equal(sourceCreations, before + 1);
});

test('routeMedia() accepts an analyser node as its destination', () => {
  const before = sourceCreations;
  const element = {};
  const context = new FakeAudioContext();
  const audio = engine();
  const analyser = new FakeNode();
  audio.ensure = () => context;

  const route = audio.routeMedia(element, analyser);
  assert.ok(route);
  assert.equal(route.bus, analyser);
  assert.equal(sourceCreations, before + 1);
});

test('stream() does not claim an element already routed by Auralis media routing', () => {
  const before = sourceCreations;
  const element = {};
  const context = new FakeAudioContext();
  const audio = engine();
  audio.ensure = () => context;
  audio.mediaFilter = new FakeNode();

  assert.ok(audio.routeMedia(element, 'world'));
  assert.equal(audio.stream(element), null);
  assert.equal(sourceCreations, before + 1);
});


test('spatialPosition maps camera-relative directions into HRTF coordinates', () => {
  const near = (actual, expected) => assert.ok(Math.abs(actual - expected) < 1e-6, `${actual} != ${expected}`);
  const right = spatialPosition(1, 0);
  near(right[0], 1); near(right[1], 0); near(right[2], 0);
  const ahead = spatialPosition(0, 1);
  near(ahead[0], 0); near(ahead[1], 0); near(ahead[2], -1);
  const behind = spatialPosition(0, -1);
  near(behind[0], 0); near(behind[1], 0); near(behind[2], 1);
  const below = spatialPosition(0, 0, 2);
  near(below[0], 0); near(below[1], -1); near(below[2], 0);
  assert.deepEqual(spatialPosition(0, 0), [0, 0, -1]);
});

test('spatialPosition normalises diagonal vectors and contains invalid inputs', () => {
  const diagonal = spatialPosition(1, 1, 0);
  near(Math.hypot(...diagonal), 1);
  const invalid = spatialPosition(Number.NaN, Infinity, Number.NaN);
  assert.deepEqual(invalid, [0, 0, -1]);
});
