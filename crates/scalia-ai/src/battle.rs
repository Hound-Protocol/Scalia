//! The close battle, soldier by soldier: who can see whom, who shoots, who is hit, who is pinned. Fixed 50 ms
//! steps in soldier order, integer maths and a hashed random stream: the same start gives the same fight on
//! every machine (co-op). The AI layers above decide where soldiers go and how they stand; this decides what
//! happens when they meet.

use crate::sight::{isqrt, Point, SightGrid};
use crate::grid::SoldierGrid;
use crate::leader::{Brain, LEAD_MS};
use crate::save::{decode, encode, frame, unframe, Command, MapSpec};
use crate::tactics::{Building, CoverMap, Height};
use scalia_core::killhouse::Killhouse;
use scalia_core::hash::mix64;
use scalia_core::nav::{find_path, NavGrid, Prop};

/// How he moves: pace and how low.
#[derive(Copy, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Gait {
    Walk,
    Run,
    Crouch,
    Sprint,
    /// Upright but slow and deliberate: moving with caution, weapon up, watching.
    Careful,
}

impl Gait {
    pub fn speed_mm_s(self) -> i64 {
        match self {
            Gait::Walk => 1_500,
            Gait::Run => 3_500,
            Gait::Crouch => 1_000,
            Gait::Sprint => 4_600,
            Gait::Careful => 1_000,
        }
    }
}

/// What his squad leader has told him to do.
#[derive(Copy, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Task {
    Idle,
    Move { to: (i64, i64), gait: Gait },
    Hold { at: (i64, i64), face: (i64, i64) },
}

/// What a soldier calls out (the radio voices and on-screen labels pick these up).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Bark {
    TakingCover,
    Contact,
    Reloading,
    ManDown,
    Frag,
}

/// Something a soldier is busy doing with his hands (the animation plays it; he does not shoot meanwhile).
#[derive(Copy, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Act {
    Throw,
    Drone,
    /// Filling and laying sandbags.
    Build,
}

/// Radius a soldier needs to pass (doorways are 1.2 m and more).
pub(crate) const BODY_MM: i64 = 250;
/// Within this of where he is going he has arrived.
const ARRIVE_MM: i64 = 150;
const PERSONAL_MM: i64 = 700;
/// Two men are never sent to stand closer than this: a shared destination is spread into separate spots.
pub const SPACING_MM: i64 = 900;
/// Once there, a man stays put until pushed this far off his spot.
const SETTLE_MM: i64 = 600;
/// 24 directions, 15° apart, as unit vectors ×1000.
pub(crate) const DIRS_24: [(i64, i64); 24] = [(1000, 0), (966, 259), (866, 500), (707, 707), (500, 866), (259, 966), (0, 1000), (-259, 966), (-500, 866), (-707, 707), (-866, 500), (-966, 259), (-1000, 0), (-966, -259), (-866, -500), (-707, -707), (-500, -866), (-259, -966), (0, -1000), (259, -966), (500, -866), (707, -707), (866, -500), (966, -259)];
/// Winding up and braking (mm/s²): about a second from standing to a run, a little less to stop.
const ACCEL: i64 = 4_000;
const BRAKE: i64 = 5_000;
/// Suppression at which he looks for cover, and below which (with nothing to shoot for a while) he leaves it.
const SEEK_COVER: i32 = 400;
const LEAVE_COVER: i32 = 150;
const CALM_MS: i32 = 3_000;
const COVER_REACH_MM: i64 = 12_000;

#[derive(Copy, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Stance {
    Stand,
    Crouch,
    Prone,
}

impl Stance {
    /// Eye height (mm).
    pub fn eye(self) -> i64 {
        match self {
            Stance::Stand => 1_600,
            Stance::Crouch => 1_000,
            Stance::Prone => 350,
        }
    }
    /// Points on the body a shooter tries to see: legs, chest, head (mm above the ground).
    pub fn body(self) -> [i64; 3] {
        match self {
            Stance::Stand => [500, 1_150, 1_600],
            Stance::Crouch => [400, 750, 1_000],
            Stance::Prone => [150, 250, 350],
        }
    }
    /// How big a target he makes (per mille of a standing man).
    fn size(self) -> i64 {
        match self {
            Stance::Stand => 1_000,
            Stance::Crouch => 750,
            Stance::Prone => 450,
        }
    }
}

pub const MAG: i32 = 30;
pub const DAMAGE: i32 = 35;
/// Squad numbers a command may name (each has a leader record and a contact list).
pub const MAX_SQUADS: usize = 4_096;
/// Fire modes by range: full auto close in, controlled bursts at mid range, single aimed shots beyond.
const AUTO_MM: i64 = 15_000;
const BURST_MM: i64 = 60_000;
/// Between rounds in a burst (about 800 rounds a minute), and the pause to re-aim after a burst or single shot.
const CYCLIC_MS: i32 = 75;
const AFTER_AUTO_MS: i32 = 300;
const AFTER_BURST_MS: i32 = 400;
const AFTER_SINGLE_MS: i32 = 450;
/// Accuracy of each round in a burst relative to the first (‰): recoil climbs the muzzle.
const RECOIL: [i64; 6] = [1_000, 700, 550, 450, 400, 400];
const RELOAD_MS: i32 = 2_500;
const RANGE_MM: i64 = 200_000;
/// Suppression (0..1000) a round adds passing by, and hitting; and how fast it fades per second.
const SUPPRESS_MISS: i32 = 150;
const SUPPRESS_HIT: i32 = 300;
const SUPPRESS_FADE_S: i32 = 400;
/// Perception runs this often.
const PERCEIVE_MS: i32 = 250;
/// The step the perception schedule assumes (each man looks on one step in PERCEIVE_MS / STEP_HINT_MS).
const STEP_HINT_MS: i32 = 50;
/// Per look: at most this many enemies tested for sight (nearest first), and this many kept as seen.
const RAYS_MAX: usize = 24;
const SEE_MAX: usize = 8;
/// Enemies nearer than this are re-checked every look; further ones every FAR_EVERY-th look (once a second),
/// staying in view between checks.
const NEAR_MM: i64 = 50_000;
const FAR_EVERY: i64 = 4;
/// Route searches (A*) started per step; more wait a step. Straight walks need none.
const PATHS_PER_STEP: u32 = 8;
/// Half the view cone, as its cosine (per mille): 120 degrees across.
const CONE_COS: i64 = 500;
/// Anyone this close is noticed whichever way he faces (heard, felt).
const CLOSE_MM: i64 = 3_000;
/// Gunfire is heard this far away.
const HEAR_MM: i64 = 60_000;
/// Contact confidence (per mille) and its fade per second once out of sight, down to a floor (the last known
/// position is never forgotten in a fight).
const CONF_SEEN: i32 = 1_000;
const CONF_SHOT_AT: i32 = 700;
const CONF_HEARD: i32 = 300;
const CONF_FADE_S: i32 = 100;
const CONF_FLOOR: i32 = 100;
/// Turning speed: 4 rad/s, as the cosine and sine (per mille) of the turn in one 50 ms step.
const TURN_COS: i64 = 980;
const TURN_SIN: i64 = 199;

