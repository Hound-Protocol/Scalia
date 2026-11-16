// Auralis Audio Engine — browser-side Web Audio renderer for Pack Engine acoustics. The acoustics (Rust) work out how each sound reaches the listener
// (gain, low-pass, how much is heard as reverb, where it is around his head) and what space he is in; this
// graph applies it:
//
//   voice: source -> gain -> high-pass -> low-pass -+-> HRTF panner -+-> dry -> mix
//                                                   |                +-> the reverb of the room it is made in
//                                                   |                    (small room, large room, corridor)
//                                                   +-> echoes off what stands around it (map geometry):
//                                                       delay -> gain -> darker -> from the wall's side -> mix
//   In the open with nothing near, a shot is just the crack: nothing to bounce off, no echo.
//   mix -> tablet band-pass (200 Hz - 10 kHz while the PDA is up, open otherwise) -> limiter -> speakers
//
// Built for headphones (HRTF). Impulse responses are generated here, nothing to download.

const VOICES = 64;
const MAX_TAILS = 64;
// HDR audio (as Frostbite): every sound has a logical loudness (dB at the source); what reaches the ear is that
// plus the acoustics' gain. The loudest thing heard lately sets a window: anything more than WINDOW_DB below it
// is not played at all, anything well below it is pushed down further, and the peak relaxes over seconds.
// Presets as in Battlefield: HiFi keeps a wide range (60 dB), War Tapes squeezes it (25 dB: punchy, loud).
const HDR = { hifi: 60, wartapes: 25 };
const QUIET_DB = 20, RELEASE_DB_S = 14, PEAK_FLOOR_DB = 40;
const PRIORITY = { blast: 4, shot: 3, ui: 2, radio: 2, step: 1 };

// Convert the camera-relative ground vector into the listener's HRTF coordinate frame.
// +x is right, -z is ahead, +z is behind, and -y is below the listener.
export function spatialPosition(rx, ry, below = 0) {
  const x = Number.isFinite(rx) ? rx : 0;
  const y = Number.isFinite(ry) ? ry : 0;
  const by = (Number.isFinite(below) ? Math.max(0, below) : 0) * 0.35;
  const len = Math.hypot(x, y, by);
  return len < 0.3 ? [0, 0, -1] : [x / len, -by / len, -y / len];
}

// A media element may have exactly one MediaElementAudioSourceNode for its lifetime.
// Keep this module-wide so separate AudioEngine instances cannot reconnect the same element.
const MEDIA_ELEMENT_STREAMS = new WeakMap();
const MEDIA_ELEMENT_ROUTES = new WeakMap();

/// A stereo impulse response: decaying noise (darker as it decays), plus early reflections.
function impulse(ctx, seconds, rt60, early, damp) {
  const rate = ctx.sampleRate, n = Math.floor(rate * seconds);
  const buf = ctx.createBuffer(2, n, rate);
  for (let ch = 0; ch < 2; ch++) {
    const d = buf.getChannelData(ch);
    let lp = 0;
    for (let i = 0; i < n; i++) {
      const t = i / rate;
      // -60 dB at rt60; a one-pole low-pass closing over time (walls and air eat the highs first).
      const env = Math.exp(-6.9 * t / rt60);
      const k = Math.min(0.95, damp * (1 + t * 4));
      lp = lp * k + (Math.random() * 2 - 1) * (1 - k);
      d[i] = lp * env * (t < 0.004 ? t / 0.004 : 1);
    }
    for (const [at, g] of early) {
      const i = Math.floor((at + (ch ? 0.0013 : 0)) * rate);
      if (i < n) d[i] += g * (ch ? 0.9 : 1);
    }
  }
  return buf;
}

/// The open air: a loud sound outdoors does not stop dead, it rolls away in a soft, gentle echo (scattered off
/// the ground and into the distance). No room in it: a late, smooth start, a few soft swells, a long fade, and
/// neither boom nor hiss.
function airTail(ctx) {
  const rate = ctx.sampleRate, n = Math.floor(rate * 2.0);
  const buf = ctx.createBuffer(2, n, rate);
  const swells = [[0.28, 0.55], [0.62, 0.35], [1.05, 0.2]];
  for (let ch = 0; ch < 2; ch++) {
    const d = buf.getChannelData(ch);
    let lo = 0, lo2 = 0;
    for (let i = 0; i < n; i++) {
      const t = i / rate;
      // Band-limited noise (roughly 300 Hz - 3 kHz): two one-pole low-passes, minus a slower one for the lows.
      const x = Math.random() * 2 - 1;
      lo = lo * 0.82 + x * 0.18;
      lo2 = lo2 * 0.985 + lo * 0.015;
      const band = lo - lo2;
      const onset = t < 0.04 ? 0 : Math.min(1, (t - 0.04) / 0.08);
      let env = Math.exp(-t / 0.5);
      for (const [at, g] of swells) env += g * Math.exp(-((t - at) * (t - at)) / 0.006) * (ch ? 0.85 : 1);
      d[i] = band * env * onset * 0.9;
    }
  }
  return buf;
}

