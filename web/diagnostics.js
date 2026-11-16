// Small, dependency-free helpers for the in-game diagnostics panel.

export function summarizeFrameTimes(samples) {
  const values = samples.filter((value) => Number.isFinite(value) && value > 0).sort((a, b) => a - b);
  if (!values.length) return null;
  const sum = values.reduce((total, value) => total + value, 0);
  return {
    count: values.length,
    averageMs: sum / values.length,
    p95Ms: values[Math.min(values.length - 1, Math.ceil(values.length * 0.95) - 1)],
    worstMs: values[values.length - 1],
  };
}

export function readJsHeap(perf = globalThis.performance) {
  const memory = perf?.memory;
  if (!memory || !Number.isFinite(memory.usedJSHeapSize)) return null;
  return {
    usedBytes: memory.usedJSHeapSize,
    totalBytes: Number.isFinite(memory.totalJSHeapSize) ? memory.totalJSHeapSize : null,
    limitBytes: Number.isFinite(memory.jsHeapSizeLimit) ? memory.jsHeapSizeLimit : null,
  };
}

export function createDiagnosticSnapshot(data) {
  return {
    schema: 1,
    capturedAt: new Date().toISOString(),
    build: data.build,
    runtime: {
      userAgent: data.userAgent,
      screen: data.screen,
      devicePixelRatio: data.devicePixelRatio,
      scene: data.scene,
      tablet: data.tablet,
      gpu: data.gpu,
    },
    performance: {
      fps: data.fps,
      internalResolution: data.internalResolution,
      cpuFrameMs: data.cpuFrameMs,
      frameTimes: data.frameTimes,
      gpuPassMs: data.gpuPassMs,
      jsHeap: data.jsHeap,
      soldiers: data.soldiers,
      unavailable: ['drawCalls', 'triangles', 'effects', 'lights', 'cpuSubsystemBreakdown'],
    },
    recentErrors: data.recentErrors,
    log: data.log,
  };
}
