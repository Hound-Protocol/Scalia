import test from 'node:test';
import assert from 'node:assert/strict';
import { AudioEngine, spatialPosition } from '../audio.js';

const connectedElements = new WeakSet();
const near = (actual, expected) => assert.ok(Math.abs(actual - expected) < 1e-6, `${actual} != ${expected}`);
let sourceCreations = 0;

class FakeNode {
  constructor() {
    const param = () => ({ value: 1, setTargetAtTime(value) { this.value = value; }, cancelScheduledValues() {}, setValueAtTime(value) { this.value = value; }, linearRampToValueAtTime(value) { this.value = value; } });
    this.gain = param();
    this.frequency = param();
    this.playbackRate = param();
    this.pan = param();
    this.Q = { value: 0 };
    this.positionX = { value: 0, setTargetAtTime() {} };
    this.positionY = { value: 0, setTargetAtTime() {} };
    this.positionZ = { value: 0, setTargetAtTime() {} };
  }
  connect(destination) { this.connections ??= []; this.connections.push(destination); return destination; }
  disconnect() { this.disconnected = true; }
}

class FakeAudioContext {
  currentTime = 0;
  constructor() { this.createdNodes = []; }
  track(node) { this.createdNodes.push(node); return node; }
  createMediaElementSource(element) {
    sourceCreations++;
    if (connectedElements.has(element)) {
      const error = new Error('already connected');
      error.name = 'InvalidStateError';
      throw error;
    }
    connectedElements.add(element);
    return this.track(new FakeNode());
  }
  createGain() { return this.track(new FakeNode()); }
  createBiquadFilter() { return this.track(new FakeNode()); }
  createPanner() { return this.track(new FakeNode()); }
  createStereoPanner() { return this.track(new FakeNode()); }
  createWaveShaper() { return this.track(new FakeNode()); }
  createDelay() { return this.track(new FakeNode()); }
  createBufferSource() {
    const source = this.track(new FakeNode());
    source.start = (...args) => { source.startArgs = args; };
    source.stop = (when) => { source.stopAt = when; source.onended?.(); };
    return source;
  }
}



function playbackEngine() {
  const audio = new AudioEngine();
  const context = new FakeAudioContext();
  audio.ctx = context;
  audio.ensure = () => context;
  audio.buffer = () => ({ duration: 0.2 });
  audio.world = new FakeNode();
  audio.mix = new FakeNode();
  audio.ui = new FakeNode();
  audio.air = new FakeNode();
  audio.rev = { small: new FakeNode(), large: new FakeNode(), corridor: new FakeNode() };
  audio.crunch = new Float32Array(8);
  audio.heat = 0;
  audio.heatAt = 0;
  audio.muffle = null;
  audio.ringGain = null;
  return audio;
}

function lifecycleEngine() {
  const audio = Object.create(AudioEngine.prototype);
  audio.active = [];
  audio.voices = new Set();
  audio.tails = [];
  audio.voiceSequence = 0;
  audio.voiceMetrics = { dropped: 0, stolen: 0, completed: 0, cleaned: 0 };
  return audio;
}