export class AudioEngine {
  constructor() {
    this.ctx = null;
    this.buffers = new Map();
    this.bufferLoads = new Map();
    this.active = [];
    this.voices = new Set();
    this.tails = [];
    this.voiceSequence = 0;
    this.voiceMetrics = { dropped: 0, stolen: 0, completed: 0, cleaned: 0 };
    this.env = { enclosure: 0, size: 30, rt60: 0.25, corridor: 0, pda: 0 };
    this.peak = PEAK_FLOOR_DB; this.peakAt = 0;
    this.hdr = 'wartapes'; this.windowDb = HDR.wartapes;
    // AI Lab world voices share the simulation clock; UI and ambience remain real-time.
    this.worldTimeScale = 1;
  }

  _pruneActive() {
    this.active = this.active.filter((voice) => !voice.done && voice.state !== 'stopping');
  }

  _admitVoice(priority, gain) {
    this._pruneActive();
    if (this.active.length < VOICES) return true;
    const worst = this.active.reduce((a, b) => {
      if (a.pri !== b.pri) return a.pri < b.pri ? a : b;
      if (a.gain !== b.gain) return a.gain < b.gain ? a : b;
      return a.id < b.id ? a : b;
    });
    if (worst.pri > priority || (worst.pri === priority && worst.gain >= gain)) {
      this.voiceMetrics.dropped++;
      return false;
    }
    this.voiceMetrics.stolen++;
    this._stopVoice(worst, 'stolen');
    return true;
  }

  _registerVoice(src, nodes, { category = 'world', priority = 1, gain = 1, tailSeconds = 0, out = null, onended = null } = {}) {
    const voice = {
      id: ++this.voiceSequence, src, nodes, category, pri: priority, gain,
      tailSeconds: Math.max(0, Math.min(4.2, tailSeconds)), state: 'scheduled', done: false,
      timer: null, reason: null, out,
    };
    const engine = this;
    voice.handle = {
      id: voice.id,
      category,
      src,
      gain: out,
      get state() { return voice.state; },
      stop(fadeSeconds = 0) { return engine._stopVoice(voice, 'cancelled', fadeSeconds); },
      cancel() { return engine._stopVoice(voice, 'cancelled'); },
    };
    this.voices.add(voice);
    this.active.push(voice);
    src.onended = () => { this._finishVoice(voice); onended?.(voice.handle); };
    return voice;
  }

  _stopVoice(voice, reason = 'stopped', fadeSeconds = 0) {
    if (!voice || voice.done || voice.state === 'stopping' || voice.state === 'tailing' || voice.state === 'ended') return false;
    voice.state = 'stopping';
    voice.reason = reason;
    voice.done = true;
    this.active = this.active.filter((item) => item !== voice);
    const fade = Number.isFinite(fadeSeconds) ? Math.max(0, Math.min(10, fadeSeconds)) : 0;
    try {
      if (fade > 0 && voice.out?.gain && this.ctx) {
        const t = this.ctx.currentTime;
        voice.out.gain.cancelScheduledValues(t);
        voice.out.gain.setValueAtTime(voice.out.gain.value, t);
        voice.out.gain.linearRampToValueAtTime(0, t + fade);
        voice.src.stop(t + fade + 0.02);
      } else voice.src.stop();
    } catch (_) { this._finishVoice(voice); }
    return true;
  }

  _finishVoice(voice) {
    if (!voice || voice.state === 'tailing' || voice.state === 'ended') return;
    voice.done = true;
    this.active = this.active.filter((item) => item !== voice);
    this.voiceMetrics.completed++;
    if (voice.tailSeconds <= 0) {
      this._cleanupVoice(voice);
      return;
    }
    voice.state = 'tailing';
    this.tails.push(voice);
    while (this.tails.length > MAX_TAILS) this._cleanupVoice(this.tails[0]);
    voice.timer = setTimeout(() => this._cleanupVoice(voice), voice.tailSeconds * 1000);
  }

  _cleanupVoice(voice) {
    if (!voice || voice.state === 'ended') return;
    if (voice.timer !== null) clearTimeout(voice.timer);
    voice.timer = null;
    for (const node of voice.nodes) {
      try { node.disconnect(); } catch (_) { /* already disconnected */ }
    }
    voice.state = 'ended';
    voice.done = true;
    this.active = this.active.filter((item) => item !== voice);
    this.tails = this.tails.filter((item) => item !== voice);
    this.voices.delete(voice);
    this.voiceMetrics.cleaned++;
  }

  setWorldTimeScale(scale) {
    const next = Number.isFinite(scale) ? Math.max(0, Math.min(4, scale)) : 1;
    this.worldTimeScale = next;
    const now = this.ctx?.currentTime;
    if (now === undefined) return;
    for (const voice of this.voices) {
      if (voice.timeScaled && !voice.done) {
        voice.src.playbackRate.setValueAtTime(voice.baseRate * next, now);
      }
    }
  }

  voiceStats() {
    let nodeCount = 0;
    for (const voice of this.voices) nodeCount += voice.nodes.length;
    return {
      active: this.active.length,
      tails: this.tails.length,
      tracked: this.voices.size,
      nodes: nodeCount,
      ...this.voiceMetrics,
    };
  }

  stopAll() {
    for (const voice of [...this.voices]) {
      if (!voice.done) this._stopVoice(voice, 'shutdown');
      this._cleanupVoice(voice);
    }
  }

  /// HDR preset: 'hifi' (60 dB window) or 'wartapes' (25 dB).
  setHdr(name) {
    if (!(name in HDR)) return false;
    this.hdr = name; this.windowDb = HDR[name];
    return true;
  }

