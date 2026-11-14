// Pack Engine audio, the browser side. The acoustics (Rust) work out how each sound reaches the listener
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
// HDR audio (as Frostbite): every sound has a logical loudness (dB at the source); what reaches the ear is that
// plus the acoustics' gain. The loudest thing heard lately sets a window: anything more than WINDOW_DB below it
// is not played at all, anything well below it is pushed down further, and the peak relaxes over seconds.
// Presets as in Battlefield: HiFi keeps a wide range (60 dB), War Tapes squeezes it (25 dB: punchy, loud).
const HDR = { hifi: 60, wartapes: 25 };
const QUIET_DB = 20, RELEASE_DB_S = 14, PEAK_FLOOR_DB = 40;
const PRIORITY = { blast: 3, shot: 2, step: 1 };

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

export class AudioEngine {
  constructor() {
    this.ctx = null;
    this.buffers = new Map();
    this.active = [];
    this.env = { enclosure: 0, size: 30, rt60: 0.25, corridor: 0, pda: 0 };
    this.peak = PEAK_FLOOR_DB; this.peakAt = 0;
    this.hdr = 'wartapes'; this.windowDb = HDR.wartapes;
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
    // Tablet band-pass (2 - 7 kHz while it is up): four high-passes and four low-passes in a row (about
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
    this.mix = c.createGain(); this.mix.connect(head);
    // Reverb buses: each sound is sent to the space it is made in (its own sends, per voice).
    const bus = (ir) => {
      const input = c.createGain();
      const conv = c.createConvolver(); conv.buffer = ir;
      // A room's ring without the boom (the low end stays in the direct sound).
      const hp = c.createBiquadFilter(); hp.type = 'highpass'; hp.frequency.value = 220; hp.Q.value = 0.6;
      input.connect(conv); conv.connect(hp); hp.connect(this.mix);
      return input;
    };
    this.rev = {
      small: bus(impulse(c, 1.0, 0.55, [[0.006, 0.6], [0.011, 0.45], [0.017, 0.35], [0.024, 0.25]], 0.35)),
      large: bus(impulse(c, 3.0, 1.7, [[0.018, 0.45], [0.033, 0.35], [0.05, 0.25], [0.08, 0.2]], 0.4)),
      // A corridor flutters: regular echoes between the two close walls.
      corridor: bus(impulse(c, 1.8, 1.0, Array.from({ length: 14 }, (_, k) => [0.009 * (k + 1), 0.5 * Math.pow(0.8, k)]), 0.4)),
    };
    this.applyEnv(true);
    return c;
  }

