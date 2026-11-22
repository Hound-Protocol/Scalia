//! The animation library: one animator per figure, the same for the commander, every soldier and every enemy.
//!
//! A controller (the commander's input, a soldier's battle state) says what the figure is doing (`Drive`): how fast
//! it is really moving and wants to move, where it wants to face, its stance, what its rifle is doing, any action.
//! The animator turns that into what is drawn (`Frame`): legs and upper-body poses with hysteresis and blends,
//! weighted turning with a foot shuffle on the spot, skids and hard stops, the step-out from still, the gait phase
//! with footfalls, banking and pitching, falls, the shot kick, and the instance record the shader reads.
//! Nothing about how a figure looks is decided anywhere else.

use crate::motion::{skidding, AimTrack, Skid};
use scalia_render::pose::{self, Pose, LAYER_BLEND_S};
// AN-03: cadence from the clips the soldier plays.

/// Pace lines (m/s) for the legs, with a little give either way so easing off near a line does not flick poses.
const SPRINT_UP: f32 = 4.0;
const SPRINT_DOWN: f32 = 3.7;
const RUN_UP: f32 = 2.4;
const RUN_DOWN: f32 = 2.1;
const WALK_UP: f32 = 0.2;
const WALK_DOWN: f32 = 0.05;
/// How quickly the momentum lean follows his acceleration (1/s): a sustained push or brake leans him, quick
/// taps back and forth average out instead of rocking him on every one.
const ACCEL_FOLLOW: f32 = 2.5;
/// Within this, steering closes on its heading proportionally (1/s): no chatter about the target.
const STEER_SETTLE: f32 = 12.0;
/// How quickly the bank follows the turn (1/s), and the fastest it may change (rad/s): the body cannot flick from
/// one side to the other, whatever the steering does.
const BANK_FOLLOW: f32 = 8.0;
const BANK_RATE: f32 = 1.0;
/// How fast his legs' travel direction (relative to his body) can swing round (rad/s).
const DIR_RATE: f32 = 20.0;
/// Coming to a stop, below this pace (m/s) he settles into his standing pose.
const STOP_SETTLE: f32 = 0.3;
/// Turning on the spot faster than this (rad/s), his feet step him round instead of sliding.
const SHUFFLE_SPIN: f32 = 1.3;
/// A body turns with weight: angular acceleration (rad/s²).
pub const TURN_ACCEL: f32 = 28.0;
const FALL_BACK_S: f32 = 1.1;
const FALL_FORWARD_S: f32 = 0.95;
/// Getting down to prone or back up: how fast he moves along the drop (`Pose::Drop` phase per second), so the
/// whole way from standing to flat takes about 1.6 s, eased at both ends.
const DROP_RATE: f32 = 0.8;
/// The drop eases in and out: its pace is this share of `DROP_RATE` at the ends, full in the middle (a body
/// that starts and finishes its fall slowly reads as a person, not a snap).
const DROP_EASE_ENDS: f32 = 0.4;
/// One magazine manipulation is paced over the nominal reload duration, independently of gait.
const RELOAD_ANIMATION_S: f32 = 2.5;
/// Where on the drop each stance sits.
fn drop_at(s: Stance) -> f32 {
    match s {
        Stance::Stand => 0.0,
        Stance::Crouch => pose::DROP_KNEEL,
        Stance::Prone => 1.0,
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Stance {
    Stand,
    Crouch,
    Prone,
}

/// What the rifle is doing.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Rifle {
    /// In the hands at combat ready (low ready on an athletic base).
    Ready,
    /// Shouldered on a mark.
    Aim,
    /// Changing magazines.
    Reload,
    /// On the sling, hands free.
    Slung,
}

/// Where the figure wants to face.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Face {
    /// Nowhere in particular: he stays as he is.
    Keep,
    /// The way he goes or has been told to look; the turn rate is capped at `top` rad/s.
    Turn { heading: f32, top: f32 },
    /// The rifle on a mark: the barrel keeps pace with it and settles with a short eased pivot.
    Aim { heading: f32 },
}

/// What a controller tells the animator each frame.
#[derive(Copy, Clone, Debug)]
pub struct Drive {
    /// Seconds since the last frame, and the shader's clock (s).
    pub dt: f32,
    pub ts: f32,
    /// Real velocity over the ground (m/s, world x/y).
    pub vel: (f32, f32),
    /// The velocity he is trying for (for skids); None when the controller does not know (no skids then).
    pub want: Option<(f32, f32)>,
    pub face: Face,
    pub stance: Stance,
    pub rifle: Rifle,
    /// Allowed to sprint (sprint legs need this as well as the pace).
    pub sprint: bool,
    /// Aiming from cover (crouched aim over a wall).
    pub in_cover: bool,
    /// Shot down (alive, on the ground) or killed.
    pub down: bool,
    pub dead: bool,
    /// An action that takes the whole body over (tablet, radio, throw, first aid, a step the leader has him on,
    /// dragging...): its pose, cadence (steps/s, 0 still) and an optional upper pose over moving legs.
    pub action: Option<(Pose, f32, Option<Pose>)>,
}

impl Drive {
    pub fn new(dt: f32, ts: f32) -> Drive {
        Drive { dt, ts, vel: (0.0, 0.0), want: None, face: Face::Keep, stance: Stance::Stand, rifle: Rifle::Ready, sprint: true, in_cover: false, down: false, dead: false, action: None }
    }
}

/// What is drawn this frame.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Frame {
    pub pose: Pose,
    pub upper: Option<Pose>,
    /// The way he moves relative to the way he faces (rad).
    pub dir: f32,
    /// Gait phase (0..1) and whether the shader should animate the idle (cadence 0) or take the phase (-1).
    pub phase: f32,
    pub cadence: f32,
    pub heading: f32,
    pub speed: f32,
    /// Bank into turns and pitch with acceleration, packed for the colour's fraction (`pose::pack_tilt`).
    pub lean: f32,
    pub skid: Skid,
    /// A foot came down this frame: how loud (sprint 1, run 0.8, walk 0.5, crouched 0.35).
    pub footfall: Option<f32>,
}