  /// The HDR peak now: the loudest sound lately, relaxing at RELEASE_DB_S.
  peakNow() {
    const t = this.ctx ? this.ctx.currentTime : 0;
    return Math.max(PEAK_FLOOR_DB, this.peak - RELEASE_DB_S * Math.max(0, t - this.peakAt));
  }

  /// Waking up: everything in the world heard through a low-pass at `hz` (22000 or more: open).
  setWake(hz) {
    this.wakeHz = Math.min(22000, Math.max(200, hz));
    if (this.wakeLp) this.wakeLp.frequency.value = this.wakeHz;
    if (this.mediaWakeLp) this.mediaWakeLp.frequency.value = this.wakeHz;
  }
  /// Applies tablet attention muffling to music and ambience only.
  setMediaMuffle(on) {
    const c = this.ctx; if (!c || !this.mediaFilter || !this.mediaGain) return;
    const t = c.currentTime;
    this.mediaFilter.frequency.cancelScheduledValues(t);
    this.mediaGain.gain.cancelScheduledValues(t);
    this.mediaFilter.frequency.setTargetAtTime(on ? 600 : 20000, t, 0.12);
    this.mediaGain.gain.setTargetAtTime(on ? 0.55 : 1, t, 0.12);
  }

  /// Fade only the persistent media bus (used by the controlled session reboot).
  fadeMediaOut(seconds = 0.6) {
    const c = this.ctx; if (!c || !this.mediaGain) return;
    const t = c.currentTime;
    this.mediaGain.gain.cancelScheduledValues(t);
    this.mediaGain.gain.setTargetAtTime(0, t, Math.max(0.01, seconds / 3));
  }

  /// Route a persistent HTML media element through Auralis. A media element gets one source node for life.
  routeMedia(el, bus = 'ui') {
    if (!el || (typeof el !== 'object' && typeof el !== 'function')) return null;
    const existing = MEDIA_ELEMENT_ROUTES.get(el);
    if (existing) return existing.bus === bus ? existing : null;
    if (MEDIA_ELEMENT_STREAMS.has(el)) return null;
    const c = this.ensure(); if (!c) return null;
    const destination = bus === 'world' ? this.mediaFilter : bus === 'ui' ? this.ui : (bus && typeof bus.connect === 'function' ? bus : null);
    if (!destination) return null;
    let source;
    try { source = c.createMediaElementSource(el); }
    catch (error) {
      if (error?.name === 'InvalidStateError') {
        console.warn('[Auralis] Media element already has a source node; duplicate routing skipped.', error);
        return null;
      }
      throw error;
    }
    source.connect(destination);
    const route = { context: c, source, bus };
    MEDIA_ELEMENT_ROUTES.set(el, route);
    return route;
  }

  /// Play a decoded buffer through an Auralis bus with the same bounded lifecycle as world one-shots.
  playBuffer(buffer, { gain = 1, category = 'ui', priority = PRIORITY[category] || 1, via = null,
    loop = false, loopStart = 0, loopEnd = 0, fadeIn = 0, onended = null } = {}) {
    const c = this.ensure(); if (!c || !buffer) return null;
    const safeGain = Number.isFinite(gain) ? Math.max(0, Math.min(4, gain)) : 1;
    if (!this._admitVoice(priority, safeGain)) return null;
    const source = c.createBufferSource();
    source.buffer = buffer;
    source.loop = Boolean(loop);
    if (source.loop) {
      source.loopStart = Math.max(0, Number.isFinite(loopStart) ? loopStart : 0);
      source.loopEnd = Math.max(source.loopStart, Number.isFinite(loopEnd) ? loopEnd : 0);
    }
    const level = c.createGain();
    const now = c.currentTime;
    const fade = Number.isFinite(fadeIn) ? Math.max(0, Math.min(10, fadeIn)) : 0;
    level.gain.setValueAtTime(fade > 0 ? 0 : safeGain, now);
    if (fade > 0) level.gain.linearRampToValueAtTime(safeGain, now + fade);
    const destination = via && typeof via.connect === 'function' ? via : this.ui;
    source.connect(level).connect(destination);
    const voice = this._registerVoice(source, [source, level], {
      category, priority, gain: safeGain, tailSeconds: 0, out: level, onended,
    });
    try {
      source.start(now, source.loop ? source.loopStart : 0);
      voice.state = 'playing';
      return voice.handle;
    } catch (error) {
      this._cleanupVoice(voice);
      throw error;
    }
  }

  /// Play a decoded, non-spatial interface sound through the Auralis master limiter.
  playUi(url, gain = 1) {
    const c = this.ensure(); if (!c) return null;
    const buffer = this.buffer(url);
    if (!buffer) {
      const pending = this.bufferLoads.get(url);
      if (pending) pending.then(() => this.playUi(url, gain)).catch(() => {});
      return null;
    }
    return this.playBuffer(buffer, { gain, category: 'ui', priority: PRIORITY.ui, via: this.ui });
  }

