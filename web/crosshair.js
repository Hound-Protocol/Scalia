// Two-point, character-bound ballistic reticle. All positions are normalized viewer coordinates.
const finitePoint = (point) => Array.isArray(point) && point.length === 4 && point.every(Number.isFinite);

export const DEFAULT_CROSSHAIR_HOOKS = Object.freeze({
  trajectory: (state) => state?.trajectory?.segments ?? (finitePoint(state?.line)
    ? [{ line: state.line, status: state.penetration ?? 'clear' }]
    : []),
  recoil: (state) => state?.recoil ?? 0,
});

const STYLES = Object.freeze({
  clear: { color: '#ffffff', dash: '' },
  penetrates: { color: '#ffffff', dash: '3 4' },
  caveat: { color: '#ffd43b', dash: '2 6' },
  blocked: { color: '#ff3b30', dash: '1 6' },
});

export class CrosshairController {
  constructor({ canvas, line, referenceDot, aimDot, hooks = DEFAULT_CROSSHAIR_HOOKS, createLine }) {
    this.canvas = canvas;
    this.line = line;
    this.group = line?.querySelector?.('g') ?? line;
    this.referenceDot = referenceDot;
    this.aimDot = aimDot;
    this.hooks = hooks;
    this.createLine = createLine ?? (() => this.line.ownerDocument.createElementNS('http://www.w3.org/2000/svg', 'line'));
    this.hide();
  }

  hide() {
    if (this.line) this.line.hidden = true;
    if (this.referenceDot) this.referenceDot.hidden = true;
    if (this.aimDot) this.aimDot.hidden = true;
  }

  update({ state, pointer, active = true }) {
    const segments = active ? this.hooks.trajectory(state) : null;
    if (!Array.isArray(segments) || segments.length === 0 || !segments.every((segment) => {
      const points = Array.isArray(segment) ? segment.slice(0, 4) : segment?.line;
      return finitePoint(points);
    })) {
      this.hide();
      if (this.canvas) this.canvas.style.cursor = '';
      return;
    }

    const width = this.canvas.clientWidth;
    const height = this.canvas.clientHeight;
    if (!(width > 0 && height > 0)) { this.hide(); return; }
    // SVG coordinates are emitted in canvas CSS pixels; match its user space to those dimensions.
    this.line?.setAttribute('viewBox', `0 0 ${width} ${height}`);
    this.line?.setAttribute('preserveAspectRatio', 'none');
    while (this.group?.firstChild) this.group.removeChild(this.group.firstChild);
    for (const segment of segments) {
      const points = Array.isArray(segment) ? segment.slice(0, 4) : segment.line;
      const status = Array.isArray(segment) ? segment[4] : segment.status;
      const style = STYLES[status] ?? STYLES.clear;
      const node = this.createLine();
      node.setAttribute('x1', (points[0] * width).toFixed(1));
      node.setAttribute('y1', (points[1] * height).toFixed(1));
      node.setAttribute('x2', (points[2] * width).toFixed(1));
      node.setAttribute('y2', (points[3] * height).toFixed(1));
      node.setAttribute('stroke', style.color);
      node.setAttribute('stroke-width', '1.5');
      node.setAttribute('stroke-linecap', 'round');
      if (style.dash) node.setAttribute('stroke-dasharray', style.dash);
      this.group?.appendChild(node);
    }
    if (this.line) this.line.hidden = false;

    // The opaque reference follows the user's pointer; the translucent point follows the simulated weapon ray.
    if (this.referenceDot) {
      if (pointer && Number.isFinite(pointer.x) && Number.isFinite(pointer.y)) {
        this.referenceDot.style.transform = `translate(${pointer.x.toFixed(1)}px, ${pointer.y.toFixed(1)}px)`;
        this.referenceDot.hidden = false;
      } else {
        this.referenceDot.hidden = true;
      }
    }
    const last = segments[segments.length - 1];
    const endpoint = Array.isArray(last) ? last.slice(2, 4) : last.line.slice(2, 4);
    if (this.aimDot) {
      this.aimDot.style.transform = `translate(${(endpoint[0] * width).toFixed(1)}px, ${(endpoint[1] * height).toFixed(1)}px)`;
      this.aimDot.hidden = false;
      this.aimDot.dataset.recoil = String(Math.max(0, Number(this.hooks.recoil(state)) || 0));
    }
    if (this.canvas) this.canvas.style.cursor = 'none';
  }
}