#[derive(Copy, Clone, Debug)]
pub struct Animator {
    pub heading: f32,
    pub spin: f32,
    /// How far he still has to turn (rad), and how long he has been turning hard one way (s): small corrections
    /// (a man scanning his sector) are a twist of the body, not a shuffle of the feet.
    gap: f32,
    turning: f32,
    aim: AimTrack,
    speed: f32,
    dir: f32,
    accel: f32,
    /// The bank into turns as drawn (rad): it follows the turn smoothly and at a limited rate.
    bank: f32,
    phase: f32,
    /// Magazine manipulation progress (0..1), deliberately independent of the leg stride phase.
    reload_phase: f32,
    reloading: bool,
    legs: Pose,
    /// The pose drawn, the one before it and when the switch started (shader blend, `pose::switch_start`).
    shown: (Pose, Pose, f32),
    /// The effective upper pose, the one before it and when it changed (layer blend).
    upper: (Pose, Pose, f32),
    /// Falling: since when (s, shader clock) and how.
    fall: Option<(f32, Pose)>,
    on_back: bool,
    was_up: bool,
    clock: f32,
    /// The stance he was last drawn in, and where he is on the drop while going down to prone or getting up
    /// (None when settled).
    stance: Stance,
    drop: Option<f32>,
}

impl Default for Animator {
    fn default() -> Self {
        Animator::new(0.0)
    }
}

fn wrap(mut d: f32) -> f32 {
    while d > std::f32::consts::PI {
        d -= std::f32::consts::TAU;
    }
    while d < -std::f32::consts::PI {
        d += std::f32::consts::TAU;
    }
    d
}

impl Animator {
    pub fn new(heading: f32) -> Animator {
        Animator {
            heading,
            spin: 0.0,
            gap: 0.0,
            turning: 0.0,
            aim: AimTrack::new(heading, 0.0),
            speed: 0.0,
            dir: 0.0,
            accel: 0.0,
            bank: 0.0,
            phase: 0.0,
            reload_phase: 0.0,
            reloading: false,
            legs: Pose::Stand,
            shown: (Pose::Ready, Pose::Ready, -1000.0),
            upper: (Pose::Ready, Pose::Ready, -1000.0),
            fall: None,
            on_back: false,
            was_up: true,
            clock: 0.0,
            stance: Stance::Stand,
            drop: None,
        }
    }

    /// A figure whose stride starts at `phase` (0..1), so a squad does not walk in step.
    pub fn with_phase(heading: f32, phase: f32) -> Animator {
        Animator { phase: phase.fract(), ..Animator::new(heading) }
    }

    /// The pose drawn now.
    pub fn shown(&self) -> Pose {
        self.shown.0
    }

    fn steer(&mut self, face: Face, dt: f32) {
        match face {
            Face::Aim { heading } => {
                self.gap = wrap(heading - self.heading);
                let mut a = self.aim;
                a.heading = self.heading;
                a.spin = self.spin;
                a.step(heading, dt);
                self.aim = a;
                self.heading = a.heading;
                self.spin = a.spin;
            }
            other => {
                self.aim = AimTrack::new(self.heading, self.spin);
                let want = match other {
                    Face::Turn { heading, top } => {
                        let d = wrap(heading - self.heading);
                        self.gap = d;
                        // The rate that would stop exactly on the target at full deceleration, capped; the last few
                        // degrees close smoothly (the square root alone is so steep near zero that, stepped per
                        // frame, it overshoots both ways every frame: a buzz, banked by speed squared at a sprint).
                        (d.signum() * (2.0 * TURN_ACCEL * d.abs()).sqrt().min(STEER_SETTLE * d.abs())).clamp(-top, top)
                    }
                    _ => {
                        self.gap = 0.0;
                        0.0
                    }
                };
                self.spin += (want - self.spin).clamp(-TURN_ACCEL * dt, TURN_ACCEL * dt);
                self.heading = wrap(self.heading + self.spin * dt);
            }
        }
    }