  /// Creates the graph on first use (browsers allow audio only after a user gesture).
  ensure() {
    if (this.ctx) {
      if (this.ctx.state === 'suspended') this.ctx.resume().catch(() => {});
      return this.ctx;
    }
    let c;
    try { c = new AudioContext({ latencyHint: 'interactive' }); } catch (_) { return null; }
    this.ctx = c;
    // Master limiter: a firefight never clips.
    const lim = c.createDynamicsCompressor();
    lim.threshold.value = -4; lim.knee.value = 0; lim.ratio.value = 20; lim.attack.value = 0.002; lim.release.value = 0.18;
    lim.connect(c.destination);
    // Tablet band-pass (500 Hz - 10 kHz while it is up): four high-passes and four low-passes in a row (about
    // -48 dB/oct on each edge).
    this.hp = []; this.lp = [];
    let last = lim, head = null;
    const chain = [];
    for (let k = 0; k < 4; k++) {
      const h = c.createBiquadFilter(); h.type = 'highpass'; h.frequency.value = 10; h.Q.value = 0.707; this.hp.push(h); chain.push(h);
    }
    for (let k = 0; k < 4; k++) {
      const l = c.createBiquadFilter(); l.type = 'lowpass'; l.frequency.value = 22000; l.Q.value = 0.707; this.lp.push(l); chain.push(l);
    }
    for (let k = chain.length - 1; k >= 0; k--) { chain[k].connect(last); last = chain[k]; }
    head = last;
    // The world: everything goes through a low-pass the shock of close indoor fire can close for a moment (the
    // ears stunned), and non-gunfire through a bus that gunfire ducks (the bang pushes everything else away).
    this.muffle = c.createBiquadFilter(); this.muffle.type = 'lowpass'; this.muffle.frequency.value = 22000; this.muffle.Q.value = 0.5;
    // Waking up (the intro's eyes opening): a low-pass the menu sweeps from 1 kHz to the full range (setWake).
    this.wakeLp = c.createBiquadFilter(); this.wakeLp.type = 'lowpass'; this.wakeLp.frequency.value = this.wakeHz || 22000; this.wakeLp.Q.value = 0.5;
    this.wakeLp.connect(head);
    this.muffle.connect(this.wakeLp);
    this.mix = c.createGain(); this.mix.connect(this.muffle);
    this.world = c.createGain(); this.world.connect(this.mix);
    // Persistent music and ambience keep their own tablet-muffle and wake stages, separate from combat ducking
    // and the tablet speaker band-pass. They still share Auralis' master limiter.
    this.mediaFilter = c.createBiquadFilter(); this.mediaFilter.type = 'lowpass'; this.mediaFilter.frequency.value = 20000; this.mediaFilter.Q.value = 0.6;
    this.mediaGain = c.createGain(); this.mediaGain.gain.value = 1;
    this.mediaWakeLp = c.createBiquadFilter(); this.mediaWakeLp.type = 'lowpass'; this.mediaWakeLp.frequency.value = this.wakeHz || 22000; this.mediaWakeLp.Q.value = 0.5;
    this.mediaFilter.connect(this.mediaGain).connect(this.mediaWakeLp).connect(lim);
    // UI and cinematic sounds bypass world effects, but still pass through Auralis' master limiter.
    this.ui = c.createGain(); this.ui.connect(lim);
    // Room heat: sustained fire indoors keeps the room ringing on top of itself (rises per shot, fades in ~1.5 s).
    this.heat = 0; this.heatAt = 0;
    // Saturation curve for gunfire close by indoors: the overloaded crack of a shot in a hallway (bodycam).
    const n = 2048, curve = new Float32Array(n);
    for (let i = 0; i < n; i++) { const x = (i / (n - 1)) * 2 - 1; curve[i] = Math.tanh(3.2 * x) / Math.tanh(3.2); }
    this.crunch = curve;
    // Reverb buses: each sound is sent to the space it is made in (its own sends, per voice).
    const bus = (ir) => {
      const input = c.createGain();
      const conv = c.createConvolver(); conv.buffer = ir;
      // A room's ring without the boom (the low end stays in the direct sound).
      const hp = c.createBiquadFilter(); hp.type = 'highpass'; hp.frequency.value = 220; hp.Q.value = 0.6;
      // The convolver normalises its impulse (quiet): the ring is brought back up to sit with the shot.
      const wet = c.createGain(); wet.gain.value = 2.2;
      input.connect(conv); conv.connect(hp); hp.connect(wet); wet.connect(this.mix);
      return input;
    };
    this.rev = {
      small: bus(impulse(c, 1.0, 0.55, [[0.006, 0.6], [0.011, 0.45], [0.017, 0.35], [0.024, 0.25]], 0.35)),
      large: bus(impulse(c, 3.0, 1.7, [[0.018, 0.45], [0.033, 0.35], [0.05, 0.25], [0.08, 0.2]], 0.4)),
      // A corridor flutters: regular echoes between the two close walls.
      corridor: bus(impulse(c, 1.8, 1.0, Array.from({ length: 14 }, (_, k) => [0.009 * (k + 1), 0.5 * Math.pow(0.8, k)]), 0.4)),
    };
    // The open air's gentle tail, for loud sounds outdoors.
    this.air = c.createGain();
    const airConv = c.createConvolver(); airConv.buffer = airTail(c);
    this.air.connect(airConv); airConv.connect(this.mix);
    // Ringing ears: close gunfire indoors leaves a high whine that fades over a few seconds.
    const ring = c.createOscillator(); ring.type = 'sine'; ring.frequency.value = 6800;
    this.ringGain = c.createGain(); this.ringGain.gain.value = 0;
    // The whine is in the head, not the room: after the muffle.
    ring.connect(this.ringGain).connect(head); ring.start();
    this.ring = 0; this.ringAt = 0;
    this.applyEnv(true);
    return c;
  }