/// What a squad knows of one enemy.
#[derive(Copy, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Contact {
    pub enemy: usize,
    /// Last known position (where he was when last seen, shot from or heard).
    pub pos: (i64, i64),
    pub seen_ms: i64,
    /// 1000 seen right now .. CONF_FLOOR long lost.
    pub confidence: i32,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Soldier {
    pub team: u8,
    pub squad: u16,
    pub pos: (i64, i64),
    pub stance: Stance,
    pub moving: bool,
    pub health: i32,
    pub ammo: i32,
    pub mags: i32,
    /// 0 calm .. 1000 pinned.
    pub suppression: i32,
    pub cooldown_ms: i32,
    pub reloading_ms: i32,
    /// The enemy he is engaging.
    pub target: Option<usize>,
    /// Never shoots (tests, and soldiers told to hold fire).
    pub passive: bool,
    /// Which way he faces: a unit vector scaled to 1000 (north is (0, 1000)).
    pub facing: (i64, i64),
    pub task: Task,
    /// How he is moving now (from the task, or running to cover).
    pub gait: Gait,
    /// The cover spot he has taken or is running to (index into the cover map).
    pub cover: Option<usize>,
    /// How far he moved in the last step (millimetres): the animation's direction and pace.
    pub vel: (i64, i64),
    /// Rifle up while moving or holding (the aim layer over the legs).
    pub aim: bool,
    /// What he is busy with, until when (battle ms).
    pub act: Option<(Act, i64)>,
    /// Rounds left in the burst he is firing, and how many he has fired in it (recoil).
    burst_left: u8,
    burst_fired: u8,
    path: Vec<(i64, i64)>,
    path_to: Option<(i64, i64)>,
    /// Where he is really going: the ordered spot, moved aside if another man already has it.
    goal: Option<(i64, i64)>,
    /// Arrived at his goal: he stands still until his goal changes or he is pushed well off it.
    pub(crate) settled: bool,
    /// The stance his leader set for holding a position (low on a perimeter, low behind low cover).
    pub(crate) posture: Option<Stance>,
    /// Where his leader wants him watching while he moves (a direction), instead of where he walks.
    pub(crate) watch: Option<(i64, i64)>,
    /// Current speed (mm/s): it winds up towards the gait's pace and brakes into the stop.
    speed: i64,
    calm_ms: i32,
}