    pub fn step(&mut self, d: &Drive) -> Frame {
        let dt = d.dt.clamp(0.0, 0.1);
        self.clock = d.ts;
        let alive_up = !d.dead && !d.down;
        let speed = (d.vel.0 * d.vel.0 + d.vel.1 * d.vel.1).sqrt();
        if dt > 0.0 {
            self.accel += ((speed - self.speed) / dt - self.accel) * (dt * ACCEL_FOLLOW).min(1.0);
        }
        let was_still = self.speed < WALK_DOWN;
        self.speed = speed;
        let moving = speed > WALK_DOWN;
        let gait = |p: Pose| matches!(p, Pose::Walk | Pose::Run | Pose::Sprint);
        // Still pushing (a reversal passes through a standstill) or let go and coming to a stop.
        let wants = d.want.is_some_and(|w| w.0 * w.0 + w.1 * w.1 > 0.01);
        let stopping = d.want.is_some() && !wants && moving;
        // Through a quick reversal his feet keep going: no drop to standing, no fresh first step.
        let pressing_on = wants && gait(self.legs);
        let skid = if alive_up && d.stance == Stance::Stand { d.want.map_or(Skid::None, |w| skidding(d.vel, w)) } else { Skid::None };
        if alive_up {
            self.steer(d.face, dt);
        } else {
            self.spin = 0.0;
        }
        let travel = (-d.vel.0).atan2(d.vel.1);
        let hard = self.spin.abs() > SHUFFLE_SPIN;
        self.turning = if hard { self.turning + dt } else { 0.0 };
        let shuffling = alive_up && !moving && hard && (self.gap.abs() > 0.35 || self.turning > 0.3) && d.stance == Stance::Stand && d.action.is_none();
        let crawl_blending = self.shown.0 == Pose::Crawl
            || (self.shown.1 == Pose::Crawl && d.ts - self.shown.2 < pose::BLEND_S);
        self.dir = if moving {
            let target = wrap(travel - self.heading);
            // Setting off he steps straight that way; on the move (a reversal included) his legs swing round to
            // the new direction over a few frames rather than swapping a forward stride for a backward one.
            if self.legs == Pose::Stand { target } else { wrap(self.dir + wrap(target - self.dir).clamp(-DIR_RATE * dt, DIR_RATE * dt)) }
        } else if pressing_on {
            self.dir
        } else if shuffling {
            self.spin.signum() * std::f32::consts::FRAC_PI_2
        } else if d.stance == Stance::Prone && crawl_blending {
            // The outgoing crawl is still drawn. Its frozen phase also needs its last travel direction.
            self.dir
        } else {
            0.0
        };
        // Legs by real pace, with hysteresis.
        let was = self.legs;
        let fast = matches!(was, Pose::Run | Pose::Sprint);
        let paced = if d.sprint && speed > if was == Pose::Sprint { SPRINT_DOWN } else { SPRINT_UP } {
            Pose::Sprint
        } else if speed > if fast { RUN_DOWN } else { RUN_UP } {
            Pose::Run
        } else if speed > if matches!(was, Pose::Walk | Pose::Run | Pose::Sprint) { WALK_DOWN } else { WALK_UP } {
            Pose::Walk
        } else {
            Pose::Stand
        };
        // Coming to a stop he keeps the stride he has (slowing) and settles once, not stepping down through each
        // gait (each change restarts a blend, and a chain of them jerks); pushing on through a reversal he keeps
        // walking.
        self.legs = if stopping && gait(was) {
            if speed > STOP_SETTLE { was } else { Pose::Stand }
        } else if pressing_on && paced == Pose::Stand {
            Pose::Walk
        } else {
            paced
        };
        // Setting off from still, he steps out with a foot straight away (not from the middle of a stride).
        if was_still && was == Pose::Stand && moving && !(d.stance == Stance::Prone && crawl_blending) {
            self.phase = 0.12;
        }
        let aim = d.rifle == Rifle::Aim;
        let reload = d.rifle == Rifle::Reload;
        if reload {
            if !self.reloading {
                self.reload_phase = 0.0;
            } else {
                // Off-screen LOD may supply a bounded 200 ms tick; retain that elapsed time here.
                self.reload_phase = (self.reload_phase + d.dt.clamp(0.0, 0.2) / RELOAD_ANIMATION_S).min(1.0);
            }
        }
        self.reloading = reload;
        // Falls: shot down or killed, he goes down once (back or forward) before he lies there.
        if self.was_up && !alive_up {
            self.fall = Some((d.ts, if d.dead { Pose::FallForward } else { Pose::FallBack }));
        }
        if alive_up {
            self.fall = None;
            self.on_back = false;
        } else if !d.dead {
            self.on_back = true;
        }
        self.was_up = alive_up;
        let falling = self.fall.and_then(|(t0, p)| {
            let t = (d.ts - t0) / if p == Pose::FallBack { FALL_BACK_S } else { FALL_FORWARD_S };
            (t < 1.0).then_some((p, t))
        });
        // Going down to prone or getting up from it: he travels along the drop at his own pace (reversing from
        // wherever he is if told the other way). Stand and crouch change by the plain blend.
        let settled = alive_up && falling.is_none() && d.action.is_none();
        if !settled {
            self.drop = None;
            self.stance = d.stance;
        } else if d.stance != self.stance {
            if self.drop.is_some() || d.stance == Stance::Prone || self.stance == Stance::Prone {
                let from = self.drop.unwrap_or(drop_at(self.stance));
                self.drop = Some(from);
            }
            self.stance = d.stance;
        }
        let mut landed = None;
        if let Some(at) = self.drop {
            let to = drop_at(d.stance);
            let x = at.clamp(0.0, 1.0);
            let ease = DROP_EASE_ENDS + (1.0 - DROP_EASE_ENDS) * 4.0 * x * (1.0 - x);
            let step = DROP_RATE * ease * dt;
            let next = if at < to { (at + step).min(to) } else { (at - step).max(to) };
            if next == to {
                self.drop = None;
                landed = Some(match d.stance { Stance::Stand => Pose::Ready, Stance::Crouch => Pose::Crouch, Stance::Prone => Pose::Prone });
            } else {
                self.drop = Some(next);
            }
        }
        let dropping = self.drop;
        let stride_m_dir = pose::stride_m_dir;
        let carried = |legs: Pose| match d.rifle {
            Rifle::Aim => Some(Pose::StandAim),
            Rifle::Reload => Some(Pose::Reload),
            Rifle::Slung => Some(Pose::Slung),
            // Rifle in hand on the move he carries it and works his arms (the legs' own carry).
            Rifle::Ready => (legs == Pose::Stand).then_some(Pose::Ready),
        };
        let (pose, upper, mut steps) = if let Some((p, _)) = falling {
            (p, None, 0.0)
        } else if dropping.is_some() {
            (Pose::Drop, None, 0.0)
        } else if d.dead {
            (if self.on_back { Pose::DeadBack } else { Pose::Dead }, None, 0.0)
        } else if d.down {
            (Pose::Wounded, None, 0.3)
        } else if let Some((p, cadence, up)) = d.action {
            if moving && up.is_some() {
                let legs = if d.stance == Stance::Stand { Pose::Walk } else { Pose::CrouchWalk };
                (legs, up, speed / stride_m_dir(legs, self.dir))
            } else {
                (p, None, cadence)
            }
        } else {
            match (d.stance, moving) {
                (Stance::Prone, true) => (Pose::Crawl, aim.then_some(Pose::ProneAim), speed / stride_m_dir(Pose::Crawl, self.dir)),
                (Stance::Prone, false) => (Pose::Prone, aim.then_some(Pose::ProneAim), 0.0),
                (Stance::Crouch, true) => (Pose::CrouchWalk, if aim { Some(Pose::CrouchAim) } else if reload { Some(Pose::Reload) } else { None }, speed / stride_m_dir(Pose::CrouchWalk, self.dir)),
                (Stance::Crouch, false) => (if reload { Pose::Reload } else if aim { Pose::CoverAim /* aimed from a kneel, in cover or not; the half crouch is only the moving layer */ } else { Pose::Crouch }, None, if reload { 0.4 } else { 0.0 }),
                (Stance::Stand, _) if skid == Skid::Stop || stopping && self.shown.0 == Pose::Brake && speed > STOP_SETTLE => (Pose::Brake, None, 0.0),
                // In a cut he drops low to pivot while his body still slides the old way.
                (Stance::Stand, _) if skid == Skid::Cut => (Pose::Crouch, None, 0.0),
                (Stance::Stand, _) if self.legs == Pose::Stand && shuffling => (Pose::Walk, carried(Pose::Stand), (self.spin.abs() * 0.45).clamp(0.8, 1.8)),
                (Stance::Stand, _) if self.legs == Pose::Stand => (if reload { Pose::Reload } else { carried(Pose::Stand).unwrap_or(Pose::Ready) }, None, if reload { 0.4 } else { 0.0 }),
                // Sprinting the rifle is across the chest, whatever was asked.
                (Stance::Stand, _) if self.legs == Pose::Sprint => (Pose::Sprint, if d.rifle == Rifle::Slung { Some(Pose::Slung) } else { None }, speed / stride_m_dir(Pose::Sprint, self.dir)),
                (Stance::Stand, _) => (self.legs, carried(self.legs), speed / stride_m_dir(self.legs, self.dir)),
            }
        };
        if falling.is_some() {
            steps = 0.0;
        }
        if self.shown.0 != pose {
            self.shown = if let (Pose::Drop, Some(end)) = (self.shown.0, landed) {
                // Off the drop: it ended exactly on this stance's settled pose, so blend on from that (whatever
                // the phase) rather than from the drop.
                (pose, end, d.ts)
            } else {
                (pose, self.shown.0, pose::switch_start(self.shown.1, self.shown.2, pose, d.ts))
            };
        }
        // Both prone bases share one raise/lower state: starting or stopping the feet must not restart it.
        let eff = upper.unwrap_or(if pose == Pose::Crawl { Pose::Prone } else { pose });
        if eff != self.upper.0 {
            let elapsed = (d.ts - self.upper.2).clamp(0.0, LAYER_BLEND_S);
            let reversing_prone_aim = matches!((self.upper.0, eff),
                (Pose::Prone, Pose::ProneAim) | (Pose::ProneAim, Pose::Prone))
                && eff == self.upper.1 && elapsed < LAYER_BLEND_S;
            let since = if reversing_prone_aim { d.ts - (LAYER_BLEND_S - elapsed) } else { d.ts };
            self.upper = (eff, self.upper.0, since);
        }
        // The phase is integrated here (cadence changes as he winds up and brakes) and handed to the shader as is.
        let before = self.phase;
        self.phase = (self.phase + steps * dt).fract();
        let after = self.phase;
        let crossed = |at: f32| if after >= before { before < at && after >= at } else { before < at || after >= at };
        let footfall = (steps > 0.0 && matches!(pose, Pose::Walk | Pose::Run | Pose::Sprint | Pose::CrouchWalk) && (crossed(0.0001) || crossed(0.5)))
            .then(|| match pose { Pose::Sprint => 1.0, Pose::Run => 0.8, Pose::CrouchWalk => 0.35, _ => 0.5 });
        let (phase, cadence) = match (falling, dropping) {
            (Some((_, t)), _) => (t, -1.0),
            (None, Some(at)) => (at, -1.0),
            // The phase is always handed over (frozen when he stops) so legs blending out of a stride stay put;
            // once settled the shader takes over the idle.
            (None, None) => (self.phase, if steps == 0.0 && d.ts - self.shown.2 > 0.4 { 0.0 } else { -1.0 }),
        };
        let up = alive_up && falling.is_none();
        // Upright momentum tilt pivots around the feet; on the ground it would lift or sink the whole rig.
        let tilting = up && d.stance != Stance::Prone && dropping.is_none();
        let bank_to = if tilting { pose::bank(speed, self.spin) } else { 0.0 };
        self.bank += ((bank_to - self.bank) * (dt * BANK_FOLLOW).min(1.0)).clamp(-BANK_RATE * dt, BANK_RATE * dt);
        let lean = if tilting { pose::pack_tilt(self.bank, pose::accel_pitch(self.accel)) } else { 0.0 };
        Frame { pose, upper, dir: self.dir, phase, cadence, heading: self.heading, speed, lean, skid, footfall }
    }

