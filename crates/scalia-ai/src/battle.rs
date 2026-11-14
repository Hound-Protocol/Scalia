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

/// What the leader has a man doing, for how he is drawn (the animation layer reads it; nothing in the
/// simulation does).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Cue {
    #[default]
    None,
    /// In a door stack: shoulder to the wall, rifle at high ready.
    Stack,
    /// Pieing a door or gap: leaning out to look round its edge (true: the edge is on his right).
    Peek(bool),
    /// Inside a room being cleared: sweeping it.
    Sweep,
    /// Down on a knee, listening (the briefing).
    Kneel,
    /// The leader briefing his men: talking with his hands.
    Brief,
}

/// What a soldier calls out (the radio voices and on-screen labels pick these up).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Bark {
    TakingCover,
    Contact,
    Reloading,
    ManDown,
    Frag,
    /// The leader is down: the next man takes command.
    TakingCommand,
}

/// Something a soldier is busy doing with his hands (the animation plays it; he does not shoot meanwhile).
#[derive(Copy, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Act {
    Throw,
    Drone,
    /// Filling and laying sandbags.
    Build,
    /// First aid on a wounded squadmate.
    Aid,
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
const SEEK_COVER: i32 = 300;
const LEAVE_COVER: i32 = 150;
const CALM_MS: i32 = 3_000;
/// Trading fire with someone further than this, a man in the open looks for cover even before he is pinned.
const OPEN_FIGHT_MM: i64 = 20_000;
/// Rounds coming in at all (one near miss is enough): out of an assault, he gets behind something first.
const UNDER_FIRE: i32 = 100;
/// An enemy this close is fought where he stands: no running for cover with him that near.
const CLOSE_FIGHT_MM: i64 = 10_000;
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
/// Spare magazines carried (seven 30-round M4 magazines in all, one in the rifle).
pub const MAGS: i32 = 6;
/// Below this many rounds, a man with nothing to shoot at tops up with a fresh magazine.
const TOP_UP: i32 = 12;
/// From this suppression on he mostly keeps his head down, firing only now and then.
const PINNED: i32 = 600;
pub const DAMAGE: i32 = 35;
/// Marksmanship (‰): the chance a round hits a man standing still in the open at 10 m or less, the shooter
/// steady and calm. A rookie hits half; no soldier, however seasoned, ever does better than 85 %. Range,
/// movement, recoil, suppression and how much of the target shows all come off this.
pub const SKILL_ROOKIE: i32 = 500;
pub const SKILL_REGULAR: i32 = 600;
pub const SKILL_VETERAN: i32 = 720;
pub const SKILL_MAX: i32 = 850;
/// At or below this a man is down: he cannot move or fight and bleeds until treated.
pub const DOWN_HP: i32 = 40;
/// A man down and untreated loses one point of health this often.
const BLEED_MS: i32 = 1_000;
/// How far in front of his helper a dragged man lies (the helper's arms and his own shoulders).
/// From the dragger to the casualty's head (his hands on the shoulder straps).
const DRAG_REACH_MM: i64 = 450;
/// A man lying down: from his feet (where he stands) to the top of his head.
const BODY_LEN_MM: i64 = 1_700;
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
pub(crate) const CONF_HEARD: i32 = 300;
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
    /// Down and treated: the bleeding has stopped (he still needs evacuating).
    pub(crate) stable: bool,
    bleed_ms: i32,
    /// The wounded man he is helping, and whether he is dragging him right now.
    pub(crate) aiding: Option<usize>,
    pub(crate) dragging: Option<usize>,
    /// Current speed (mm/s): it winds up towards the gait's pace and brakes into the stop.
    speed: i64,
    calm_ms: i32,
    /// How he is to be drawn for what the leader has him doing.
    pub cue: Cue,
    /// What he wants now and his plan for it (soldier mind, GOAP).
    pub mind: crate::mind::Mind,
    /// What the plan lets him do: open fire, put covering fire down.
    pub may_fire: bool,
    pub may_suppress: bool,
    /// The enemy he has his sights on (a new one takes a moment to get a sight picture on).
    sighted: Option<usize>,
    /// Marksmanship (‰, SKILL_ROOKIE to SKILL_MAX): grows with veterancy.
    pub skill: i32,
}