  /// Decode critical one-shots before the player enters the field.
  async prepare(urls, onProgress = () => {}) {
    const context = this.ensure();
    if (!context) throw new Error('Web Audio is unavailable; sound assets cannot be decoded.');
    const failures = [];
    let completed = 0;
    await Promise.all(urls.map(async (url) => {
      try {
        const ready = this.buffer(url);
        if (!ready) {
          const pending = this.bufferLoads.get(url);
          if (!pending) throw new Error('Audio decode did not start');
          await pending;
        }
        if (!this.buffers.get(url) || this.buffers.get(url) === 'loading') throw new Error('Audio decode returned no buffer');
      } catch (error) { failures.push({ url, error: error?.message || String(error) }); }
      finally { completed++; onProgress({ completed, total: urls.length, url, ok: !failures.some((item) => item.url === url), error: failures.find((item) => item.url === url)?.error || null }); }
    }));
    return failures;
  }

  /// Loads (once) and returns a decoded sound, or null while it is still loading.
  buffer(url) {
    const b = this.buffers.get(url);
    if (b && b !== 'loading') return b;
    if (!b && this.ctx) {
      this.buffers.set(url, 'loading');
      // From the preloaded bytes when the page has them (a copy: decoding takes the buffer over).
      const get = this.fetchBytes || ((u) => fetch(u).then((r) => r.arrayBuffer()));
      const context = this.ctx;
      const pending = get(url).then((a) => context.decodeAudioData(a.slice(0))).then((d) => {
        if (this.ctx === context) this.buffers.set(url, d);
        return d;
      }).catch((error) => {
        if (this.ctx === context) this.buffers.delete(url);
        throw error;
      });
      this.bufferLoads.set(url, pending);
      pending.catch(() => {});
    }
    return null;
  }

  /// The listener's space from the acoustics: [enclosure, size m, rt60 s, corridor, tablet 0/1].
  setEnv(e) {
    if (!e || e.length < 5 || ![...e].every(Number.isFinite)) return;
    this.env = { enclosure: e[0], size: e[1], rt60: e[2], corridor: e[3], pda: e[4] };
    this.applyEnv(false);
  }

  applyEnv(now) {
    const c = this.ctx; if (!c) return;
    const { pda } = this.env;
    const t = c.currentTime;
    // Through the tablet's small speaker: nothing below 500 Hz, nothing above 10 kHz.
    for (const h of this.hp) h.frequency.setTargetAtTime(pda ? 500 : 10, t, 0.08);
    for (const l of this.lp) l.frequency.setTargetAtTime(pda ? 10000 : 22000, t, 0.08);
  }

  /// Plays a sound as the acoustics place it. `o`: { db, gain, lowpass, room, rx, ry, below, dist, delay, rate,
  /// kind, space: [enclosure, size m, corridor], echoes: [[extra delay s, gain, pan], ...] }: room is how much
  /// rings in the room it is made in; rx/ry/below place it around the ears (the camera); echoes come off what
  /// stands around it.
  /// A sound that keeps playing at one place (the jukebox): an <audio> element through the same acoustics as
  /// every other sound (distance, walls, the room's ring, around the head), re-aimed with `place(record)` (a
  /// `hear_at` record) a few times a second. Its own volume and three tone controls sit before the acoustics.
  stream(el) {
    if (!el || (typeof el !== 'object' && typeof el !== 'function')) return null;
    // Repeated requests must reuse the original source node and processing graph. The Web Audio
    // API forbids creating a second MediaElementSourceNode for the same HTMLMediaElement.
    const existing = MEDIA_ELEMENT_STREAMS.get(el);
    if (existing) return existing.stream;
    if (MEDIA_ELEMENT_ROUTES.has(el)) return null;

    const c = this.ensure(); if (!c) return null;
    let src;
    try {
      src = c.createMediaElementSource(el);
    } catch (error) {
      // Another audio path may already own this element's one permitted source node.
      // Do not let InvalidStateError escape and interrupt gameplay.
      if (error?.name === 'InvalidStateError') {
        console.warn('[Scalia audio] Media element already has a source node; duplicate routing skipped.', error);
        return null;
      }
      throw error;
    }
    const vol = c.createGain(); vol.gain.value = 0.5;
    const eq = (type, f, q) => { const b = c.createBiquadFilter(); b.type = type; b.frequency.value = f; if (q) b.Q.value = q; return b; };
    const bass = eq('lowshelf', 120), treble = eq('peaking', 2500, 0.8), highs = eq('highshelf', 8000);
    const g = c.createGain(); g.gain.value = 0;
    const transmissionMid = eq('peaking', 1500, 0.65);
    const transmissionHigh = eq('highshelf', 5000);
    const hpf = eq('highpass', 30, 0.6), lp = eq('lowpass', 20000, 0.5);
    const p = c.createPanner(); p.panningModel = 'HRTF'; p.distanceModel = 'linear'; p.rolloffFactor = 0;
    const dry = c.createGain();
    src.connect(vol).connect(bass).connect(treble).connect(highs).connect(g).connect(transmissionMid).connect(transmissionHigh).connect(hpf).connect(lp).connect(p).connect(dry).connect(this.world);
    const sends = [this.rev.small, this.rev.large, this.rev.corridor].map((bus) => { const s = c.createGain(); s.gain.value = 0; p.connect(s).connect(bus); return s; });
    const num = (x, d) => (Number.isFinite(x) ? x : d);
    const glide = (param, v) => param.setTargetAtTime(v, c.currentTime, 0.08);
    const stream = {
      /// 0..2 (1 = as recorded).
      volume(v) { glide(vol.gain, Math.max(0, Math.min(2, v))); },
      /// Each 0..10, 5 flat: bass, treble (presence) and highs, ±12 dB.
      tone(b, t, h) { for (const [f, x] of [[bass, b], [treble, t], [highs, h]]) glide(f.gain, (Math.max(0, Math.min(10, x)) - 5) * 2.4); },
      place(r) {
        if (!r || r.length < 14) return;
        const [gain, lowpass, room, rx, ry, below, dist, enc, size, cor] = [r[3], r[4], r[5], r[6], r[7], r[8], r[9], r[11], r[12], r[13]].map((x) => num(x, 0));
        const lowDb = num(r[27], 0), midDb = num(r[28], lowDb), highDb = num(r[29], midDb);
        glide(g.gain, Math.max(0, Math.min(4, gain)));
        glide(transmissionMid.gain, -Math.min(36, Math.max(0, midDb - lowDb)));
        glide(transmissionHigh.gain, -Math.min(48, Math.max(0, highDb - lowDb)));
        glide(lp.frequency, Math.max(200, Math.min(20000, lowpass || 20000)));
        glide(hpf.frequency, Math.min(90, 30 + dist * 0.3));
        const [x, y, z] = spatialPosition(rx, ry, below);
        glide(p.positionX, x); glide(p.positionY, y); glide(p.positionZ, z);
        glide(dry.gain, 1);
        const roomy = Math.max(0, Math.min(1, (9 - size) / 5)), c2 = cor * enc;
        [enc * roomy * (1 - c2), enc * (1 - roomy) * (1 - c2), c2].forEach((w, k) => glide(sends[k].gain, Math.min(0.35, w * room)));
      },
    };
    MEDIA_ELEMENT_STREAMS.set(el, { context: c, source: src, stream });
    return stream;
  }