    /// Going down to prone or getting up from it (on the drop): his feet are planted.
    pub fn dropping(&self) -> bool {
        self.drop.is_some()
    }

    /// The shader's instance record for this frame (scene.wgsl): `colour` is the palette index with the leader
    /// (+32) and selection (+64 per level) flags; `kick` is the recoil pressure, `flinch` how hard he is flinching
    /// from fire (both 0 to 1); `fade` dissolves
    /// him.
    pub fn record(&self, f: &Frame, at: (f32, f32, f32), colour: f32, kick: f32, flinch: f32, fade: f32) -> [f32; 12] {
        let remaining = 1.0 - ((self.clock - self.upper.2) / LAYER_BLEND_S).clamp(0.0, 1.0);
        // On the drop the shader is handed its two keys around the phase as pose and previous pose, with the blend
        // set to how far between them he is (pose.rs `drop_keys`); a drop interrupted mid-way blends from the kneel.
        let (pose, prev, since) = match (f.pose, self.shown.1) {
            (Pose::Drop, _) => {
                let (from, to, u) = pose::drop_keys(f.phase);
                (to, from, self.clock - u * pose::BLEND_S)
            }
            (p, Pose::Drop) => (p, Pose::Crouch, self.shown.2),
            (p, before) => (p, before, self.shown.2),
        };
        // Below -1, the cadence slot also carries the independent reload phase for the shader.
        let base_reload_blending_out = self.shown.1 == Pose::Reload && self.clock - self.shown.2 < pose::BLEND_S;
        let upper_reload_blending_out = self.upper.1 == Pose::Reload && self.clock - self.upper.2 < LAYER_BLEND_S;
        let reload_phase_active = f.pose == Pose::Reload || f.upper == Some(Pose::Reload)
            || base_reload_blending_out || upper_reload_blending_out;
        let cadence = if reload_phase_active { -1.0 - self.reload_phase.clamp(0.0, 0.9999) } else { f.cadence };
        let previous_upper = if matches!(f.pose, Pose::Prone | Pose::Crawl) {
            // Unaimed prone uses the base as drawn, not a second copy of the idle over the crawl.
            (self.upper.1 == Pose::ProneAim).then_some(Pose::ProneAim)
        } else if f.pose == Pose::Drop && matches!(self.upper.1, Pose::Prone | Pose::ProneAim) {
            None
        } else {
            Some(self.upper.1)
        };
        let remaining = if previous_upper.is_none() && f.upper.is_none() { 0.0 } else { remaining };
        [
            at.0,
            at.1,
            at.2,
            colour + f.lean,
            f.heading,
            f.phase,
            cadence,
            pose::pack_kick(pose, kick),
            pose::pack_flinch(prev, flinch),
            since,
            pose::pack_layer(f.upper, f.dir),
            pose::pack_fade_layer(fade, previous_upper, remaining),
        ]
    }