impl Soldier {
    pub fn new(team: u8, squad: u16, pos: (i64, i64), stance: Stance) -> Soldier {
        Soldier { team, squad, pos, stance, moving: false, health: 100, ammo: MAG, mags: MAGS, suppression: 0, cooldown_ms: 0, reloading_ms: 0, target: None, passive: false, facing: (0, 1_000), task: Task::Idle, gait: Gait::Walk, cover: None, vel: (0, 0), aim: false, act: None, burst_left: 0, burst_fired: 0, path: Vec::new(), path_to: None, goal: None, settled: false, posture: None, watch: None, stable: false, bleed_ms: 0, aiding: None, dragging: None, speed: 0, calm_ms: 0, skill: SKILL_REGULAR, sighted: None, mind: crate::mind::Mind::default(), may_fire: true, may_suppress: true, cue: Cue::None }
    }
    /// His own pace (mm/s): winding up and braking, not counting being nudged aside.
    pub fn pace(&self) -> i64 {
        if self.moving { self.speed } else { 0 }
    }

    /// Down: wounded too badly to move or fight (alive, at or below DOWN_HP).
    pub fn down(&self) -> bool {
        self.alive() && self.health <= DOWN_HP
    }

    /// Down and treated: no longer bleeding.
    pub fn stable(&self) -> bool {
        self.stable
    }

