// Character-bound crosshair: the pointer dot is player input; the ring is the character's live aim axis.
const finite = (value, fallback = 0) => Number.isFinite(Number(value)) ? Number(value) : fallback;

export const DEFAULT_CROSSHAIR_HOOKS = Object.freeze({
  aimLine: (state) => state?.line,
  spread: (state) => state?.spread,
});

export class CrosshairController {
  constructor({ canvas, line, ring, dot, hooks = DEFAULT_CROSSHAIR_HOOKS, baseRadius = 7, spreadRadius = 16 }) {
    this.canvas = canvas;
    this.line = line;
    this.linePath = line?.firstElementChild ?? null;
    this.ring = ring;
    this.dot = dot;
    this.hooks = hooks;
    this.baseRadius = baseRadius;
    this.spreadRadius = spreadRadius;
    this.hide();
  }

  hide() {
    if (this.line) this.line.hidden = true;
    if (this.ring) this.ring.hidden = true;
    if (this.dot) this.dot.hidden = true;
  }

  update({ state, pointer, active = true }) {
    const aimLine = active ? this.hooks.aimLine(state) : null;
    if (!Array.isArray(aimLine) || aimLine.length !== 4 || !aimLine.every(Number.isFinite)) {
      this.hide();
      if (this.canvas) this.canvas.style.cursor = '';
      return;
    }

    const width = this.canvas.clientWidth;
    const height = this.canvas.clientHeight;
    const [x0, y0, x1, y1] = aimLine;
    if (this.linePath) {
      this.linePath.setAttribute('x1', (x0 * width).toFixed(1));
      this.linePath.setAttribute('y1', (y0 * height).toFixed(1));
      this.linePath.setAttribute('x2', (x1 * width).toFixed(1));
      this.linePath.setAttribute('y2', (y1 * height).toFixed(1));
    }
    if (this.line) this.line.hidden = false;

    // Never cosmetically displace the ring: it must sit on the character's reported aim point.
    const radius = this.baseRadius + this.spreadRadius * Math.max(0, Math.min(1, finite(this.hooks.spread(state))));
    if (this.ring) {
      this.ring.style.transform = `translate(${(x1 * width).toFixed(1)}px, ${(y1 * height).toFixed(1)}px)`;
      this.ring.style.setProperty('--r', `${radius.toFixed(1)}px`);
      this.ring.hidden = false;
    }

    if (this.dot && pointer && Number.isFinite(pointer.x) && Number.isFinite(pointer.y)) {
      this.dot.style.transform = `translate(${pointer.x.toFixed(1)}px, ${pointer.y.toFixed(1)}px)`;
      this.dot.hidden = false;
    } else if (this.dot) {
      this.dot.hidden = true;
    }
    if (this.canvas) this.canvas.style.cursor = 'none';
  }
}