    /// His muzzle and bore in the world (mm and unit vector), posed exactly as drawn; None when he cannot fire.
    pub fn barrel(&self, f: &Frame, at_mm: (f32, f32)) -> (Option<[f32; 3]>, [f32; 3]) {
        let (q, b) = pose::barrel_with_phases(pose::RIFLE, f.pose, f.upper, f.phase, self.reload_phase, f.dir);
        let (roll, pitch) = pose::unpack_tilt(9.0 + f.lean);
        let bore = pose::figure_to_world(b, glam::Vec3::ZERO, f.heading, roll, pitch).to_array();
        let armed = pose::two_handed(f.upper.unwrap_or(f.pose)) > 0.0;
        let tip = armed.then(|| pose::figure_to_world(q * 1000.0, glam::Vec3::new(at_mm.0, at_mm.1, 0.0), f.heading, roll, pitch).to_array());
        (tip, bore)
    }
}

#[cfg(test)]
mod tests {

    /// Steps him at 60 fps for `secs` in `stance`, returning the poses and drop phases seen.
    fn hold(a: &mut Animator, ts: &mut f32, stance: Stance, secs: f32) -> Vec<(Pose, f32)> {
        let mut seen = Vec::new();
        for _ in 0..(secs * 60.0) as usize {
            let mut d = drive(*ts);
            d.stance = stance;
            let f = a.step(&d);
            seen.push((f.pose, f.phase));
            *ts += d.dt;
        }
        seen
    }

    #[test]
    fn going_prone_kneels_plants_and_lies_down_then_gets_back_up() {
        let mut a = Animator::new(0.0);
        let mut ts = 0.0;
        hold(&mut a, &mut ts, Stance::Stand, 1.0);
        let down = hold(&mut a, &mut ts, Stance::Prone, 2.2);
        let drop: Vec<f32> = down.iter().filter(|(p, _)| *p == Pose::Drop).map(|(_, t)| *t).collect();
        let secs = drop.len() as f32 / 60.0;
        assert!((1.4..1.9).contains(&secs), "stand to flat in about 1.6 s, not a snap: {secs}");
        // Eased: the first and last tenth of the drop move slower than the middle.
        let step: Vec<f32> = drop.windows(2).map(|w| w[1] - w[0]).collect();
        let n = step.len();
        let mid = step[n / 2 - 5..n / 2 + 5].iter().sum::<f32>() / 10.0;
        let start = step[..n / 10].iter().sum::<f32>() / (n / 10) as f32;
        assert!(start < 0.8 * mid, "the drop starts eased: start {start:.4} mid {mid:.4}");
        assert!(drop.windows(2).all(|w| w[1] > w[0]), "down the drop, never back");
        assert_eq!(down.last().unwrap().0, Pose::Prone);
        // Landing blends on from prone itself, not from the drop at some other phase.
        assert_eq!(a.shown.1, Pose::Prone);
        let up = hold(&mut a, &mut ts, Stance::Stand, 2.2);
        let rise: Vec<f32> = up.iter().filter(|(p, _)| *p == Pose::Drop).map(|(_, t)| *t).collect();
        assert!(rise.windows(2).all(|w| w[1] < w[0]), "up the drop backwards");
        assert!(!rise.is_empty() && rise[0] > 0.9);
        assert_eq!(up.last().unwrap().0, Pose::Ready);
    }

    #[test]
    fn crouching_joins_the_drop_at_the_knee_and_a_change_of_mind_turns_back() {
        let mut a = Animator::new(0.0);
        let mut ts = 0.0;
        hold(&mut a, &mut ts, Stance::Crouch, 1.0);
        let down = hold(&mut a, &mut ts, Stance::Prone, 0.3);
        let first = down.iter().find(|(p, _)| *p == Pose::Drop).unwrap().1;
        assert!((first - pose::DROP_KNEEL).abs() < 0.05, "from the knee: {first}");
        let mid = down.last().unwrap().1;
        // Told to get up half way down: he turns back from where he is.
        let back = hold(&mut a, &mut ts, Stance::Crouch, 1.0);
        assert!((back[0].1 - mid).abs() < 0.05, "no jump: {} -> {}", mid, back[0].1);
        assert_eq!(back.last().unwrap().0, Pose::Crouch);
        // Standing up from a crouch is the plain blend, no drop.
        assert!(hold(&mut a, &mut ts, Stance::Stand, 0.5).iter().all(|(p, _)| *p != Pose::Drop));
    }

    use super::*;

    fn run(a: &mut Animator, frames: usize, mut d: Drive) -> Frame {
        let mut f = a.step(&d);
        for _ in 1..frames {
            d.ts += d.dt;
            f = a.step(&d);
        }
        f
    }

    fn drive(ts: f32) -> Drive {
        Drive::new(1.0 / 60.0, ts)
    }

