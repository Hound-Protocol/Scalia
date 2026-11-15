// Diegetic player feedback: no permanent status HUD, only brief contextual controls and character reports.
export class FieldFeedback {
  constructor({ viewer, canvas, root = document.body, active = () => true }) {
    this.viewer = viewer;
    this.canvas = canvas;
    this.active = active;
    this.controlsVisible = true;
    this.lastHealth = 100;
    this.lastStance = -1;
    this.stamina = 1;
    this.checkStarted = 0;
    this.checkDone = false;
    this.rStarted = 0;
    this.rChecked = false;
    this.messageUntil = 0;
    this.hitUntil = 0;
    this.stanceUntil = 0;
    this.root = root;
    this.el = document.createElement('div');
    this.el.id = 'fieldFeedback';
    this.el.innerHTML = `
      <aside class="field-controls" aria-label="Field controls">
        <div><kbd>W A S D</kbd><span>MOVE</span></div>
        <div><kbd>SHIFT</kbd><span>SPRINT</span></div>
        <div><kbd>H</kbd><span>HOLD · SELF CHECK</span></div>
        <div><kbd>R</kbd><span>TAP · RELOAD / HOLD · CHECK</span></div>
        <div><kbd>C / X</kbd><span>STANCE</span></div>
        <div><kbd>TAB</kbd><span>FIELD PDA</span></div>
        <small>U · HIDE CONTROLS</small>
      </aside>
      <div class="field-vignette field-vignette--hurt"></div>
      <div class="field-vignette field-vignette--stamina"></div>
      <div class="field-selfcheck" hidden><span>SELF CHECK</span><i><b></b></i></div>
      <div class="field-report" hidden></div>
      <div class="field-stance" hidden></div>`;
    root.append(this.el);
    this.controls = this.el.querySelector('.field-controls');
    this.hurt = this.el.querySelector('.field-vignette--hurt');
    this.fatigue = this.el.querySelector('.field-vignette--stamina');
    this.progress = this.el.querySelector('.field-selfcheck');
    this.progressBar = this.progress.querySelector('b');
    this.report = this.el.querySelector('.field-report');
    this.stance = this.el.querySelector('.field-stance');
  }
  toggleControls() { this.controlsVisible = !this.controlsVisible; this.controls.hidden = !this.controlsVisible; }
  beginHealthCheck(now) {
    if (!this.active() || this.checkStarted) return;
    this.checkStarted = now;
    this.checkDone = false;
    this.progress.hidden = false;
    this.viewer.commander_inspect(true);
  }
  endHealthCheck(now) {
    if (!this.checkStarted) return;
    this.checkStarted = 0;
    this.progress.hidden = true;
    this.progressBar.style.width = '0%';
    this.viewer.commander_inspect(false);
  }
  beginAmmoCheck(now) { if (this.active()) { this.rStarted = now; this.rChecked = false; } }
  endAmmoCheck(now) {
    if (!this.rStarted) return;
    const elapsed = now - this.rStarted;
    this.rStarted = 0;
    if (elapsed >= 520 || this.rChecked) return;
    this.viewer.commander_reload();
  }
  say(message, now = performance.now(), duration = 2800) {
    this.report.textContent = message;
    this.report.hidden = false;
    this.messageUntil = now + duration;
  }
  update(now, dt, weapon, sprinting) {
    const on = this.active();
    this.controls.hidden = !on || !this.controlsVisible;
    if (!on) { this.endHealthCheck(now); this.rStarted = 0; this.hurt.hidden = true; this.fatigue.hidden = true; return; }

    const health = Number.isFinite(weapon.health) ? weapon.health : 100;
    if (health < this.lastHealth) {
      this.hitUntil = now + 900;
      this.canvas.classList.remove('field-hit');
      void this.canvas.offsetWidth;
      this.canvas.classList.add('field-hit');
      this.say('I’m hit!', now, 1100);
    }
    this.lastHealth = health;
    this.hurt.hidden = health >= 100 && now >= this.hitUntil;
    this.hurt.style.setProperty('--hurt', String(Math.max(0.12, (100 - health) / 100)));
    this.stamina = Math.max(0, Math.min(1, this.stamina + (sprinting ? -0.20 : 0.12) * dt));
    this.fatigue.hidden = this.stamina > 0.92;
    this.fatigue.style.setProperty('--fatigue', String((1 - this.stamina) * 0.78));

    if (this.checkStarted) {
      const p = Math.min(1, (now - this.checkStarted) / 1450);
      this.progressBar.style.width = (p * 100).toFixed(1) + '%';
      const at = this.viewer.commander_screen();
      this.progress.style.left = (at[0] * innerWidth) + 'px';
      this.progress.style.top = (at[1] * innerHeight - 54) + 'px';
      if (p >= 1 && !this.checkDone) {
        this.checkDone = true;
        if (health >= 100) this.say('I’m all well.', now);
        else this.say('I’ve been hit. I can’t identify the wound yet.', now, 3600);
      }
    }
    if (this.rStarted && !this.rChecked && now - this.rStarted >= 520) {
      this.rChecked = true;
      const mags = Math.floor((weapon.spare || 0) / 30);
      this.say(`Magazine: ${weapon.ammo} / 30. ${mags} spare magazines.`, now, 3000);
    }
    if (now >= this.messageUntil) this.report.hidden = true;

    if (weapon.stance !== this.lastStance) {
      this.lastStance = weapon.stance;
      this.stance.textContent = ['STANDING', 'CROUCHED', 'PRONE'][weapon.stance] || 'STANDING';
      this.stance.hidden = false;
      this.stanceUntil = now + 1300;
    }
    this.stance.hidden = now >= this.stanceUntil;
    if (!this.report.hidden) {
      const p = this.viewer.commander_screen();
      this.report.style.left = (p[0] * innerWidth) + 'px';
      this.report.style.top = (p[1] * innerHeight - 38) + 'px';
    }
  }
}