    /// The wounded man he is dragging, if any.
    pub fn dragging(&self) -> Option<usize> {
        self.dragging
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
    /// The leader briefs a man on his part in the plan.
    Briefed { leader: usize, soldier: usize, role: &'static str },
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
            // Under fire, cover is the first thing: he sprints for it.
            return Some((map.spots[c].pos, Gait::Sprint));
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

    /// Wounds: a man just gone down drops (prone, task and kit let go) and a squadmate shouts; the untreated
    /// down bleed, and die of it.
    fn wounds(&mut self, dt_ms: i32) {
        for i in 0..self.soldiers.len() {
            if !self.soldiers[i].down() {
                continue;
            }
            let s = &mut self.soldiers[i];
            if s.stance != Stance::Prone {
                s.stance = Stance::Prone;
                s.task = Task::Idle;
                s.target = None;
                s.cover = None;
                s.act = None;
                s.aim = false;
                s.posture = None;
                s.path.clear();
                s.path_to = None;
                s.aiding = None;
                s.dragging = None;
                let (sq, team, p) = (s.squad, s.team, s.pos);
                let shout = self.soldiers.iter().enumerate().filter(|(j, o)| *j != i && o.squad == sq && o.team == team && o.alive() && !o.down()).min_by_key(|(j, o)| ((o.pos.0 - p.0).pow(2) + (o.pos.1 - p.1).pow(2), *j)).map(|(j, _)| j);
                if let Some(j) = shout {
                    self.log.push(Event::Bark { soldier: j, bark: Bark::ManDown });
                }
            }
            let s = &mut self.soldiers[i];
            if s.stable {
                continue;
            }
            s.bleed_ms += dt_ms;
            while s.bleed_ms >= BLEED_MS && s.alive() {
                s.bleed_ms -= BLEED_MS;
                s.health -= 1;
            }
            if !s.alive() && !self.dead_logged[i] {
                self.dead_logged[i] = true;
                self.log.push(Event::Killed { soldier: i, by: i });
            }
        }
    }

    /// A dragged man lies at his helper's hands: just in front of him (the helper walks backwards, facing him).
    fn drag_along(&mut self) {
        for h in 0..self.soldiers.len() {
            let Some(w) = self.soldiers[h].dragging else { continue };
            if !self.soldiers[h].alive() || self.soldiers[h].down() || !self.soldiers[w].alive() {
                self.soldiers[h].dragging = None;
                continue;
            }
            let (hp, wp) = (self.soldiers[h].pos, self.soldiers[w].pos);
            let (dx, dy) = (wp.0 - hp.0, wp.1 - hp.1);
            let d = isqrt(dx * dx + dy * dy).max(1);
            let f = (dx * 1_000 / d, dy * 1_000 / d);
            self.soldiers[h].facing = f;
            // He is pulled by the shoulder straps, head first: lying on his back facing away from the man
            // dragging him, so his head (a body length behind his feet) is at the dragger's hands.
            self.soldiers[w].facing = f;
            let at = DRAG_REACH_MM + BODY_LEN_MM;
            self.soldiers[w].pos = (hp.0 + f.0 * at / 1_000, hp.1 + f.1 * at / 1_000);
        }
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
            if self.soldiers[i].down() {
                // Down: he lies where he fell (or where he is dragged).
                let s = &mut self.soldiers[i];
                s.moving = false;
                s.vel = (0, 0);
                s.speed = 0;
                continue;
            }
            let mut dest = self.destination(i);
            // Rifle up and an enemy in sight: stop and shoot, then carry on (not when running for cover).
            let s = &self.soldiers[i];
            if s.aim && s.target.is_some() && s.cover.is_none() && dest.is_some_and(|d| d.1 != Gait::Sprint) {
                dest = None;
            }
            let mut step = (0i64, 0i64);
            if let Some((to, gait)) = dest {
                self.soldiers[i].gait = gait;
                let pos = self.soldiers[i].pos;
                // Re-told to stand about where he already stands (a hold at his own feet, given again each
                // tick): he keeps his spot rather than hunting for a new one, which made men shuffle in place.
                let keep = self.soldiers[i].path_to != Some(to)
                    && self.soldiers[i].goal.is_some_and(|g| near(g, to, SETTLE_MM) && near(g, pos, SETTLE_MM))
                    && matches!(self.soldiers[i].task, Task::Hold { .. });
                if keep {
                    self.soldiers[i].path_to = Some(to);
                }
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
                // The wounded lie still and are tended where they lie: nobody is shoved off them.
                if j == i || !self.soldiers[j].alive() || self.soldiers[j].down() || self.soldiers[i].aiding.is_some() {
                    continue;
                }
                let (dx, dy) = (p.0 - o.0, p.1 - o.1);
                let d = isqrt(dx * dx + dy * dy);
                // Walking men brush past each other; standing men only make room when someone is really close.
                // A settled man gives way only to someone actually brushing him, and gently: no sliding about.
                let near = if step != (0, 0) { BODY_MM * 2 } else if self.soldiers[i].settled { BODY_MM * 2 - 50 } else { PERSONAL_MM - 150 };
                if d < near {
                    let (ux, uy) = if d == 0 { if i < j { (-1_000, 0) } else { (1_000, 0) } } else { (dx * 1_000 / d, dy * 1_000 / d) };
                    let k = (near - d) * dt_ms as i64 / 250;
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

    /// Every soldier thinks (soldier mind, GOAP): his facts from what he knows and how he stands, his most
    /// wanted thing and a plan for it; the plan's first action sets what he does (take cover, reload, fire,
    /// covering fire, carry on with the order). Calm again and nothing to shoot for a while, he leaves cover.
    fn think(&mut self) {
        use crate::mind::{fact, think, Act, Situation, Want};
        let now = self.time_ms;
        for i in 0..self.soldiers.len() {
            let s = &self.soldiers[i];
            if !s.alive() || s.down() {
                self.soldiers[i].mind = crate::mind::Mind::default();
                continue;
            }
            let busy = s.aiding.is_some() || s.dragging.is_some();
            let indoors = self.building.as_ref().is_some_and(|(k, _)| k.room_at(s.pos.0, s.pos.1).is_some());
            let assaulting = self.brains.get(s.squad as usize).is_some_and(|b| matches!(b.order, crate::leader::Order::Capture));
            let target_close = s.target.is_some_and(|t| near(self.soldiers[t].pos, s.pos, CLOSE_FIGHT_MM));
            let open_fight = s.target.is_some_and(|t| !near(self.soldiers[t].pos, s.pos, OPEN_FIGHT_MM)) && s.gait != Gait::Crouch && s.suppression > 0 && !assaulting;
            // Shot at: one round is enough, unless in an assault (then only pinned).
            let under_fire = if assaulting { s.suppression >= SEEK_COVER } else { s.suppression >= UNDER_FIRE } || open_fight;
            let mut in_cover = s.cover.zip(self.cover.as_ref()).is_some_and(|(c, m)| near(m.spots[c].pos, s.pos, 800));
            // Flanked: the cover he is in no longer stands between him and the nearest threat. It is no cover
            // any more; he looks for another.
            let mut flanked = false;
            if in_cover && s.suppression >= UNDER_FIRE {
                if let (Some(c), Some(map)) = (s.cover, self.cover.as_ref()) {
                    let nearest = self.contacts[s.squad as usize].iter().filter(|c| c.confidence >= CONF_HEARD && now - c.seen_ms <= 3_000).min_by_key(|c| ((c.pos.0 - s.pos.0).pow(2) + (c.pos.1 - s.pos.1).pow(2), c.enemy)).map(|c| c.pos);
                    if nearest.is_some_and(|t| map.protection(&map.spots[c], t, &self.sight) < 2) {
                        in_cover = false;
                        flanked = true;
                    }
                }
            }
            let s = &self.soldiers[i];
            // Cover worth running for: the spot he is running to, or the best one in reach against the four
            // nearest known threats.
            let mut candidate = None;
            let cover_near = if s.cover.is_some() && !flanked {
                true
            } else if under_fire && !indoors && !target_close {
                if let Some(map) = self.cover.as_ref() {
                    let mut threats: Vec<(i64, i64, i64)> = self.contacts[s.squad as usize].iter().filter(|c| c.confidence >= CONF_HEARD).map(|c| ((c.pos.0 - s.pos.0).pow(2) + (c.pos.1 - s.pos.1).pow(2), c.pos.0, c.pos.1)).collect();
                    threats.sort_unstable();
                    let threats: Vec<(i64, i64)> = threats.iter().take(4).map(|t| (t.1, t.2)).collect();
                    if !threats.is_empty() {
                        let taken: Vec<usize> = self.soldiers.iter().filter_map(|o| o.cover).collect();
                        candidate = map.best(s.pos, &threats, COVER_REACH_MM, &self.sight, &taken);
                    }
                }
                candidate.is_some()
            } else {
                false
            };
            let safe = in_cover || !under_fire || target_close || indoors || !cover_near;
            let contact = self.contacts[s.squad as usize].iter().any(|c| now - c.seen_ms <= 4_000 && c.confidence >= CONF_HEARD && self.soldiers[c.enemy].alive() && !self.soldiers[c.enemy].down());
            let bits = [
                (fact::TARGET, s.target.is_some()),
                (fact::IN_COVER, in_cover),
                (fact::COVER_NEAR, cover_near),
                (fact::UNDER_FIRE, under_fire),
                (fact::SAFE, safe),
                (fact::MAG_OK, s.ammo > 0),
                (fact::SPARE_MAGS, s.mags > 0),
                (fact::CONTACT, contact),
            ];
            let facts = bits.iter().fold(0u64, |a, &(b, on)| if on { a | 1 << b } else { a });
            let sit = Situation { facts, suppression: s.suppression, target_close, mag_empty: s.ammo <= 0, aim: s.aim, aiding: busy };
            let mind = think(&sit);
            let doing = mind.doing();
            let me = &mut self.soldiers[i];
            me.may_fire = !busy && match doing {
                Some(Act::TakeCover) => target_close,
                _ => true,
            };
            me.may_suppress = doing == Some(Act::Suppress);
            match doing {
                Some(Act::TakeCover) => {
                    if flanked && candidate.is_some() {
                        me.cover = None;
                    }
                    if me.cover.is_none() {
                        if let Some(c) = candidate {
                            me.cover = Some(c);
                            me.calm_ms = 0;
                            self.log.push(Event::Bark { soldier: i, bark: Bark::TakingCover });
                        }
                    }
                }
                Some(Act::Reload) | Some(Act::ReloadExposed) => self.reload(i),
                _ => {}
            }
            // Out of cover once it is calm again: no rounds coming in and nothing to shoot for a while.
            let me = &mut self.soldiers[i];
            if me.cover.is_some() && mind.want != Want::Survive {
                // The calm counts from when he is behind it, not from when he set off for it.
                let calm = in_cover && me.suppression < LEAVE_COVER && me.target.is_none();
                me.calm_ms = if calm { me.calm_ms + PERCEIVE_MS } else { 0 };
                if me.calm_ms >= CALM_MS {
                    me.cover = None;
                }
            }
            self.soldiers[i].mind = mind;
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
        // Range: full skill out to 10 m, then falling off (55 % of it at 50 m, 30 % at 100 m, 12 % at 200 m).
        let range = match d_m {
            0..=10 => 1_000,
            11..=50 => 1_000 - (d_m - 10) * 450 / 40,
            51..=100 => 550 - (d_m - 50) * 250 / 50,
            101..=200 => 300 - (d_m - 100) * 180 / 100,
            _ => 120,
        };
        let base = s.skill.clamp(0, SKILL_MAX) as i64 * range / 1_000;
        let moving = if s.moving { 500 } else { 1_000 };
        let target_moving = if t.moving { 700 } else { 1_000 };
        let calm = 1_000 - s.suppression as i64 / 2;
        let shown = self.exposure(shooter, target) * 1_000 / 3;
        base * t.stance.size() / 1_000 * moving / 1_000 * target_moving / 1_000 * calm / 1_000 * shown / 1_000
    }

    /// A fresh magazine in, if he has one (a little slower or quicker every time; fumbled under fire).
    fn reload(&mut self, i: usize) {
        if self.soldiers[i].mags <= 0 || self.soldiers[i].reloading_ms > 0 {
            return;
        }
        self.rolls += 1;
        let r = self.roll(i, self.rolls) as i32;
        let s = &mut self.soldiers[i];
        s.mags -= 1;
        s.reloading_ms = (RELOAD_MS - 300 + r * 700 / 1_000) * (1_000 + s.suppression / 2) / 1_000;
        s.burst_left = 0;
        self.log.push(Event::Reload { soldier: i });
        self.log.push(Event::Bark { soldier: i, bark: Bark::Reloading });
    }

    /// Nothing in his sights: top up the magazine in the lull, or (rifle up, calm enough, not on the move)
    /// put covering fire where the squad last saw an enemy, to keep his head down.
    fn lull(&mut self, i: usize, dt_ms: i32) {
        let s = &self.soldiers[i];
        if s.ammo < TOP_UP && s.mags > 0 && s.ammo < MAG {
            self.reload(i);
            return;
        }
        self.soldiers[i].cooldown_ms = self.soldiers[i].cooldown_ms.max(-dt_ms);
        let s = &self.soldiers[i];
        if !s.may_suppress || !s.aim || s.moving || s.cooldown_ms > 0 || s.suppression >= PINNED / 2 || s.ammo <= TOP_UP {
            return;
        }
        let now = self.time_ms;
        let Some(c) = self.contacts[s.squad as usize].iter().filter(|c| now - c.seen_ms <= 4_000 && c.confidence >= CONF_HEARD && self.soldiers[c.enemy].alive() && !self.soldiers[c.enemy].down()).filter(|c| near(c.pos, s.pos, BURST_MM) && !near(c.pos, s.pos, 3_000)).max_by_key(|c| (c.seen_ms, c.confidence)).cloned() else { return };
        // Only toward where he can put rounds (a clear line to the spot), and not every lull.
        let aim_at = Point { x: c.pos.0, y: c.pos.1, z: 1_200 };
        if !self.sight.clear(s.eye(), aim_at) {
            return;
        }
        self.rolls += 1;
        if self.roll(i, self.rolls) < 450 {
            self.soldiers[i].cooldown_ms = 600;
            return;
        }
        self.rolls += 1;
        let n = 2 + self.roll(i, self.rolls) % 3;
        let from = self.soldiers[i].eye();
        for k in 0..n.min(self.soldiers[i].ammo as i64) {
            let at_ms = self.time_ms + k * CYCLIC_MS as i64;
            let r = mix64(self.seed ^ (at_ms as u64) << 9 ^ i as u64);
            let to = Point { x: c.pos.0 + (r % 3_001) as i64 - 1_500, y: c.pos.1 + ((r >> 16) % 3_001) as i64 - 1_500, z: 900 };
            self.soldiers[i].ammo -= 1;
            self.log.push(Event::Shot { shooter: i, target: c.enemy, from, to, hit: false, at_ms });
        }
        // Rounds cracking in around him: he feels it if he is still there.
        let e = &mut self.soldiers[c.enemy];
        if near(e.pos, c.pos, 4_000) {
            e.suppression = (e.suppression + SUPPRESS_MISS * n as i32 / 2).min(1_000);
        }
        let (tsq, spos) = (self.soldiers[c.enemy].squad as usize, self.soldiers[i].pos);
        self.report(tsq, i, spos, CONF_SHOT_AT);
        self.rolls += 1;
        self.soldiers[i].cooldown_ms = 1_200 + (self.roll(i, self.rolls) * 1_600 / 1_000) as i32;
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
            self.think();
        }
        self.wounds(dt_ms);
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
        self.drag_along();
        for i in 0..self.soldiers.len() {
            // Down men and men busy with a casualty do not fight.
            if !self.soldiers[i].alive() || self.soldiers[i].passive || self.soldiers[i].act.is_some() || self.soldiers[i].down() || self.soldiers[i].dragging.is_some() {
                // Hands busy (or down): no target held, no burst half-fired.
                if self.soldiers[i].dragging.is_some() || self.soldiers[i].act.is_some() || self.soldiers[i].down() {
                    self.soldiers[i].target = None;
                    self.soldiers[i].burst_left = 0;
                }
                continue;
            }
            if self.soldiers[i].reloading_ms > 0 {
                self.soldiers[i].reloading_ms -= dt_ms;
                if self.soldiers[i].reloading_ms <= 0 {
                    self.soldiers[i].ammo = MAG;
                    // A fresh magazine, then a moment to get back on the sights.
                    self.soldiers[i].sighted = None;
                }
                continue;
            }
            // Engage the nearest enemy he sees.
            let me = &self.soldiers[i];
            let mut best: Option<(i64, usize)> = None;
            for &j in &self.visible[i] {
                let (j, t) = (j as usize, &self.soldiers[j as usize]);
                // Nobody keeps shooting a man who is down.
                if !t.alive() || t.down() {
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
            // His mind holds his fire (running for cover, say), unless the enemy is right on top of him.
            if !self.soldiers[i].may_fire && self.soldiers[i].target.is_none_or(|t| !near(self.soldiers[t].pos, self.soldiers[i].pos, CLOSE_FIGHT_MM)) {
                self.soldiers[i].burst_left = 0;
                self.soldiers[i].sighted = None;
                continue;
            }
            let Some(j) = self.soldiers[i].target else {
                self.soldiers[i].burst_left = 0;
                self.soldiers[i].sighted = None;
                self.lull(i, dt_ms);
                continue;
            };
            // A new target: he needs a sight picture before the first round (longer far off, longer for a
            // rookie, a little different every time).
            if self.soldiers[i].sighted != Some(j) {
                self.soldiers[i].sighted = Some(j);
                self.soldiers[i].burst_left = 0;
                let d_m = crate::grid::dist(self.soldiers[i].pos, self.soldiers[j].pos) / 1_000;
                self.rolls += 1;
                let r = self.roll(i, self.rolls) as i32;
                let slow = 1_000 + (SKILL_MAX - self.soldiers[i].skill.min(SKILL_MAX)) * 2;
                let acquire = ((250 + d_m as i32 * 8) * slow / 1_000 + r * 350 / 1_000) * (1_000 + self.soldiers[i].suppression) / 1_000;
                self.soldiers[i].cooldown_ms = self.soldiers[i].cooldown_ms.max(acquire);
            }
            // Rounds fall at their own times inside the step; a long wait does not bank rounds.
            let step_start = self.time_ms - dt_ms as i64;
            self.soldiers[i].cooldown_ms = self.soldiers[i].cooldown_ms.max(-dt_ms);
            while self.soldiers[i].cooldown_ms <= 0 && self.soldiers[j].alive() {
                if self.soldiers[i].ammo <= 0 {
                    self.soldiers[i].burst_left = 0;
                    self.reload(i);
                    break;
                }
                let d = crate::grid::dist(self.soldiers[i].pos, self.soldiers[j].pos);
                if self.soldiers[i].burst_left == 0 {
                    // What now, with the rifle up: his nerve decides. Calm, he works the target with bursts
                    // that suit the range; under fire they get shorter and rarer; pinned, he mostly stays down
                    // and only now and then pops off a round or two.
                    self.rolls += 1;
                    let r = self.roll(i, self.rolls);
                    let sup = self.soldiers[i].suppression as i64;
                    if sup >= PINNED as i64 && r < sup - 300 {
                        self.rolls += 1;
                        self.soldiers[i].cooldown_ms += 500 + (self.roll(i, self.rolls) * 900 / 1_000) as i32;
                        continue;
                    }
                    self.rolls += 1;
                    let r2 = self.roll(i, self.rolls);
                    let full = if d < AUTO_MM { 3 + r2 % 4 } else if d < BURST_MM { 2 + r2 % 3 } else { 1 + r2 % 2 };
                    let n = (full * (1_000 - sup * 6 / 10) / 1_000).max(1).min(self.soldiers[i].ammo as i64);
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
                } else {
                    // Between bursts: re-aim, check the result, breathe. Longer far off, much longer when the
                    // rounds are coming his way, never quite the same twice.
                    let (lo, hi) = if d < AUTO_MM { (AFTER_AUTO_MS, AFTER_AUTO_MS * 2) } else if d < BURST_MM { (AFTER_BURST_MS, AFTER_BURST_MS * 3) } else { (AFTER_SINGLE_MS * 2, AFTER_SINGLE_MS * 4) };
                    let r = (mix64(self.seed ^ (at_ms as u64) << 8 ^ i as u64) % 1_000) as i32;
                    (lo + (hi - lo) * r / 1_000) * (1_000 + me.suppression * 2) / 1_000
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
/// Whether two points are within `r` of each other.
fn near(a: (i64, i64), b: (i64, i64), r: i64) -> bool {
    (a.0 - b.0).pow(2) + (a.1 - b.1).pow(2) <= r * r
}

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
