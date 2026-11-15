import test from 'node:test';
import assert from 'node:assert/strict';
import { AudioEngine } from '../audio.js';

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