function addVoice(audio, priority = 1, gain = 0.1, tailSeconds = 0) {
  const source = {
    onended: null,
    stopped: false,
    stop() { this.stopped = true; this.onended?.(); },
  };
  const node = new FakeNode();
  const voice = audio._registerVoice(source, [source, node], {
    category: 'test', priority, gain, tailSeconds,
  });
  voice.out = new FakeNode();
  voice.state = 'playing';
  return { voice, source, node };
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
  const right = spatialPosition(1, 0);
  near(right[0], 1); near(right[1], 0); near(right[2], 0);
  const left = spatialPosition(-1, 0);
  near(left[0], -1); near(left[1], 0); near(left[2], 0);
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


test('voice handles stop and clean up their owned graph exactly once', () => {
  const audio = lifecycleEngine();
  const { voice, source, node } = addVoice(audio, 2, 0.4);
  assert.equal(audio.active.length, 1);
  assert.equal(audio.voices.size, 1);
  assert.equal(voice.handle.stop(), true);
  assert.equal(source.stopped, true);
  assert.equal(voice.handle.state, 'ended');
  assert.equal(node.disconnected, true);
  assert.deepEqual(audio.voiceStats(), {
    active: 0, tails: 0, tracked: 0, nodes: 0,
    dropped: 0, stolen: 0, completed: 1, cleaned: 1,
  });
  assert.equal(voice.handle.stop(), false);
});

test('voice allocator deterministically steals a lower-priority voice at the 64 voice cap', () => {
  const audio = lifecycleEngine();
  const voices = Array.from({ length: 64 }, () => addVoice(audio, 1, 0.1));
  assert.equal(audio.active.length, 64);
  assert.equal(audio._admitVoice(2, 0.05), true);
  assert.equal(voices[0].source.stopped, true);
  assert.equal(voices[0].voice.handle.state, 'ended');
  assert.equal(audio.active.length, 63);
  assert.equal(audio.voiceStats().stolen, 1);
  audio.stopAll();
  assert.equal(audio.voiceStats().tracked, 0);
});

test('voice allocator drops a lower-priority sound when all 64 voices are more important', () => {
  const audio = lifecycleEngine();
  const voices = Array.from({ length: 64 }, () => addVoice(audio, 3, 0.5));
  assert.equal(audio._admitVoice(1, 1), false);
  assert.equal(audio.active.length, 64);
  assert.equal(voices.every(({ source }) => !source.stopped), true);
  assert.equal(audio.voiceStats().dropped, 1);
  audio.stopAll();
});

test('completed voice tails are bounded and retired nodes are disconnected', () => {
  const audio = lifecycleEngine();
  const voices = Array.from({ length: 65 }, () => addVoice(audio, 1, 0.1, 0.5));
  for (const { source } of voices) source.onended();
  assert.equal(audio.tails.length, 64);
  assert.equal(voices[0].voice.handle.state, 'ended');
  assert.equal(voices[0].node.disconnected, true);
  assert.equal(audio.voiceStats().nodes, 64 * 2);
  audio.stopAll();
  assert.equal(audio.voiceStats().tracked, 0);
  assert.equal(audio.voiceStats().nodes, 0);
});


test('play() returns a cancellable handle and disconnects every voice-owned node', () => {
  const audio = playbackEngine();
  const handle = audio.play('test-shot', {
    kind: 'shot', db: 100, gain: 1, room: 0, rx: 0, ry: 1,
    below: 0, dist: 1, lowpass: 20000, space: [0, 30, 0], echoes: [],
  });
  assert.ok(handle);
  assert.equal(handle.state, 'playing');
  assert.equal(audio.voiceStats().active, 1);
  assert.ok(audio.voiceStats().nodes >= 10);
  assert.equal(handle.cancel(), true);
  assert.equal(handle.state, 'ended');
  assert.equal(audio.voiceStats().tracked, 0);
  assert.equal(audio.voiceStats().nodes, 0);
  assert.equal(audio.ctx.createdNodes.every((node) => node.disconnected), true);
});

test('playUi() participates in the same voice cap and cancellation lifecycle', () => {
  const audio = playbackEngine();
  const handle = audio.playUi('ui-click', 0.8);
  assert.ok(handle);
  assert.equal(handle.category, 'ui');
  assert.equal(audio.voiceStats().active, 1);
  assert.equal(handle.stop(), true);
  assert.equal(audio.voiceStats().active, 0);
  assert.equal(audio.voiceStats().nodes, 0);
});


test('reverb, air and echo branches retain then disconnect every auxiliary node', () => {
  const audio = playbackEngine();
  const handle = audio.play('dense-blast', {
    kind: 'blast', db: 150, gain: 1, room: 1, rx: 0, ry: 1,
    below: 0, dist: 1, far: 0.5, lowpass: 20000,
    space: [0.5, 30, 0], echoes: [[0.1, 0.5, 0]],
  });
  assert.ok(handle);
  const source = [...audio.voices][0].src;
  source.onended();
  assert.equal(handle.state, 'tailing');
  assert.equal(audio.voiceStats().tails, 1);
  assert.ok(audio.voiceStats().nodes > 20);
  audio.stopAll();
  assert.equal(handle.state, 'ended');
  assert.equal(audio.voiceStats().nodes, 0);
  assert.equal(audio.ctx.createdNodes.every((node) => node.disconnected), true);
});


test('playBuffer() manages looping menu voices and supports graceful fade cancellation', () => {
  const audio = playbackEngine();
  const handle = audio.playBuffer({ duration: 4 }, {
    gain: 0.6, category: 'ui', priority: 2, loop: true,
    loopStart: 0.2, loopEnd: 3.5, fadeIn: 0.4,
  });
  assert.ok(handle);
  assert.equal(handle.state, 'playing');
  assert.equal(handle.category, 'ui');
  assert.equal(handle.src.loop, true);
  assert.deepEqual(handle.src.startArgs, [0, 0.2]);
  assert.equal(handle.gain.gain.value, 0.6);
  assert.equal(handle.stop(0.05), true);
  assert.equal(handle.src.stopAt, 0.07);
  assert.equal(handle.state, 'ended');
  assert.equal(audio.voiceStats().nodes, 0);
});