impl Soldier {
    pub fn new(team: u8, squad: u16, pos: (i64, i64), stance: Stance) -> Soldier {
        Soldier { team, squad, pos, stance, moving: false, health: 100, ammo: MAG, mags: 6, suppression: 0, cooldown_ms: 0, reloading_ms: 0, target: None, passive: false, facing: (0, 1_000), task: Task::Idle, gait: Gait::Walk, cover: None, vel: (0, 0), aim: false, act: None, burst_left: 0, burst_fired: 0, path: Vec::new(), path_to: None, goal: None, settled: false, posture: None, watch: None, speed: 0, calm_ms: 0 }
    }
    pub fn alive(&self) -> bool {
        self.health > 0
    }
    pub fn eye(&self) -> Point {
        Point { x: self.pos.0, y: self.pos.1, z: self.stance.eye() }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// A round fired, at `at_ms` (battle time; several can fall within one step in a burst).
    Shot { shooter: usize, target: usize, from: Point, to: Point, hit: bool, at_ms: i64 },
    Killed { soldier: usize, by: usize },
    Reload { soldier: usize },
    Bark { soldier: usize, bark: Bark },
    /// A squad leader starts a step of his plan.
    Order { squad: usize, step: &'static str },
    Explosion { pos: (i64, i64) },
}

pub struct Battle {
    pub soldiers: Vec<Soldier>,
    pub sight: SightGrid,
    pub seed: u64,
    pub time_ms: i64,
    /// Everything that happened, in order (the renderer and sound read new entries each frame).
    pub log: Vec<Event>,
    /// Each squad's contacts (indexed by squad id).
    pub contacts: Vec<Vec<Contact>>,
    /// Where each squad last heard friendly gunfire (a fight going on), and when.
    pub heard: Vec<Option<((i64, i64), i64)>>,
    /// The enemies each soldier sees right now, nearest first (at most SEE_MAX), refreshed by perception.
    visible: Vec<Vec<u32>>,
    /// Where everyone is (rebuilt each step; not state).
    grid: SoldierGrid,
    /// Where everyone with a spot to reach is going (by spot), for handing out free spots.
    goals: SoldierGrid,
    scratch: Vec<u32>,
    /// Route searches started this step (a budget, so a thousand new orders do not land on one step).
    paths_this_step: u32,
    perceive_ms: i32,
    nav: Option<NavGrid>,
    pub cover: Option<CoverMap>,
    pub(crate) building: Option<(Killhouse, Building)>,
    pub(crate) brains: Vec<Brain>,
    pub(crate) dead_logged: Vec<bool>,
    lead_ms: i64,
    /// The map the grids were built from (a battle without one cannot be saved).
    map: Option<MapSpec>,
    /// Commands waiting for their time, in submission order.
    queue: Vec<(i64, Command)>,
    /// Commands as they took effect: (battle ms of the step, command). Replaying these reproduces the battle.
    history: Vec<(i64, Command)>,
    /// Dice rolled so far: salts each roll (state, so a restored battle rolls the same dice).
    rolls: u64,
}

/// What a save holds: the map and every piece of state that decides what happens next (not the event log,
/// which is output, nor the grids, which are rebuilt from the map).
#[derive(serde::Serialize, serde::Deserialize)]
struct State {
    map: Option<MapSpec>,
    soldiers: Vec<Soldier>,
    seed: u64,
    time_ms: i64,
    contacts: Vec<Vec<Contact>>,
    heard: Vec<Option<((i64, i64), i64)>>,
    visible: Vec<Vec<u32>>,
    perceive_ms: i32,
    brains: Vec<Brain>,
    dead_logged: Vec<bool>,
    lead_ms: i64,
    queue: Vec<(i64, Command)>,
    history: Vec<(i64, Command)>,
    rolls: u64,
}

impl Battle {
    pub fn new(sight: SightGrid, soldiers: Vec<Soldier>, seed: u64) -> Battle {
        let (sight_x0, sight_y0, sight_w, sight_h) = (sight.x_mm, sight.y_mm, sight.w as i64 * 250, sight.h as i64 * 250);
        let squads = soldiers.iter().map(|s| s.squad as usize + 1).max().unwrap_or(0);
        let n = soldiers.len();
        Battle { soldiers, sight, seed, time_ms: 0, log: Vec::new(), contacts: vec![Vec::new(); squads], heard: vec![None; squads], visible: vec![Vec::new(); n], grid: SoldierGrid::new(sight_x0, sight_y0, sight_w, sight_h), goals: SoldierGrid::new(sight_x0, sight_y0, sight_w, sight_h), scratch: Vec::new(), paths_this_step: 0, perceive_ms: 0, nav: None, cover: None, building: None, brains: Vec::new(), dead_logged: vec![false; n], lead_ms: 0, map: None, queue: Vec::new(), history: Vec::new(), rolls: 0 }
    }

    /// Puts a prop on the ground mid-battle (sandbags): it blocks sight and movement and gives cover from now on.
    pub(crate) fn place_prop(&mut self, p: Prop) {
        let Some(map) = self.map.as_mut() else { return };
        map.push_prop(p);
        let props = map.props();
        self.sight.stamp(&p);
        if let Some(nav) = self.nav.as_mut() {
            nav.rebuild(&props);
        }
        if let (Some(cover), Some(nav)) = (self.cover.as_mut(), self.nav.as_ref()) {
            cover.add_prop(props.len() - 1, &p, nav);
        }
        // Routes planned through the new prop are planned again.
        for s in &mut self.soldiers {
            s.path_to = None;
        }
    }

    /// Adds a soldier (use `Command::Spawn` for a battle that is replayed or saved). Returns his index.
    pub fn join(&mut self, s: Soldier) -> usize {
        let sq = s.squad as usize;
        if self.contacts.len() <= sq {
            self.contacts.resize(sq + 1, Vec::new());
            self.heard.resize(sq + 1, None);
        }
        let mut s = s;
        // Never inside a prop: the nearest walkable spot (rings of 250 mm out to 5 m).
        if let Some(n) = self.nav.as_ref() {
            if !n.walkable_at(s.pos.0, s.pos.1, BODY_MM) {
                let p = s.pos;
                let found = (1..=20i64).find_map(|ring| DIRS_24.iter().map(|d| (p.0 + d.0 * ring * 250 / 1_000, p.1 + d.1 * ring * 250 / 1_000)).find(|q| n.walkable_at(q.0, q.1, BODY_MM)));
                if let Some(q) = found {
                    s.pos = q;
                }
            }
        }
        self.soldiers.push(s);
        self.visible.push(Vec::new());
        self.dead_logged.push(false);
        self.soldiers.len() - 1
    }

    /// A battle on a real map: soldiers path around obstacles and take cover.
    pub fn with_map(nav: NavGrid, sight: SightGrid, cover: CoverMap, soldiers: Vec<Soldier>, seed: u64) -> Battle {
        let mut b = Battle::new(sight, soldiers, seed);
        b.nav = Some(nav);
        b.cover = Some(cover);
        b
    }

    /// A battle on a described map (the grids built from it, the training house set if it has one): the kind
    /// of battle that can be saved, restored and replayed.
    pub fn on_map(map: MapSpec, soldiers: Vec<Soldier>, seed: u64) -> Battle {
        let (nav, sight, cover) = map.grids();
        let mut b = Battle::with_map(nav, sight, cover, soldiers, seed);
        if let Some(o) = map.killhouse {
            b.set_building(Killhouse::new(o.0, o.1));
        }
        b.map = Some(map);
        b
    }

    /// Queues a command to take effect at the first step at or after `at_ms` (a late one, at the next step).
    pub fn submit(&mut self, at_ms: i64, c: Command) {
        self.queue.push((at_ms, c));
    }

    /// The commands as they took effect, with the step's time.
    pub fn history(&self) -> &[(i64, Command)] {
        &self.history
    }

    /// Applies the commands that are due, in submission order; ones naming no such soldier are dropped.
    fn apply_commands(&mut self) {
        let now = self.time_ms;
        let (due, wait): (Vec<_>, Vec<_>) = std::mem::take(&mut self.queue).into_iter().partition(|c| c.0 <= now);
        self.queue = wait;
        for (_, c) in due {
            match c {
                Command::Task { soldier, task } if soldier < self.soldiers.len() && self.soldiers[soldier].alive() => self.order(soldier, task),
                Command::Squad { squad, order } if squad < MAX_SQUADS => self.command(squad, order),
                Command::Spawn { team, squad, pos, stance } if (squad as usize) < MAX_SQUADS => {
                    self.join(Soldier::new(team, squad, pos, stance));
                }
                _ => continue,
            }
            self.history.push((now, c));
        }
    }

    fn state(&self) -> State {
        State {
            map: self.map.clone(),
            soldiers: self.soldiers.clone(),
            seed: self.seed,
            time_ms: self.time_ms,
            contacts: self.contacts.clone(),
            heard: self.heard.clone(),
            visible: self.visible.clone(),
            perceive_ms: self.perceive_ms,
            brains: self.brains.clone(),
            dead_logged: self.dead_logged.clone(),
            lead_ms: self.lead_ms,
            queue: self.queue.clone(),
            history: self.history.clone(),
            rolls: self.rolls,
        }
    }

    /// The battle as bytes: versioned, checksummed, restorable with `load`. Panics for a battle built without a
    /// map (`on_map`), which could not be rebuilt.
    pub fn save(&self) -> Vec<u8> {
        assert!(self.map.is_some(), "only a battle built with on_map can be saved");
        frame(&encode(&self.state()))
    }

    /// A battle from `save` bytes; refuses damaged saves and other format versions.
    pub fn load(bytes: &[u8]) -> Result<Battle, String> {
        let st: State = decode(unframe(bytes)?)?;
        let map = st.map.clone().ok_or("save has no map")?;
        let mut b = Battle::on_map(map, st.soldiers, st.seed);
        let n = b.soldiers.len();
        if st.visible.len() != n || st.dead_logged.len() != n {
            return Err("save is inconsistent (soldier tables differ in size)".into());
        }
        b.time_ms = st.time_ms;
        b.contacts = st.contacts;
        b.heard = st.heard;
        b.visible = st.visible;
        b.perceive_ms = st.perceive_ms;
        b.brains = st.brains;
        b.dead_logged = st.dead_logged;
        b.lead_ms = st.lead_ms;
        b.queue = st.queue;
        b.history = st.history;
        b.rolls = st.rolls;
        Ok(b)
    }

    pub(crate) fn state_bytes(&self) -> Vec<u8> {
        encode(&self.state())
    }

    /// The map this battle is on (with any props placed during it), if it was built from one.
    pub fn map_spec(&self) -> Option<&MapSpec> {
        self.map.as_ref()
    }

    pub fn nav(&self) -> Option<&NavGrid> {
        self.nav.as_ref()
    }

    /// Gives soldier `i` a task (his squad leader's job; also the tests').
    pub fn order(&mut self, i: usize, task: Task) {
        self.soldiers[i].task = task;
    }

    /// Where he is going now and how: to his cover if he has taken one, else his task.
    fn destination(&self, i: usize) -> Option<((i64, i64), Gait)> {
        let s = &self.soldiers[i];
        if let (Some(c), Some(map)) = (s.cover, self.cover.as_ref()) {
            return Some((map.spots[c].pos, Gait::Run));
        }
        match s.task {
            Task::Idle => None,
            Task::Move { to, gait } => Some((to, gait)),
            Task::Hold { at, .. } => Some((at, if s.gait == Gait::Run { Gait::Run } else { Gait::Walk })),
        }
    }

    /// The spot nearest `to` (on rings 1 m apart) that is walkable and at least SPACING_MM from every other
    /// living man's goal or, for men without one, where he stands.
    fn free_spot(&self, i: usize, to: (i64, i64)) -> (i64, i64) {
        // Spots that could clash: those of men going to or standing on a spot near `to`, and where the others
        // stand.
        let r = 5 * SPACING_MM;
        let mut near = Vec::new();
        let mut taken: Vec<(i64, i64)> = Vec::new();
        self.goals.near(to, r, &mut near);
        taken.extend(near.iter().map(|&j| j as usize).filter(|&j| j != i).filter_map(|j| self.soldiers[j].goal));
        self.grid.near(to, r, &mut near);
        taken.extend(near.iter().map(|&j| j as usize).filter(|&j| j != i && !Self::claims_goal(&self.soldiers[j])).map(|j| self.soldiers[j].pos));
        let ok = |p: (i64, i64)| {
            self.nav.as_ref().is_none_or(|n| n.walkable_at(p.0, p.1, 0)) && taken.iter().all(|&t| isqrt((t.0 - p.0).pow(2) + (t.1 - p.1).pow(2)) >= SPACING_MM)
        };
        if ok(to) {
            return to;
        }
        // Rings of 6, 12, 24 and 24 spots, from a fixed table of 24 directions (no platform trigonometry).
        for (ring, n) in [(1i64, 6usize), (2, 12), (3, 24), (4, 24)] {
            let r = ring * SPACING_MM;
            for k in 0..n {
                let (c, s) = DIRS_24[k * 24 / n];
                let p = (to.0 + r * c / 1_000, to.1 + r * s / 1_000);
                if ok(p) {
                    return p;
                }
            }
        }
        to
    }

    /// Moves everyone a step along their paths, keeping personal space; sets stance and pace.
    fn move_soldiers(&mut self, dt_ms: i32) {
        let before: Vec<(i64, i64)> = self.soldiers.iter().map(|s| s.pos).collect();
        for i in 0..self.soldiers.len() {
            if !self.soldiers[i].alive() {
                // The dead drop where they are: no motion, no task, nothing in hand.
                let s = &mut self.soldiers[i];
                s.moving = false;
                s.vel = (0, 0);
                s.speed = 0;
                s.task = Task::Idle;
                s.act = None;
                s.path.clear();
                continue;
            }
            let mut dest = self.destination(i);
            // Rifle up and an enemy in sight: stop and shoot, then carry on (not when running for cover).
            let s = &self.soldiers[i];
            if s.aim && s.target.is_some() && s.cover.is_none() && dest.is_some_and(|d| d.1 != Gait::Sprint) {
                dest = None;
            }
            let mut step = (0i64, 0i64);
            if dest.is_none() {
                self.soldiers[i].settled = false;
            }
            if let Some((to, gait)) = dest {
                self.soldiers[i].gait = gait;
                let pos = self.soldiers[i].pos;
                if self.soldiers[i].path_to != Some(to) {
                    let goal = self.free_spot(i, to);
                    let straight = self.nav.as_ref().is_none_or(|n| walk_line(n, pos, goal));
                    let s = &mut self.soldiers[i];
                    // A straight walk needs no route; otherwise the path stays empty until a search is run.
                    s.path = if straight { vec![goal] } else { Vec::new() };
                    s.path_to = Some(to);
                    s.goal = Some(goal);
                    s.settled = false;
                    self.goals.insert(i, goal);
                }
                if self.soldiers[i].path.is_empty() {
                    if self.paths_this_step >= PATHS_PER_STEP {
                        // Over this step's route budget: he waits a step where he is.
                        self.soldiers[i].vel = (0, 0);
                        self.soldiers[i].moving = false;
                        continue;
                    }
                    self.paths_this_step += 1;
                    let goal = self.soldiers[i].goal.unwrap_or(to);
                    let path = self.nav.as_ref().and_then(|n| find_path(n, pos, goal, BODY_MM)).unwrap_or_else(|| vec![goal]);
                    self.soldiers[i].path = path;
                }
                let goal = self.soldiers[i].goal.unwrap_or(to);
                let left = isqrt((goal.0 - pos.0).pow(2) + (goal.1 - pos.1).pow(2));
                let s = &mut self.soldiers[i];
                if left <= 20 {
                    s.settled = true;
                } else if s.settled && left > SETTLE_MM {
                    // Shoved well off his spot: walk back to it.
                    s.settled = false;
                    s.path = vec![goal];
                }
                if !s.settled {
                    while s.path.len() > 1 && isqrt((s.path[0].0 - pos.0).pow(2) + (s.path[0].1 - pos.1).pow(2)) <= ARRIVE_MM {
                        s.path.remove(0);
                    }
                    let aim = s.path.first().copied().unwrap_or(goal);
                    let (ax, ay) = (aim.0 - pos.0, aim.1 - pos.1);
                    let d = isqrt(ax * ax + ay * ay);
                    if d > 0 {
                        // Wind up towards the pace; brake so he stops on the spot (v² = 2·a·d), never below a shuffle.
                        let brake_cap = isqrt(BRAKE * left).max(250);
                        let want = gait.speed_mm_s().min(brake_cap);
                        let dv = if want > s.speed { ACCEL } else { BRAKE } * dt_ms as i64 / 1_000;
                        s.speed = if want > s.speed { (s.speed + dv).min(want) } else { (s.speed - dv).max(want) };
                        let len = (s.speed * dt_ms as i64 / 1_000).max(1).min(d);
                        step = (ax * len / d, ay * len / d);
                    }
                }
            }
            // Personal space: ease away from anyone too close (positions as they were at the start of the step).
            let p = self.soldiers[i].pos;
            let mut push = (0i64, 0i64);
            let mut near = std::mem::take(&mut self.scratch);
            self.grid.near(p, PERSONAL_MM, &mut near);
            for &j in &near {
                let (j, o) = (j as usize, before[j as usize]);
                if j == i || !self.soldiers[j].alive() {
                    continue;
                }
                let (dx, dy) = (p.0 - o.0, p.1 - o.1);
                let d = isqrt(dx * dx + dy * dy);
                // Walking men brush past each other; standing men only make room when someone is really close.
                let near = if step != (0, 0) { BODY_MM * 2 } else { PERSONAL_MM - 150 };
                if d < near {
                    let (ux, uy) = if d == 0 { if i < j { (-1, 0) } else { (1, 0) } } else { (dx * 1_000 / d, dy * 1_000 / d) };
                    let k = (near - d) * dt_ms as i64 / 100;
                    push = (push.0 + ux * k / 1_000, push.1 + uy * k / 1_000);
                }
            }
            self.scratch = near;
            let mut next = (p.0 + step.0 + push.0, p.1 + step.1 + push.1);
            if let Some(n) = &self.nav {
                // Blocked: drop the push, then slide along the obstacle (x part, then y part of the step).
                let tries = [next, (p.0 + step.0, p.1 + step.1), (p.0 + step.0, p.1), (p.0, p.1 + step.1)];
                next = tries.into_iter().find(|q| n.walkable_at(q.0, q.1, 0)).unwrap_or(p);
            }
            let s = &mut self.soldiers[i];
            s.vel = (next.0 - p.0, next.1 - p.1);
            s.pos = next;
            s.moving = step != (0, 0);
            if !s.moving {
                s.speed = 0;
            }
            // Low behind low cover and when moving crouched; standing otherwise.
            let low_cover = s.cover.zip(self.cover.as_ref()).is_some_and(|(c, m)| m.spots[c].height == Height::Low);
            // (Without a task or cover he keeps the stance he was given.)
            if s.task != Task::Idle || s.cover.is_some() {
                s.stance = if s.gait == Gait::Crouch && s.moving || low_cover && !s.moving { Stance::Crouch } else { Stance::Stand };
                if !s.moving && s.cover.is_none() {
                    if let Some(p) = s.posture {
                        s.stance = p;
                    }
                }
            }
        }
    }

    /// Cover decisions (with perception): pinned with known threats, run to the best cover in reach; calm again
    /// and nothing to shoot for a while, leave it.
    fn seek_cover(&mut self) {
        let Some(map) = self.cover.as_ref() else { return };
        for i in 0..self.soldiers.len() {
            let s = &self.soldiers[i];
            if !s.alive() {
                continue;
            }
            if s.cover.is_none() && s.suppression >= SEEK_COVER {
                // The four nearest known threats (a big fight has hundreds of contacts).
                let mut threats: Vec<(i64, i64, i64)> = self.contacts[s.squad as usize].iter().filter(|c| c.confidence >= CONF_HEARD).map(|c| ((c.pos.0 - s.pos.0).pow(2) + (c.pos.1 - s.pos.1).pow(2), c.pos.0, c.pos.1)).collect();
                threats.sort_unstable();
                let threats: Vec<(i64, i64)> = threats.iter().take(4).map(|t| (t.1, t.2)).collect();
                if threats.is_empty() {
                    continue;
                }
                let taken: Vec<usize> = self.soldiers.iter().filter_map(|o| o.cover).collect();
                if let Some(c) = map.best(s.pos, &threats, COVER_REACH_MM, &self.sight, &taken) {
                    self.soldiers[i].cover = Some(c);
                    self.soldiers[i].calm_ms = 0;
                    self.log.push(Event::Bark { soldier: i, bark: Bark::TakingCover });
                }
            } else if s.cover.is_some() {
                let calm = s.suppression < LEAVE_COVER && s.target.is_none();
                let me = &mut self.soldiers[i];
                me.calm_ms = if calm { me.calm_ms + PERCEIVE_MS } else { 0 };
                if me.calm_ms >= CALM_MS {
                    me.cover = None;
                }
            }
        }
    }

    /// Whether soldier `i` sees enemy `j` right now (as of the last perception pass).
    pub fn sees(&self, i: usize, j: usize) -> bool {
        self.visible[i].contains(&(j as u32))
    }

    /// What soldier `i`'s squad knows of enemy `j`.
    pub fn contact(&self, i: usize, j: usize) -> Option<&Contact> {
        let list = &self.contacts[self.soldiers[i].squad as usize];
        list.binary_search_by_key(&j, |c| c.enemy).ok().map(|k| &list[k])
    }

    /// Where soldier `i`'s squad last heard friendly gunfire.
    pub fn heard(&self, i: usize) -> Option<(i64, i64)> {
        self.heard[self.soldiers[i].squad as usize].map(|h| h.0)
    }

    pub(crate) fn report_pub(&mut self, squad: usize, enemy: usize, pos: (i64, i64), confidence: i32) {
        self.report(squad, enemy, pos, confidence);
    }

    /// Records (or refreshes) a squad's contact; a weaker report never overwrites a stronger, fresher one.
    fn report(&mut self, squad: usize, enemy: usize, pos: (i64, i64), confidence: i32) {
        let now = self.time_ms;
        let list = &mut self.contacts[squad];
        // Kept sorted by enemy: a big fight has hundreds of contacts per squad.
        match list.binary_search_by_key(&enemy, |c| c.enemy) {
            Ok(k) => {
                let c = &mut list[k];
                if confidence >= c.confidence {
                    *c = Contact { enemy, pos, seen_ms: now, confidence };
                }
            }
            Err(k) => list.insert(k, Contact { enemy, pos, seen_ms: now, confidence }),
        }
    }

    /// Who notices whom: inside the view cone (or very close) and in sight. Each man looks once per
    /// PERCEIVE_MS, on his own step (the work is spread over the steps), nearest enemies first: he tests at most
    /// RAYS_MAX of them and keeps the SEE_MAX nearest he sees. Sightings go to the squad.
    fn perceive(&mut self, dt_ms: i32) {
        let n = self.soldiers.len();
        // Each man looks once per PERCEIVE_MS, at his own offset in the window (spreading the work over the
        // steps): he looks on the step that passes it, whatever the step length.
        let slots = (PERCEIVE_MS / STEP_HINT_MS).max(1) as i64;
        let (now, before) = (self.time_ms, self.time_ms - dt_ms as i64);
        let mut near = std::mem::take(&mut self.scratch);
        let mut cands: Vec<(i64, u32)> = Vec::new();
        for i in 0..n {
            let off = (i as i64 % slots) * STEP_HINT_MS as i64;
            let window = PERCEIVE_MS as i64;
            if (now - off).div_euclid(window) == (before - off).div_euclid(window) {
                continue;
            }
            let me = &self.soldiers[i];
            if !me.alive() {
                self.visible[i].clear();
                continue;
            }
            // Outward ring by ring, until the RAYS_MAX nearest candidates are certain: nobody in a later ring
            // can be nearer than ring × CELL_MM. Far enemies only on this man's full look.
            let full = (self.time_ms / PERCEIVE_MS as i64 + i as i64) % FAR_EVERY == 0;
            let reach = if full { RANGE_MM } else { NEAR_MM };
            cands.clear();
            for r in 0..=self.grid.rings_for(reach) {
                near.clear();
                self.grid.ring(me.pos, r, &mut near);
                for &j in &near {
                    let t = &self.soldiers[j as usize];
                    if t.team == me.team {
                        continue;
                    }
                    let (dx, dy) = (t.pos.0 - me.pos.0, t.pos.1 - me.pos.1);
                    let d2 = dx * dx + dy * dy;
                    if d2 > reach * reach {
                        continue;
                    }
                    // In the cone: facing·d ≥ |d|·cos, done on squares (no square root in the hot loop).
                    let dot = me.facing.0 * dx + me.facing.1 * dy;
                    if in_cone(dot, d2) || d2 <= CLOSE_MM * CLOSE_MM {
                        cands.push((d2, j));
                    }
                }
                if cands.len() >= RAYS_MAX {
                    cands.sort_unstable();
                    cands.truncate(RAYS_MAX);
                    let bound = r * crate::grid::CELL_MM;
                    if cands[RAYS_MAX - 1].0 < bound * bound {
                        break;
                    }
                }
            }
            cands.sort_unstable();
            let mut seen = std::mem::take(&mut self.visible[i]);
            // Between full looks, the far enemies he saw stay in view (still alive and in range).
            let me_pos = self.soldiers[i].pos;
            let me_facing = self.soldiers[i].facing;
            seen.retain(|&j| {
                let t = &self.soldiers[j as usize];
                let (dx, dy) = (t.pos.0 - me_pos.0, t.pos.1 - me_pos.1);
                let d2 = dx * dx + dy * dy;
                !full && t.alive() && d2 >= NEAR_MM * NEAR_MM && d2 <= RANGE_MM * RANGE_MM && in_cone(me_facing.0 * dx + me_facing.1 * dy, d2)
            });
            for &(_, j) in cands.iter().take(RAYS_MAX) {
                if !seen.contains(&j) && self.in_sight(i, j as usize) {
                    seen.push(j);
                    if seen.len() >= SEE_MAX {
                        break;
                    }
                }
            }
            let sq = self.soldiers[i].squad as usize;
            for &j in &seen {
                let pos = self.soldiers[j as usize].pos;
                self.report(sq, j as usize, pos, CONF_SEEN);
            }
            self.visible[i] = seen;
        }
        self.scratch = near;
    }

    fn rebuild_grid(&mut self) {
        let soldiers = &self.soldiers;
        self.grid.rebuild(soldiers.iter().map(|s| (s.pos, s.alive())));
        self.goals.rebuild(soldiers.iter().map(|s| (s.goal.unwrap_or(s.pos), s.alive() && Self::claims_goal(s))));
    }

    /// Whether a soldier's spot counts as his (he is going there or standing on it), rather than where he is.
    fn claims_goal(s: &Soldier) -> bool {
        s.path_to.is_some() && s.task != Task::Idle && s.goal.is_some()
    }

    /// Whether any of the target's body points is in the shooter's sight (stops at the first).
    fn in_sight(&self, shooter: usize, target: usize) -> bool {
        let (s, t) = (&self.soldiers[shooter], &self.soldiers[target]);
        // Head first: low cover hides the legs most often, so the answer comes soonest that way round.
        t.stance.body().iter().rev().any(|&z| self.sight.clear(s.eye(), Point { x: t.pos.0, y: t.pos.1, z }))
    }

    /// How many of the target's three body points the shooter can see (0..3).
    pub fn exposure(&self, shooter: usize, target: usize) -> i64 {
        let (s, t) = (&self.soldiers[shooter], &self.soldiers[target]);
        t.stance.body().iter().filter(|&&z| self.sight.clear(s.eye(), Point { x: t.pos.0, y: t.pos.1, z })).count() as i64
    }

    /// Chance (per mille) that a shot from `shooter` hits `target` now.
    pub fn hit_permille(&self, shooter: usize, target: usize) -> i64 {
        let (s, t) = (&self.soldiers[shooter], &self.soldiers[target]);
        let (dx, dy) = (t.pos.0 - s.pos.0, t.pos.1 - s.pos.1);
        let d_m = isqrt(dx * dx + dy * dy) / 1000;
        let base = (900 - (d_m - 10).max(0) * 600 / 140).clamp(80, 900);
        let moving = if s.moving { 500 } else { 1_000 };
        let target_moving = if t.moving { 700 } else { 1_000 };
        let calm = 1_000 - s.suppression as i64 / 2;
        let shown = self.exposure(shooter, target) * 1_000 / 3;
        base * t.stance.size() / 1_000 * moving / 1_000 * target_moving / 1_000 * calm / 1_000 * shown / 1_000
    }

    fn roll(&self, who: usize, n: u64) -> i64 {
        (mix64(self.seed ^ (self.time_ms as u64).wrapping_mul(0x9e37_79b9) ^ (who as u64) << 40 ^ n) % 1_000) as i64
    }

    /// One fixed step of `dt_ms`.
    pub fn step(&mut self, dt_ms: i32) {
        self.time_ms += dt_ms as i64;
        self.apply_commands();
        let fade = SUPPRESS_FADE_S * dt_ms / 1_000;
        for s in &mut self.soldiers {
            s.suppression = (s.suppression - fade).max(0);
            s.cooldown_ms -= dt_ms;
        }
        let cfade = CONF_FADE_S * dt_ms / 1_000;
        for list in &mut self.contacts {
            for c in list.iter_mut() {
                c.confidence = (c.confidence - cfade).max(CONF_FLOOR);
            }
        }
        self.rebuild_grid();
        self.paths_this_step = 0;
        // Confirmed dead: off the contact lists.
        if self.dead_logged.iter().any(|&d| d) {
            let soldiers = &self.soldiers;
            for list in &mut self.contacts {
                list.retain(|c| soldiers[c.enemy].alive());
            }
        }
        self.perceive(dt_ms);
        self.perceive_ms -= dt_ms;
        if self.perceive_ms <= 0 {
            self.perceive_ms += PERCEIVE_MS;
            self.seek_cover();
        }
        self.lead_ms += dt_ms as i64;
        while self.lead_ms >= LEAD_MS {
            self.lead_ms -= LEAD_MS;
            self.lead();
        }
        let now = self.time_ms;
        for s in &mut self.soldiers {
            if s.act.is_some_and(|(_, until)| now >= until) {
                s.act = None;
            }
        }
        self.move_soldiers(dt_ms);
        for i in 0..self.soldiers.len() {
            if !self.soldiers[i].alive() || self.soldiers[i].passive || self.soldiers[i].act.is_some() {
                continue;
            }
            if self.soldiers[i].reloading_ms > 0 {
                self.soldiers[i].reloading_ms -= dt_ms;
                if self.soldiers[i].reloading_ms <= 0 {
                    self.soldiers[i].ammo = MAG;
                }
                continue;
            }
            // Engage the nearest enemy he sees.
            let me = &self.soldiers[i];
            let mut best: Option<(i64, usize)> = None;
            for &j in &self.visible[i] {
                let (j, t) = (j as usize, &self.soldiers[j as usize]);
                if !t.alive() {
                    continue;
                }
                let (dx, dy) = (t.pos.0 - me.pos.0, t.pos.1 - me.pos.1);
                let d2 = dx * dx + dy * dy;
                if best.map_or(true, |(b, _)| d2 < b) {
                    best = Some((d2, j));
                }
            }
            self.soldiers[i].target = best.map(|b| b.1);
            // Face the fight: the target, else the freshest contact the squad has (shots from behind).
            let look = match self.soldiers[i].target {
                Some(j) => Some(self.soldiers[j].pos),
                None => self.contacts[self.soldiers[i].squad as usize].iter().filter(|c| self.time_ms - c.seen_ms <= 3_000 && c.confidence >= CONF_HEARD).max_by_key(|c| (c.confidence, c.seen_ms)).map(|c| c.pos),
            };
            let me = &mut self.soldiers[i];
            if let Some(p) = look {
                me.facing = turn_toward(me.facing, (p.0 - me.pos.0, p.1 - me.pos.1));
            } else if me.moving {
                me.facing = turn_toward(me.facing, me.watch.unwrap_or(me.vel));
            } else if let Some(c) = me.cover.zip(self.cover.as_ref()).map(|(c, m)| m.spots[c].toward) {
                me.facing = turn_toward(me.facing, c);
            } else if let Task::Hold { face, .. } = me.task {
                me.facing = turn_toward(me.facing, (face.0 - me.pos.0, face.1 - me.pos.1));
            }
            let Some(j) = self.soldiers[i].target else {
                self.soldiers[i].burst_left = 0;
                continue;
            };
            // Rounds fall at their own times inside the step; a long wait does not bank rounds.
            let step_start = self.time_ms - dt_ms as i64;
            self.soldiers[i].cooldown_ms = self.soldiers[i].cooldown_ms.max(-dt_ms);
            while self.soldiers[i].cooldown_ms <= 0 && self.soldiers[j].alive() {
                if self.soldiers[i].ammo <= 0 {
                    self.soldiers[i].burst_left = 0;
                    if self.soldiers[i].mags > 0 {
                        self.soldiers[i].mags -= 1;
                        self.soldiers[i].reloading_ms = RELOAD_MS;
                        self.log.push(Event::Reload { soldier: i });
                    }
                    break;
                }
                let d = crate::grid::dist(self.soldiers[i].pos, self.soldiers[j].pos);
                if self.soldiers[i].burst_left == 0 {
                    // A new burst: how many rounds by range.
                    self.rolls += 1;
                    let r = self.roll(i, self.rolls);
                    let n = if d < AUTO_MM { 4 + r % 3 } else if d < BURST_MM { 2 + r % 2 } else { 1 };
                    self.soldiers[i].burst_left = n as u8;
                    self.soldiers[i].burst_fired = 0;
                }
                let at_ms = (self.time_ms + self.soldiers[i].cooldown_ms as i64).max(step_start);
                let recoil = RECOIL[(self.soldiers[i].burst_fired as usize).min(RECOIL.len() - 1)];
                let p = self.hit_permille(i, j) * recoil / 1_000;
                self.rolls += 1;
                let hit = self.roll(i, self.rolls) < p;
                let (from, t) = (self.soldiers[i].eye(), &self.soldiers[j]);
                let to = Point { x: t.pos.0, y: t.pos.1, z: t.stance.body()[1] };
                let me = &mut self.soldiers[i];
                me.ammo -= 1;
                me.burst_left -= 1;
                me.burst_fired += 1;
                me.cooldown_ms += if me.burst_left > 0 {
                    CYCLIC_MS
                } else if d < AUTO_MM {
                    AFTER_AUTO_MS
                } else if d < BURST_MM {
                    AFTER_BURST_MS
                } else {
                    AFTER_SINGLE_MS
                };
                self.log.push(Event::Shot { shooter: i, target: j, from, to, hit, at_ms });
                // The target's squad learns where the fire came from; each squad within earshot hears it.
                let (shooter_pos, shooter_team) = (self.soldiers[i].pos, self.soldiers[i].team);
                let tsq = self.soldiers[j].squad as usize;
                self.report(tsq, i, shooter_pos, CONF_SHOT_AT);
                let mut near = std::mem::take(&mut self.scratch);
                self.grid.near(shooter_pos, HEAR_MM, &mut near);
                let mut told: Vec<usize> = Vec::new();
                for &k in &near {
                    let o = &self.soldiers[k as usize];
                    let (dx, dy) = (o.pos.0 - shooter_pos.0, o.pos.1 - shooter_pos.1);
                    let sq = o.squad as usize;
                    if !o.alive() || dx * dx + dy * dy > HEAR_MM * HEAR_MM || told.contains(&sq) {
                        continue;
                    }
                    told.push(sq);
                    if o.team == shooter_team {
                        self.heard[sq] = Some((shooter_pos, self.time_ms));
                    } else {
                        self.report(sq, i, shooter_pos, CONF_HEARD);
                    }
                }
                self.scratch = near;
                let t = &mut self.soldiers[j];
                t.suppression = (t.suppression + if hit { SUPPRESS_HIT } else { SUPPRESS_MISS }).min(1_000);
                if hit {
                    t.health = t.health.saturating_sub(DAMAGE);
                    if !t.alive() {
                        self.dead_logged[j] = true;
                        self.log.push(Event::Killed { soldier: j, by: i });
                    }
                }
            }
            if !self.soldiers[j].alive() {
                self.soldiers[i].burst_left = 0;
            }
        }
    }
}


/// Whether `dot` (facing ×1000 · offset) ≥ ⌊√d2⌋ × CONE_COS (d2 = |offset|²), without a square root:
/// for dot ≥ 0 that is d2 < (⌊dot / CONE_COS⌋ + 1)². (At d2 = 0 the old test said no and this says yes; that
/// case is always within CLOSE_MM, which the caller accepts anyway.)
fn in_cone(dot: i64, d2: i64) -> bool {
    if dot < 0 {
        return false;
    }
    let q = dot / CONE_COS + 1;
    d2 < q * q
}

/// Whether a man can walk straight from `a` to `b` (checked every 250 mm with his body's clearance).
fn walk_line(n: &NavGrid, a: (i64, i64), b: (i64, i64)) -> bool {
    let len = crate::grid::dist(a, b);
    let k = (len / 250).max(1);
    (0..=k).all(|t| n.walkable_at(a.0 + (b.0 - a.0) * t / k, a.1 + (b.1 - a.1) * t / k, BODY_MM))
}

/// Turns a facing (unit vector x1000) toward a direction by at most one step's worth (4 rad/s at 50 ms).
fn turn_toward(f: (i64, i64), d: (i64, i64)) -> (i64, i64) {
    let len = isqrt(d.0 * d.0 + d.1 * d.1);
    if len == 0 {
        return f;
    }
    let u = (d.0 * 1_000 / len, d.1 * 1_000 / len);
    let dot = f.0 * u.0 + f.1 * u.1; // x 1e6
    if dot >= TURN_COS * 1_000 {
        return u;
    }
    let side = if f.0 * u.1 - f.1 * u.0 >= 0 { 1 } else { -1 };
    let r = ((f.0 * TURN_COS - side * f.1 * TURN_SIN) / 1_000, (side * f.0 * TURN_SIN + f.1 * TURN_COS) / 1_000);
    let l = isqrt(r.0 * r.0 + r.1 * r.1).max(1);
    (r.0 * 1_000 / l, r.1 * 1_000 / l)
}

#[cfg(test)]
mod cone_tests {
    use super::*;

    #[test]
    fn the_root_free_cone_test_matches_the_old_one() {
        for k in 0..200_000u64 {
            let r = mix64(k);
            let (dx, dy) = ((r % 400_001) as i64 - 200_000, ((r >> 24) % 400_001) as i64 - 200_000);
            let a = (r >> 48) % 360;
            let f = crate::battle::DIRS_24[(a / 15) as usize];
            let (dot, d2) = (f.0 * dx + f.1 * dy, dx * dx + dy * dy);
            if d2 == 0 {
                continue;
            }
            assert_eq!(in_cone(dot, d2), dot >= isqrt(d2).max(1) * CONE_COS, "{dx} {dy} {f:?}");
        }
    }
}