    #[test]
    fn prone_aim_and_fire_work_at_rest_and_over_the_crawl_gait() {
        let mut a = Animator::new(0.0);
        let mut ts = 0.0;
        hold(&mut a, &mut ts, Stance::Prone, 2.2);

        let mut d = drive(ts);
        d.stance = Stance::Prone;
        d.rifle = Rifle::Aim;
        d.face = Face::Aim { heading: 0.4 };
        let f = run(&mut a, 5, d);
        assert_eq!((f.pose, f.upper), (Pose::Prone, Some(Pose::ProneAim)));
        assert!(a.barrel(&f, (0.0, 0.0)).0.is_some(), "the prone rifle has a firing muzzle");

        d.ts = ts + 0.2;
        d.vel = (0.0, 1.0);
        d.want = Some(d.vel);
        let moving = run(&mut a, 5, d);
        assert_eq!(moving.pose, Pose::Crawl);
        assert_eq!(moving.upper, Some(Pose::ProneAim), "aim stays on the upper body while the legs crawl");
        assert!(a.barrel(&moving, (0.0, 0.0)).0.is_some(), "the layered prone aim keeps its muzzle");
    }

    #[test]
    fn aimed_crawl_stops_and_restarts_without_switching_its_upper_body_or_resetting_its_stride() {
        let mut a = Animator::new(0.0);
        let mut ts = 0.0;
        hold(&mut a, &mut ts, Stance::Prone, 2.2);
        let mut d = drive(ts);
        d.stance = Stance::Prone;
        d.rifle = Rifle::Aim;
        d.vel = (1.05, 0.0);
        let advance = |a: &mut Animator, d: &mut Drive| {
            d.ts += d.dt;
            a.step(d)
        };
        for _ in 0..22 { advance(&mut a, &mut d); }
        let moving = advance(&mut a, &mut d);
        d.vel = (0.0, 0.0);
        let stopped = advance(&mut a, &mut d);
        assert_eq!((moving.pose, stopped.pose), (Pose::Crawl, Pose::Prone));
        assert_eq!((moving.upper, stopped.upper), (Some(Pose::ProneAim), Some(Pose::ProneAim)));
        assert_eq!((stopped.dir, stopped.phase), (moving.dir, moving.phase));
        assert_eq!((moving.lean, stopped.lean), (0.0, 0.0), "no standing momentum tilt on the ground");
        assert_eq!(a.record(&stopped, (0.0, 0.0, 0.0), 0.0, 0.0, 0.0, 0.0)[11], 0.0, "legs stopping must not restart the aim layer");

        d.vel = (1.05, 0.0);
        let resumed = advance(&mut a, &mut d);
        let expected = (stopped.phase + d.dt * 1.05 / pose::stride_m_dir(Pose::Crawl, stopped.dir)).fract();
        assert!((resumed.phase - expected).abs() < 1e-5, "crawl resumes from the frozen stride, not a new step");
    }

    #[test]
    fn reversing_prone_aim_mid_blend_preserves_the_rifle_raise() {
        let mut a = Animator::new(0.0);
        let mut ts = 0.0;
        hold(&mut a, &mut ts, Stance::Prone, 2.2);
        let mut d = drive(ts);
        d.stance = Stance::Prone;
        d.rifle = Rifle::Aim;
        a.step(&d);
        d.ts += 0.10;
        d.rifle = Rifle::Ready;
        let f = a.step(&d);
        let record = a.record(&f, (0.0, 0.0, 0.0), 0.0, 0.0, 0.0, 0.0);
        let (_, old, remaining) = pose::unpack_fade_layer(record[11]);
        assert_eq!(f.upper, None);
        assert_eq!(old, Pose::ProneAim as u32 as f32 + 1.0);
        assert!((remaining - 0.10 / LAYER_BLEND_S).abs() < 0.02, "aim weight jumped on reversal: {remaining}");
    }

    #[test]
    fn standing_in_the_fight_he_is_combat_ready_and_aims_when_told() {
        let mut a = Animator::new(0.0);
        assert_eq!(run(&mut a, 5, drive(0.0)).pose, Pose::Ready);
        let mut d = drive(1.0);
        d.rifle = Rifle::Aim;
        d.face = Face::Aim { heading: 0.0 };
        assert_eq!(run(&mut a, 5, d).pose, Pose::StandAim);
        d.rifle = Rifle::Slung;
        d.face = Face::Keep;
        assert_eq!(run(&mut a, 5, d).pose, Pose::Slung);
    }

    #[test]
    fn moving_reload_has_its_own_phase_and_restarts_cleanly() {
        let (mut slow, mut fast) = (Animator::new(0.0), Animator::new(0.0));
        let (mut a, mut b) = (drive(0.0), drive(0.0));
        a.rifle = Rifle::Reload;
        b.rifle = Rifle::Reload;
        a.vel = (0.0, 1.0);
        b.vel = (0.0, 3.0);
        let (mut fa, mut fb) = (slow.step(&a), fast.step(&b));
        for k in 1..90 {
            a.ts = k as f32 / 60.0;
            b.ts = a.ts;
            fa = slow.step(&a);
            fb = fast.step(&b);
        }
        assert_eq!(fa.upper, Some(Pose::Reload));
        assert_eq!(fb.upper, Some(Pose::Reload));
        assert!((slow.reload_phase - fast.reload_phase).abs() < 1e-6, "reload phase follows elapsed time, not stride");
        assert!((fa.phase - fb.phase).abs() > 0.1, "the two leg cycles remain independent");
        let record = slow.record(&fa, (0.0, 0.0, 0.0), 0.0, 0.0, 0.0, 0.0);
        assert!((record[6] - (-1.0 - slow.reload_phase)).abs() < 1e-5, "shader receives the reload clock");

        a.rifle = Rifle::Ready;
        for k in 90..102 {
            a.ts = k as f32 / 60.0;
            fa = slow.step(&a);
        }
        let fading = slow.record(&fa, (0.0, 0.0, 0.0), 0.0, 0.0, 0.0, 0.0);
        assert!(fading[6] < -1.0, "the reload phase survives the upper-body blend-out");

        for k in 102..130 {
            a.ts = k as f32 / 60.0;
            fa = slow.step(&a);
        }
        let settled = slow.record(&fa, (0.0, 0.0, 0.0), 0.0, 0.0, 0.0, 0.0);
        assert_eq!(settled[6], fa.cadence, "normal gait/idle cadence returns after the blend");

        a.rifle = Rifle::Reload;
        a.ts = 140.0 / 60.0;
        slow.step(&a);
        assert_eq!(slow.reload_phase, 0.0, "a new magazine change starts at its first key");
    }
    #[test]
    fn legs_follow_real_pace_with_give_either_way() {
        let mut a = Animator::new(0.0);
        let mut d = drive(0.0);
        d.vel = (0.0, 1.2);
        assert_eq!(run(&mut a, 3, d).pose, Pose::Walk);
        d.vel = (0.0, 2.3);
        assert_eq!(run(&mut a, 3, d).pose, Pose::Walk, "short of the run line");
        d.vel = (0.0, 2.6);
        assert_eq!(run(&mut a, 3, d).pose, Pose::Run);
        d.vel = (0.0, 2.3);
        assert_eq!(run(&mut a, 3, d).pose, Pose::Run, "easing off a touch keeps him running");
        d.vel = (0.0, 4.4);
        assert_eq!(run(&mut a, 3, d).pose, Pose::Sprint);
        d.sprint = false;
        assert_eq!(run(&mut a, 3, d).pose, Pose::Run, "no sprint without leave to sprint");
    }