  /// Loads (once) and returns a decoded sound, or null while it is still loading.
  buffer(url) {
    const b = this.buffers.get(url);
    if (b && b !== 'loading') return b;
    if (!b && this.ctx) {
      this.buffers.set(url, 'loading');
      fetch(url).then((r) => r.arrayBuffer()).then((a) => this.ctx.decodeAudioData(a)).then((d) => this.buffers.set(url, d)).catch(() => this.buffers.delete(url));
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
    // Through the tablet's small speaker: nothing below 2 kHz, nothing above 7 kHz.
    for (const h of this.hp) h.frequency.setTargetAtTime(pda ? 2000 : 10, t, 0.08);
    for (const l of this.lp) l.frequency.setTargetAtTime(pda ? 7000 : 22000, t, 0.08);
  }

  /// Plays a sound as the acoustics place it. `o`: { db, gain, lowpass, room, rx, ry, below, dist, delay, rate,
  /// kind, space: [enclosure, size m, corridor], echoes: [[extra delay s, gain, pan], ...] }: room is how much
  /// rings in the room it is made in; rx/ry/below place it around the ears (the camera); echoes come off what
  /// stands around it.
  play(url, o) {
    const c = this.ensure(); if (!c) return;
    // Anything odd coming in (a missing or non-finite number) is made safe here: audio never stops the game.
    const num = (x, d, lo, hi) => (Number.isFinite(x) ? Math.min(hi, Math.max(lo, x)) : d);
    o = { ...o, gain: num(o.gain, 0, 0, 8), lowpass: num(o.lowpass, 20000, 200, 20000), room: num(o.room, 0, 0, 1), dist: num(o.dist, 0, 0, 1e5),
      rx: num(o.rx, 0, -1e6, 1e6), ry: num(o.ry, 1, -1e6, 1e6), below: num(o.below, 0, 0, 1e4), delay: num(o.delay, 0, 0, 30), rate: num(o.rate, 1, 0.1, 4) };
    const sp = o.space || [0, 30, 0];
    const enc = num(sp[0], 0, 0, 1), size = num(sp[1], 30, 0, 100), cor = num(sp[2], 0, 0, 1) * enc;
    if (o.gain <= 0.0005) return;
    // HDR: how loud it is at the ear, against the loudest lately.
    const heard = num(o.db, 100, 0, 200) + 20 * Math.log10(o.gain);
    const peak = this.peakNow();
    if (heard < peak - this.windowDb) return;
    if (heard > peak) {
      // A new loudest: everything already playing well under it ducks for a moment (the ears clamp).
      const jump = heard - peak;
      if (jump > 12) {
        const duck = Math.pow(10, -Math.min(24, (jump - 12) * 0.6) / 20);
        for (const v of this.active) if (!v.done) { v.out.gain.cancelScheduledValues(c.currentTime); v.out.gain.setTargetAtTime(v.out.gain.value * duck, c.currentTime, 0.02); v.out.gain.setTargetAtTime(v.out.gain.value, c.currentTime + 0.4, 0.6); }
      }
      this.peak = heard; this.peakAt = c.currentTime;
    }
    // Well under the peak: pushed down further (the loud fight masks the quiet things).
    const under = Math.max(0, peak - heard - QUIET_DB);
    o.gain *= Math.pow(10, -under * 0.5 / 20);
    const buf = this.buffer(url); if (!buf) return;
    const pri = PRIORITY[o.kind] || 1;
    this.active = this.active.filter((v) => !v.done);
    if (this.active.length >= VOICES) {
      // Full: the quietest of the least important voices gives way, or this one is dropped.
      const worst = this.active.reduce((a, b) => (a.pri - b.pri || a.gain - b.gain) <= 0 ? a : b);
      if (worst.pri > pri || (worst.pri === pri && worst.gain >= o.gain)) return;
      try { worst.src.stop(); } catch (_) {}
      worst.done = true;
    }
    const src = c.createBufferSource(); src.buffer = buf; src.playbackRate.value = o.rate || 1;
    const g = c.createGain(); g.gain.value = o.gain;
    const lp = c.createBiquadFilter(); lp.type = 'lowpass'; lp.frequency.value = Math.max(200, Math.min(20000, o.lowpass)); lp.Q.value = 0.5;
    // Distance thins the low end too: what carries is the crack, not the boom.
    const hpf = c.createBiquadFilter(); hpf.type = 'highpass'; hpf.frequency.value = Math.min(450, 40 + o.dist * 3); hpf.Q.value = 0.6;
    const p = c.createPanner();
    p.panningModel = 'HRTF'; p.distanceModel = 'linear'; p.rolloffFactor = 0; p.refDistance = 1;
    // Around the head (the camera): right is +x, ahead is -z, below is -y. Below counts for less than the
    // ground direction, so left and right stay clear from high up.
    const by = o.below * 0.35;
    const len = Math.hypot(o.rx, o.ry, by);
    const [x, y, z] = len < 0.3 ? [0, 0, -1] : [o.rx / len, -by / len, -o.ry / len];
    p.positionX.value = x; p.positionY.value = y; p.positionZ.value = z;
    // The direct sound: far off it is quieter, thinner and duller, never drowned in reverb.
    const dry = c.createGain(); dry.gain.value = 1 - o.room * 0.5;
    const out = c.createGain();
    src.connect(g).connect(out).connect(hpf).connect(lp).connect(p);
    p.connect(dry).connect(this.mix);
    // A room it is made in rings: small or large by size, or a corridor's flutter. Open ground: no ring.
    const roomy = Math.max(0, Math.min(1, (9 - size) / 5));
    const sends = [[this.rev.small, enc * roomy * (1 - cor)], [this.rev.large, enc * (1 - roomy) * (1 - cor)], [this.rev.corridor, cor]];
    for (const [busIn, w] of sends) {
      const amount = w * o.room;
      if (amount < 0.01) continue;
      const send = c.createGain(); send.gain.value = amount;
      p.connect(send).connect(busIn);
    }
    // Echoes off what stands around it: each a little later, weaker and darker, from the wall's side.
    for (const e of o.echoes || []) {
      const [edelay, egain, epan] = [num(e[0], 0, 0, 2), num(e[1], 0, 0, 1), num(e[2], 0, -1, 1)];
      if (egain < 0.02) continue;
      const d = c.createDelay(2); d.delayTime.value = edelay;
      const eg = c.createGain(); eg.gain.value = egain;
      const elp = c.createBiquadFilter(); elp.type = 'lowpass'; elp.frequency.value = Math.min(lp.frequency.value, 5000) * 0.6; elp.Q.value = 0.5;
      const ep = c.createStereoPanner(); ep.pan.value = epan * 0.85;
      lp.connect(d).connect(eg).connect(elp).connect(ep).connect(this.mix);
    }
    const v = { src, pri, gain: o.gain, out, done: false };
    src.onended = () => { v.done = true; };
    this.active.push(v);
    src.start(c.currentTime + Math.max(0, o.delay || 0));
  }
}
