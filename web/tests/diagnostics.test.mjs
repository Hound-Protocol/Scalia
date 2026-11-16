import test from 'node:test';
import assert from 'node:assert/strict';
import { createDiagnosticSnapshot, readJsHeap, summarizeFrameTimes } from '../diagnostics.js';

test('summarizes valid frame intervals and ignores invalid samples', () => {
  const result = summarizeFrameTimes([16, 17, 20, 10, 0, NaN, Infinity, -1]);
  assert.deepEqual(result, { count: 4, averageMs: 15.75, p95Ms: 20, worstMs: 20 });
});

test('returns null when no valid frame intervals exist', () => {
  assert.equal(summarizeFrameTimes([0, NaN, Infinity, -2]), null);
});

test('reads Chromium heap metrics only when exposed', () => {
  assert.deepEqual(readJsHeap({ memory: {
    usedJSHeapSize: 1024, totalJSHeapSize: 2048, jsHeapSizeLimit: 4096,
  } }), { usedBytes: 1024, totalBytes: 2048, limitBytes: 4096 });
  assert.equal(readJsHeap({}), null);
});

test('creates a timestamped, serializable diagnostic snapshot with explicit unavailable metrics', () => {
  const snapshot = createDiagnosticSnapshot({
    build: 'test', userAgent: 'test-agent', screen: '1920x1080',
    devicePixelRatio: 1, scene: 'ground', tablet: false, gpu: 'WebGPU',
    fps: 60, internalResolution: '640x360', cpuFrameMs: 4,
    frameTimes: { count: 10, averageMs: 16, p95Ms: 20, worstMs: 24 },
    gpuPassMs: { shadow: 1, scene: 2, upscale: 0.5 }, jsHeap: null,
    soldiers: 1000, recentErrors: [], log: '',
  });
  assert.equal(snapshot.schema, 1);
  assert.ok(Number.isFinite(Date.parse(snapshot.capturedAt)));
  assert.deepEqual(snapshot.performance.unavailable,
    ['drawCalls', 'triangles', 'effects', 'lights', 'cpuSubsystemBreakdown']);
  assert.doesNotThrow(() => JSON.stringify(snapshot));
});
