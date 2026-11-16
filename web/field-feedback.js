// Diegetic player feedback: no permanent status HUD, only brief contextual controls and character reports.
export class FieldFeedback {
  constructor({ viewer, canvas, root = document.body, active = () => true }) {
    this.viewer = viewer;
    this.canvas = canvas;
    this.active = active;
    this.controlsVisible = true;
    this.lastHealth = 100;
    this.lastStance = -1;
    this.sprintLoad = 0;
    this.stamina = 1;
    this.checkPhase = null;
    this.checkPhaseAt = 0;
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
        <div><kbd>H</kbd><span>SELF CHECK</span></div>
        <div><kbd>R</kbd><span>TAP · RELOAD / HOLD · CHECK</span></div>
        <div><kbd>C / X</kbd><span>STANCE</span></div>
        <div><kbd>G</kbd><span>SLING / READY RIFLE</span></div>
        <div><kbd>TAB</kbd><span>FIELD PDA</span></div>
        <small>U · TOGGLE CONTROLS</small>
      </aside>
      <div class="field-vignette field-vignette--hurt"></div>
      <div class="field-selfcheck" hidden><span>SELF CHECK</span><i><b></b></i></div>
      <div class="field-report" hidden></div>
      <div class="field-stance" hidden></div>`;
    root.append(this.el);
    this.controls = this.el.querySelector('.field-controls');
    this.hurt = this.el.querySelector('.field-vignette--hurt');
    this.progress = this.el.querySelector('.field-selfcheck');
    this.progressBar = this.progress.querySelector('b');
    this.report = this.el.querySelector('.field-report');
    this.stance = this.el.querySelector('.field-stance');
  }
  toggleControls() { this.controlsVisible = !this.controlsVisible; this.controls.hidden = !this.controlsVisible; }
  toggleHealthCheck(now) {
    if (!this.active()) return;
    if (this.checkPhase) { this.cancelHealthCheck(now); return; }
    this.checkPhase = 'checking';
    this.checkPhaseAt = now;
    this.checkDone = false;
    this.progress.hidden = false;
    this.report.hidden = true;
    this.progressBar.style.width = '0%';
    this.viewer.commander_inspect(true);
  }
  cancelHealthCheck(now) {
    if (!this.checkPhase) return;
    this.checkPhase = null;
    this.checkPhaseAt = now;
    this.progress.hidden = true;
    this.progressBar.style.width = '0%';
    this.report.hidden = true;
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
  update(now, dt, weapon, sprinting, jogging) {
    const on = this.active();
    this.controls.hidden = !on || !this.controlsVisible;
    if (!on) { this.cancelHealthCheck(now); this.rStarted = 0; this.hurt.hidden = true; return; }

    const health = Number.isFinite(weapon.health) ? weapon.health : 100;
    // A real shot removes multiple health points; the downed bleed tick removes only one.
    // Do not replay impact feedback while the commander is already on the ground.
    if (health <= this.lastHealth - 2) {
      this.hitUntil = now + 900;
      this.say('I’m hit!', now, 1100);
    }
    this.lastHealth = health;
    this.hurt.hidden = health >= 100 && now >= this.hitUntil;
    this.hurt.style.setProperty('--hurt', String(Math.max(0.12, (100 - health) / 100)));
    const stance = weapon.stance ?? 0;
    if (sprinting && this.stamina > 0) this.sprintLoad = Math.min(40, this.sprintLoad + dt);
    else {
      const regen = jogging ? 0.5 : (stance === 1 || stance === 2 ? 1.2 : 1.0);
      this.sprintLoad = Math.max(0, this.sprintLoad - dt * regen);
    }
    this.stamina = 1 - Math.max(0, this.sprintLoad - 30) / 10;

    if (this.checkPhase) {
      const elapsed = now - this.checkPhaseAt;
      const at = this.viewer.commander_screen();
      this.progress.style.left = (at[0] * innerWidth) + 'px';
      this.progress.style.top = (at[1] * innerHeight - 54) + 'px';
      if (this.checkPhase === 'checking') {
        const p = Math.min(1, elapsed / 1500);
        this.progressBar.style.width = (p * 100).toFixed(1) + '%';
        if (p >= 1) {
          this.checkDone = true;
          const injuryNames = (Array.isArray(weapon.injuries) ? weapon.injuries : []).map((site) => ({
            head: 'HEAD', neck: 'NECK', chest: 'CHEST', abdomen: 'ABDOMEN',
            left_arm: 'LEFT ARM', right_arm: 'RIGHT ARM', left_leg: 'LEFT LEG', right_leg: 'RIGHT LEG',
          }[site] || String(site).toUpperCase().replaceAll('_', ' ')));
          const injuryNote = injuryNames.length ? ` INJURY SITES: ${[...new Set(injuryNames)].join(', ')}.` : '';
          if (health <= 0) this.say(`KIA — NO VITAL SIGNS.${injuryNote}`, now, 2000);
          else if (weapon.down) this.say(`INCAPACITATED — ${weapon.stable ? 'BLEEDING CONTROLLED.' : `BLEEDING ${weapon.bleed_per_sec || 1} HP / SEC.`}${injuryNote}`, now, 2000);
          else if (health < 100) this.say(`WOUNDED — ${health}% HEALTH.${injuryNote}`, now, 2000);
          else this.say('ALL VITAL SIGNS NORMAL.', now, 2000);
          this.progress.hidden = true;
          this.checkPhase = 'report';
          this.checkPhaseAt = now;
        }
      } else if (this.checkPhase === 'report' && elapsed >= 2000) {
        this.checkPhase = 'return';
        this.checkPhaseAt = now;
        this.viewer.commander_inspect(false);
      } else if (this.checkPhase === 'return' && elapsed >= 260) {
        this.checkPhase = null;
        this.report.hidden = true;
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