    #[test]
    fn soldiers_and_the_commander_get_the_same_frame_from_the_same_drive() {
        let (mut a, mut b) = (Animator::new(0.3), Animator::new(0.3));
        let mut d = drive(0.0);
        d.vel = (1.0, 2.0);
        d.want = Some((1.0, 2.0));
        d.face = Face::Turn { heading: 1.0, top: 6.0 };
        d.rifle = Rifle::Aim;
        for k in 0..90 {
            d.ts = k as f32 / 60.0;
            assert_eq!(a.step(&d), b.step(&d));
        }
    }

    #[test]
    fn turning_on_the_spot_he_shuffles_his_feet() {
        let mut a = Animator::new(0.0);
        let mut d = drive(0.0);
        d.face = Face::Turn { heading: 2.5, top: 6.0 };
        let f = run(&mut a, 12, d);
        assert_eq!(f.pose, Pose::Walk, "stepping round");
        assert_eq!(f.upper, Some(Pose::Ready), "rifle held at ready while he steps round");
        assert!(f.dir.abs() > 1.0, "side steps into the turn");
        let f = run(&mut a, 120, d);
        assert_eq!(f.pose, Pose::Ready, "settled once round");
        assert!((f.heading - 2.5).abs() < 0.02);
    }

    #[test]
    fn a_hard_stop_from_a_sprint_plants_the_brake() {
        let mut a = Animator::new(0.0);
        let mut d = drive(0.0);
        d.vel = (0.0, 4.5);
        d.want = Some((0.0, 4.5));
        run(&mut a, 10, d);
        d.want = Some((0.0, 0.0));
        assert_eq!(run(&mut a, 1, d).pose, Pose::Brake);
    }

    #[test]
    fn shot_down_he_falls_once_then_lies_there() {
        let mut a = Animator::new(0.0);
        run(&mut a, 3, drive(0.0));
        let mut d = drive(1.0);
        d.down = true;
        assert_eq!(a.step(&d).pose, Pose::FallBack);
        assert_eq!(run(&mut a, 90, d).pose, Pose::Wounded);
        d.dead = true;
        d.ts = 5.0;
        assert_eq!(run(&mut a, 3, d).pose, Pose::DeadBack, "died on his back, stays on his back");
    }

    #[test]
    fn setting_off_he_steps_out_and_his_feet_are_heard() {
        let mut a = Animator::new(0.0);
        let mut d = drive(0.0);
        d.vel = (0.0, 1.4);
        let mut heard = 0;
        for k in 0..120 {
            d.ts = k as f32 / 60.0;
            heard += a.step(&d).footfall.is_some() as usize;
        }
        assert!((3..=6).contains(&heard), "about two steps a second: {heard}");
    }

    #[test]
    fn the_drop_reaches_the_shader_as_two_keys_and_a_blend() {
        use scalia_render::pose::{drop_keys, BLEND_S};
        let mut a = Animator::new(0.0);
        run(&mut a, 3, drive(0.0));
        let mut d = drive(1.0);
        d.stance = Stance::Prone;
        let mut seen = 0;
        for k in 0..90 {
            d.ts = 1.0 + k as f32 / 60.0;
            let f = a.step(&d);
            if f.pose != Pose::Drop {
                continue;
            }
            seen += 1;
            let r = a.record(&f, (0.0, 0.0, 0.0), 0.0, 0.0, 0.0, 0.0);
            let (from, to, u) = drop_keys(f.phase);
            assert_eq!((r[7] as u32, r[8] as u32), (to as u32, from as u32));
            assert!(((d.ts - r[9]) / BLEND_S - u).abs() < 1e-4, "the blend is how far between the keys");
            assert!(r[7] as u32 != Pose::Drop as u32 && r[8] as u32 != Pose::Drop as u32, "the shader never sees the drop");
        }
        assert!(seen > 20);
    }

    #[test]
    fn crouched_and_still_he_aims_from_a_kneel() {
        let mut a = Animator::new(0.0);
        let mut d = drive(0.0);
        d.stance = Stance::Crouch;
        d.rifle = Rifle::Aim;
        let mut f = a.step(&d);
        for _ in 0..60 { d.ts += d.dt; f = a.step(&d); }
        let (legs, kneel) = (pose::params(f.pose, 0.0), pose::params(Pose::Crouch, 0.0));
        assert_eq!((legs.hip, legs.knee), (kneel.hip, kneel.knee), "{:?} does not kneel", f.pose);
        assert!(pose::two_handed(f.upper.unwrap_or(f.pose)) > 0.0, "still aiming");
    }

