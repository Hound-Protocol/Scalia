// Contextual world-interaction hints. Rust owns eligibility and target metadata;
// this module owns only projection placement, input-token rendering and presentation.
const KEYBOARD_GLYPHS = Object.freeze({ use: 'E' });

export class HintManager {
  constructor({ viewer, canvas, root, visible }) {
    this.viewer = viewer;
    this.canvas = canvas;
    this.root = root || document.getElementById('interactionHints');
    this.visible = visible || (() => true);
    this.elements = new Map();
  }

  resolveInputToken(token) {
    return String(token || '').replace(/\{\+([a-z0-9_-]+)\}/gi, (_, action) =>
      KEYBOARD_GLYPHS[action.toLowerCase()] || action.toUpperCase());
  }

  update() {
    if (!this.visible()) {
      this.root.hidden = true;
      this.clear();
      return;
    }
    this.root.hidden = false;

    let marks;
    try { marks = JSON.parse(this.viewer.hint_marks() || '[]'); }
    catch { marks = []; }

    const width = this.canvas.clientWidth;
    const height = this.canvas.clientHeight;
    const seen = new Set();

    for (const hint of marks) {
      if (!Number.isFinite(hint.x) || !Number.isFinite(hint.y)) continue;
      let x = hint.x, y = hint.y;
      if (hint.clampToScreen) {
        const dx = x - 0.5, dy = y - 0.5;
        const safeX = 0.44, safeY = 0.44;
        const tx = Math.abs(dx) > 1e-6 ? safeX / Math.abs(dx) : Infinity;
        const ty = Math.abs(dy) > 1e-6 ? safeY / Math.abs(dy) : Infinity;
        const t = Math.min(1, tx, ty);
        x = 0.5 + dx * t;
        y = 0.5 + dy * t;
      }

      let element = this.elements.get(hint.id);
      const fresh = !element;
      if (!element) {
        element = document.createElement('div');
        element.className = 'interaction-hint';
        const caption = document.createElement('div');
        caption.className = 'interaction-hint__caption';
        const row = document.createElement('div');
        row.className = 'interaction-hint__row';
        const key = document.createElement('kbd');
        key.className = 'interaction-hint__key';
        const action = document.createElement('span');
        action.className = 'interaction-hint__action';
        action.textContent = 'INTERACT';
        row.append(key, action);
        element.append(caption, row);
        this.root.appendChild(element);
        this.elements.set(hint.id, element);
      }

      const caption = element.querySelector('.interaction-hint__caption');
      const key = element.querySelector('.interaction-hint__key');
      const nextCaption = String(hint.caption || 'INTERACT');
      const nextKey = this.resolveInputToken(hint.input);
      const changed = caption.textContent !== nextCaption || key.textContent !== nextKey;
      if (changed) {
        caption.textContent = nextCaption;
        key.textContent = nextKey;
      }
      element.style.left = `${(x * width).toFixed(1)}px`;
      element.style.top = `${(y * height).toFixed(1)}px`;
      if (fresh || changed) {
        element.classList.remove('is-live');
        void element.offsetWidth;
        element.classList.add('is-live');
      }
      seen.add(hint.id);
    }

    for (const [id, element] of this.elements) {
      if (!seen.has(id)) {
        element.remove();
        this.elements.delete(id);
      }
    }
  }

  clear() {
    for (const element of this.elements.values()) element.remove();
    this.elements.clear();
  }
}