  play(url, o) {
    const c = this.ensure(); if (!c) return;
    // Anything odd coming in (a missing or non-finite number) is made safe here: audio never stops the game.
    const num = (x, d, lo, hi) => (Number.isFinite(x) ? Math.min(hi, Math.max(lo, x)) : d);
    o = { ...o, gain: num(o.gain, 0, 0, 8), lowpass: num(o.lowpass, 20000, 200, 20000), lowDb: num(o.lowDb, 0, 0, 120), midDb: num(o.midDb, 0, 0, 120), highDb: num(o.highDb, 0, 0, 120), room: num(o.room, 0, 0, 1), dist: num(o.dist, 0, 0, 1e5),
      far: num(o.far, 0, 0, 1), rx: num(o.rx, 0, -1e6, 1e6), ry: num(o.ry, 1, -1e6, 1e6), below: num(o.below, 0, 0, 1e4), delay: num(o.delay, 0, 0, 30), rate: num(o.rate, 1, 0.1, 4) };
    const sp = o.space || [0, 30, 0];
    const enc = num(sp[0], 0, 0, 1), size = num(sp[1], 30, 0, 100), cor = num(sp[2], 0, 0, 1) * enc;
    if (o.gain <= 0.0005) return;
    // HDR: how loud it is at the ear, against the loudest lately.
    const heard = num(o.db, 100, 0, 200) + 20 * Math.log10(o.gain);
    const peak = this.peakNow();
    // Close by (his own and his men's steps, a mag change at arm's length) nothing is lost under the fight: it
    // is pushed down a little at most, never cut.
    const near = o.dist < 15;
    if (heard < peak - this.windowDb && !near) return;
    if (heard > peak) {
      // A new loudest: everything already playing well under it ducks for a moment (the ears clamp).
      const jump = heard - peak;
      if (jump > 12) {
        const duck = Math.pow(10, -Math.min(24, (jump - 12) * 0.6) / 20);
        for (const v of this.active) if (!v.done && v.out?.gain) { v.out.gain.cancelScheduledValues(c.currentTime); v.out.gain.setTargetAtTime(v.out.gain.value * duck, c.currentTime, 0.02); v.out.gain.setTargetAtTime(v.out.gain.value, c.currentTime + 0.4, 0.6); }
      }
      this.peak = heard; this.peakAt = c.currentTime;
    }
    // Well under the peak: pushed down further (the loud fight masks the quiet things).
    const under = Math.min(near ? 12 : 1e9, Math.max(0, peak - heard - QUIET_DB));
    o.gain *= Math.pow(10, -under * 0.5 / 20);
    const buf = this.buffer(url); if (!buf) return;
    const pri = PRIORITY[o.kind] || 1;
    if (!this._admitVoice(pri, o.gain)) return null;
    // Gunfire inside (Battlefield 3 / bodycam): it overwhelms. Close shots crunch (saturated), the room rings
    // hard and keeps ringing as the fire goes on, the walls slap the crack straight back, everything else is
    // shoved aside for a moment, and close by the ears ring and dull.
    const gun = o.kind === 'shot' || o.kind === 'blast';
    const inside = gun ? enc : 0;
    const close = Math.max(0, Math.min(1, 1 - o.dist / 30));
    const nodes = [];
    const own = (node) => { nodes.push(node); return node; };
    const src = own(c.createBufferSource()); src.buffer = buf;
    const timeScaled = o.timeScaled === true;
    const baseRate = o.rate || 1;
    src.playbackRate.value = baseRate * (timeScaled ? this.worldTimeScale : 1);
    const g = own(c.createGain()); g.gain.value = o.gain * (1 + 0.6 * inside);
    const body = own(c.createWaveShaper());
    const drive = inside * close * (1 - o.far);
    if (drive > 0.05) { body.curve = this.crunch; body.oversample = '4x'; }
    const transmissionMid = own(c.createBiquadFilter()); transmissionMid.type = 'peaking'; transmissionMid.frequency.value = 1500; transmissionMid.Q.value = 0.65;
    transmissionMid.gain.value = -Math.min(36, Math.max(0, o.midDb - o.lowDb));
    const transmissionHigh = own(c.createBiquadFilter()); transmissionHigh.type = 'highshelf'; transmissionHigh.frequency.value = 5000;
    transmissionHigh.gain.value = -Math.min(48, Math.max(0, o.highDb - o.lowDb));
    const lp = own(c.createBiquadFilter()); lp.type = 'lowpass'; lp.frequency.value = Math.max(200, Math.min(20000, o.lowpass)); lp.Q.value = 0.5;
    // The lows carry further; material transmission removes progressively more mid and high energy.
    const hpf = own(c.createBiquadFilter()); hpf.type = 'highpass'; hpf.frequency.value = Math.min(90, 30 + o.dist * 0.3); hpf.Q.value = 0.6;
    // Distant fire is a boom more than a crack: the low end comes up as the top goes.
    const boom = own(c.createBiquadFilter()); boom.type = 'lowshelf'; boom.frequency.value = 160; boom.gain.value = gun ? 7 * o.far : 0;
    const p = own(c.createPanner());
    p.panningModel = 'HRTF'; p.distanceModel = 'linear'; p.rolloffFactor = 0; p.refDistance = 1;
    // Around the head (the camera): right is +x, ahead is -z, below is -y. Below counts for less than the
    // ground direction, so left and right stay clear from high up.
    const [x, y, z] = spatialPosition(o.rx, o.ry, o.below);
    p.positionX.value = x; p.positionY.value = y; p.positionZ.value = z;
    // The direct sound: far off it is quieter, thinner and duller, never drowned in reverb.
    const dry = own(c.createGain()); dry.gain.value = 1;
    const out = own(c.createGain());
    // Driven into the curve by how close and how enclosed; brought back down after it.
    const pre = own(c.createGain()); pre.gain.value = 1 + 2.5 * drive;
    const post = own(c.createGain()); post.gain.value = 1 / (1 + 1.2 * drive);
    src.connect(g).connect(pre).connect(body).connect(post).connect(out).connect(transmissionMid).connect(transmissionHigh).connect(hpf).connect(boom).connect(lp).connect(p);
    p.connect(dry).connect(gun ? this.mix : this.world);
    // A room it is made in rings: small or large by size, or a corridor's flutter. Open ground: no ring.
    const roomy = Math.max(0, Math.min(1, (9 - size) / 5));
    const sends = [[this.rev.small, enc * roomy * (1 - cor)], [this.rev.large, enc * (1 - roomy) * (1 - cor)], [this.rev.corridor, cor]];
    if (inside > 0) {
      const now = c.currentTime;
      this.heat = Math.min(1.5, this.heat * Math.exp(-(now - this.heatAt) / 1.5) + 0.25 * inside);
      this.heatAt = now;
    }
    for (const [busIn, w] of sends) {
      const amount = Math.min(0.45, w * o.room * (1 + 2.6 * inside) * (1 + (inside > 0 ? this.heat : 0)));
      if (amount < 0.01) continue;
      const send = own(c.createGain()); send.gain.value = amount;
      p.connect(send).connect(busIn);
    }
    // Outdoors a loud sound trails off in a soft echo rather than stopping dead: more the louder it is and the
    // more open the ground (rooms have their own ring).
    const loudness = Math.max(0, Math.min(1, (num(o.db, 100, 0, 200) - 110) / 40));
    const airAmount = Math.min(0.35, (1 - enc) * loudness * (0.45 + 0.5 * (gun ? o.far : 0)));
    if (airAmount > 0.01) {
      const send = own(c.createGain()); send.gain.value = airAmount;
      p.connect(send).connect(this.air);
    }
    // Echoes off what stands around it: each a little later, weaker and darker, from the wall's side.
    for (const e of o.echoes || []) {
      const [edelay, egain, epan] = [num(e[0], 0, 0, 2), num(e[1], 0, 0, 1), num(e[2], 0, -1, 1)];
      if (egain < 0.02) continue;
      const d = own(c.createDelay(2)); d.delayTime.value = edelay;
      const eg = own(c.createGain()); eg.gain.value = Math.min(0.5, egain);
      const elp = own(c.createBiquadFilter()); elp.type = 'lowpass'; elp.frequency.value = Math.min(lp.frequency.value, 5000) * 0.6; elp.Q.value = 0.5;
      const ep = own(c.createStereoPanner()); ep.pan.value = epan * 0.85;
      lp.connect(d).connect(eg).connect(elp).connect(ep).connect(this.mix);
    }
    // Distant fire rolls: the report comes back off the land and buildings far off, late, dark and soft, the
    // further away the later.
    if (gun && o.far > 0.05) {
      const spread = 1 + o.dist / 150;
      for (const [dt, gk, pan] of [[0.11, 0.4, -0.5], [0.27, 0.26, 0.45], [0.52, 0.15, -0.2], [0.9, 0.08, 0.3]]) {
        const d = own(c.createDelay(4)); d.delayTime.value = Math.min(3.9, dt * spread);
        const eg = own(c.createGain()); eg.gain.value = gk * o.far;
        const elp = own(c.createBiquadFilter()); elp.type = 'lowpass'; elp.frequency.value = Math.min(lp.frequency.value, 1400); elp.Q.value = 0.4;
        const ep = own(c.createStereoPanner()); ep.pan.value = pan;
        lp.connect(d).connect(eg).connect(elp).connect(ep).connect(this.mix);
      }
    }
    // A blast's shock: the pressure wave arrives with the sound and squeezes everything for a moment, more the
    // closer (bodycam: the world goes dull and comes back).
    if (o.kind === 'blast' && o.dist < 80 && this.muffle) {
      const t0 = c.currentTime + Math.max(0, o.delay || 0);
      const k = Math.max(0, 1 - o.dist / 80) * Math.min(1, o.gain);
      const mf = this.muffle.frequency;
      mf.cancelScheduledValues(t0); mf.setValueAtTime(mf.value, t0);
      mf.linearRampToValueAtTime(Math.max(350, 22000 - 21500 * k), t0 + 0.02);
      mf.setTargetAtTime(22000, t0 + 0.15 + 0.4 * k, 0.5 + k);
      const wg = this.world.gain;
      wg.cancelScheduledValues(t0); wg.setValueAtTime(wg.value, t0);
      wg.linearRampToValueAtTime(Math.max(0.15, 1 - 0.8 * k), t0 + 0.02);
      wg.setTargetAtTime(1, t0 + 0.2, 0.6);
    }
    if (inside > 0.3) {
      const t0 = c.currentTime + Math.max(0, o.delay || 0);
      // The slap: the nearest walls throw the crack straight back (a few, close together, from both sides).
      for (const [dt, gk, pan] of [[0.007, 0.7, -0.6], [0.013, 0.55, 0.6], [0.021, 0.45, -0.3], [0.034, 0.3, 0.2]]) {
        const d = own(c.createDelay(0.1)); d.delayTime.value = dt * (0.6 + size / 20);
        const eg = own(c.createGain()); eg.gain.value = gk * inside;
        const ep = own(c.createStereoPanner()); ep.pan.value = pan;
        lp.connect(d).connect(eg).connect(ep).connect(this.mix);
      }
      // The bang shoves everything else aside for a moment (footsteps, the room's own noises).
      const wg = this.world.gain;
      wg.cancelScheduledValues(t0); wg.setValueAtTime(wg.value, t0);
      wg.linearRampToValueAtTime(Math.max(0.25, 1 - 0.7 * inside * (0.4 + close)), t0 + 0.015);
      wg.setTargetAtTime(1, t0 + 0.08, 0.35);
      // Ringing, dulled ears from close shots: build up with each one, the whine fades over ~5 s, the world
      // comes back from muffled over a second or so after a burst.
      if (o.dist < 25 && this.ringGain) {
        const now = c.currentTime;
        this.ring = Math.min(0.035, this.ring * Math.exp(-(now - this.ringAt) / 2.5) + 0.007 * inside);
        this.ringAt = now;
        const rg = this.ringGain.gain;
        rg.cancelScheduledValues(t0); rg.setValueAtTime(rg.value, t0);
        rg.linearRampToValueAtTime(this.ring, t0 + 0.05);
        rg.setTargetAtTime(0, t0 + 0.5, 2.5);
        const stun = this.ring / 0.035;
        if (stun > 0.3) {
          const mf = this.muffle.frequency;
          mf.cancelScheduledValues(t0); mf.setValueAtTime(mf.value, t0);
          mf.linearRampToValueAtTime(Math.max(1800, 22000 - 19000 * stun), t0 + 0.03);
          mf.setTargetAtTime(22000, t0 + 0.25, 0.6);
        }
      }
    }
    // Keep the source graph only as long as its audible tails need it. Every node created for this voice
    // is tracked and disconnected on retirement; tail retention is bounded independently from live voices.
    const maxEcho = (o.echoes || []).reduce((m, e) => Math.max(m, num(e[0], 0, 0, 2)), 0);
    const farTail = gun && o.far > 0.05 ? Math.min(3.9, 0.9 * (1 + o.dist / 150)) + 0.12 : 0;
    const hasRoomTail = sends.some(([bus, weight]) => weight * o.room * (1 + 2.6 * inside) * (1 + (inside > 0 ? this.heat : 0)) > 0.01);
    const hasAirTail = airAmount > 0.01;
    const tailSeconds = Math.min(4.2, Math.max(
      maxEcho > 0 ? maxEcho + 0.12 : 0,
      farTail,
      hasRoomTail ? 3.1 : 0,
      hasAirTail ? 2.1 : 0,
      inside > 0.3 ? 0.12 : 0,
    ));
    const voice = this._registerVoice(src, nodes, {
      category: o.kind || 'world', priority: pri, gain: o.gain, tailSeconds, out,
    });
    voice.timeScaled = timeScaled;
    voice.baseRate = baseRate;
    try {
      src.start(c.currentTime + Math.max(0, o.delay || 0));
      voice.state = 'playing';
      return voice.handle;
    } catch (error) {
      this._cleanupVoice(voice);
      throw error;
    }
  }
}