    #[test]
    fn every_soldier_fires_from_the_rifle_that_is_drawn() {
        // Soldiers are drawn as the commander's procedural figure: rounds and flash leave that barrel.
        let mut a = Animator::new(0.0);
        let mut d = drive(0.0);
        d.rifle = Rifle::Aim;
        let mut f = a.step(&d);
        for _ in 0..60 { d.ts += d.dt; f = a.step(&d); }
        let (tip, _) = a.barrel(&f, (0.0, 0.0));
        let (q, _) = pose::barrel_with_phases(pose::RIFLE, f.pose, f.upper, f.phase, a.reload_phase, f.dir);
        let (roll, pitch) = pose::unpack_tilt(9.0 + f.lean);
        let want = pose::figure_to_world(q * 1000.0, glam::Vec3::ZERO, f.heading, roll, pitch);
        let got = glam::Vec3::from_array(tip.expect("armed"));
        assert!((got - want).length() < 1.0, "{got} vs {want}");
    }

    #[test]
    fn every_soldier_steps_at_the_commanders_procedural_stride() {
        // The commander's animation is the reference for every soldier: legs cycle at the procedural stride, not
        // the Mixamo clips' (those stay as reference only).
        let mut a = Animator::new(0.0);
        let mut d = drive(0.0);
        d.vel = (0.0, 1.2);
        d.want = Some(d.vel);
        let mut f = a.step(&d);
        for _ in 0..40 { d.ts += d.dt; f = a.step(&d); }
        assert_eq!(f.pose, Pose::Walk);
        let before = f.phase;
        d.ts += d.dt;
        let after = a.step(&d).phase;
        let expected = (before + d.dt * 1.2 / pose::stride_m_dir(Pose::Walk, f.dir)).fract();
        assert!((after - expected).abs() < 1e-4, "{after} vs {expected}");
    }

    #[test]
    fn gait_changes_keep_stride_phase_continuous_and_shader_phase_enabled() {
        let mut a = Animator::new(0.0);
        let mut d = drive(0.0);
        let mut previous: Option<f32> = None;
        // Cross walk -> run -> sprint -> run -> walk at real pace. Pose changes must not
        // reset the alternating-foot cycle. Moving poses must keep shader phase animation enabled.
        for (frame, speed, sprint, expected) in [
            (0, 1.5, true, Pose::Walk),
            (30, 2.6, true, Pose::Run),
            (60, 4.5, true, Pose::Sprint),
            (90, 4.5, false, Pose::Run),
            (120, 1.5, false, Pose::Walk),
        ] {
            d.vel = (0.0, speed);
            d.sprint = sprint;
            d.ts = frame as f32 / 60.0;
            let f = a.step(&d);
            assert_eq!(f.pose, expected, "frame {frame}");
            assert_eq!(f.cadence, -1.0, "moving gait must use the shader's integrated phase at frame {frame}");
            if let Some(before) = previous {
                let delta = (f.phase - before + 1.0).fract();
                assert!(delta < 0.2, "stride phase jumped at gait transition {frame}: {before} -> {}", f.phase);
            }
            previous = Some(f.phase);
        }
    }

    #[test]
    fn gait_thresholds_do_not_flicker_when_pace_hovers_at_each_boundary() {
        let mut a = Animator::new(0.0);
        let mut d = drive(0.0);
        d.vel = (0.0, 1.0);
        assert_eq!(a.step(&d).pose, Pose::Walk);
        for speed in [0.19, 0.21, 0.18, 0.22, 0.19] {
            d.vel = (0.0, speed);
            d.ts += d.dt;
            assert_eq!(a.step(&d).pose, Pose::Walk, "walk hysteresis at {speed}");
        }
        for speed in [2.41, 2.35, 2.25, 2.39, 2.2] {
            d.vel = (0.0, speed);
            d.ts += d.dt;
            let pose = a.step(&d).pose;
            assert_eq!(pose, Pose::Run, "run hysteresis holds above the down threshold at {speed}");
        }
    }
}

/// S9: going down to prone from a run. His feet stop stepping at once (the drop is planted, no stride), but his
/// body keeps the momentum it had and slides on while he goes down, as a man does who drops mid-stride.
#[cfg(test)]
mod transition_tests {
    use super::*;

    #[test]
    fn going_prone_from_a_run_plants_his_feet_but_keeps_his_momentum_through_the_drop() {
        let dt = 1.0 / 60.0;
        let mut a = Animator::new(0.0);
        let mut ts = 0.0f32;
        let mut speed = 3.0f32;
        let mut dir = (0.0f32, 1.0f32);
        let mut slid = 0.0f32;
        let mut drop_frames = 0;
        for k in 0..(2.0 / dt) as usize {
            let prone = k as f32 * dt >= 1.0;
            let mut d = Drive::new(dt, ts);
            d.stance = if prone { Stance::Prone } else { Stance::Stand };
            d.vel = (dir.0 * speed, dir.1 * speed);
            d.want = Some(if prone { (0.0, 0.0) } else { (0.0, 3.0) });
            d.face = Face::Keep;
            let f = a.step(&d);
            if prone && f.pose == Pose::Drop {
                drop_frames += 1;
                assert!(f.cadence < 0.0, "the drop is planted: no stride cadence while he goes down");
                assert!(f.footfall.is_none(), "no footfall while he goes down");
            }
            if prone {
                // Momentum: his feet can only brake him so fast, towards the crawl pace he is aiming for.
                let vel = if a.dropping() { crate::motion::drop_slide((dir.0 * speed, dir.1 * speed), dt) } else { crate::motion::step_velocity((dir.0 * speed, dir.1 * speed), (0.0, 0.0), dt, crate::motion::CMD_FEET) };
                speed = (vel.0 * vel.0 + vel.1 * vel.1).sqrt();
                if speed > 1e-3 { dir = (vel.0 / speed, vel.1 / speed); }
                if f.pose == Pose::Drop { slid += speed * dt; }
            }
            ts += dt;
        }
        assert!(drop_frames > 10, "the drop lasted {drop_frames} frames");
        assert!(slid > 0.8, "he slid only {slid:.2} m while going down: the momentum was lost at once");
    }
}
