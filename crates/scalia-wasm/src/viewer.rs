//! The game viewer in Rust: three scenes drawn by `scalia-render`, driven by the core directly.
//! JavaScript only forwards the animation frame and input.

use std::collections::HashMap;

use glam::Vec3;
use scalia_core::derive::{derive_visible, Derived, Phase, ViewBox};
use scalia_core::flight::{flight_pos, plan_flight, FlightPlan, DEMO_PEAK_SPEED_MM_S};
use scalia_core::nav::{NavGrid, Prop, PropKind};
use scalia_core::nav_agents::AgentGroup;
use scalia_core::planet::Planet;
use scalia_core::command::{self as chain_of, Command, Kind, Stance, Step, Task};
use scalia_core::groups::GroupId;
use std::collections::BTreeMap;
use scalia_core::position::{Pos, RegionId};
use scalia_core::scenario::MARCH_SPEED_MM_S;
use scalia_core::process::ProcessKind;
use scalia_core::derive::{derive_vehicles, DerivedVehicle};
use scalia_core::groups::GroupSpec;
use scalia_core::units::{Composition, Needs, UnitType};
use scalia_core::system::{BodyId, System, PARK_OFFSET};
use scalia_core::time::WorldMs;
use scalia_core::world::World;
use scalia_render::bend::{globe_point, Bend};
use scalia_render::gpu::{FrameDesc, Gpu};
use scalia_render::mesh::pda_held;
use scalia_render::mesh::{figure, figure_far, figure_lo, grass_tuft, MeshBuilder};
use scalia_render::pose::Pose;
use scalia_render::sky::{clock, sky};
use wasm_bindgen::prelude::*;

/// Army sizes the N key cycles through, as (groups, soldiers per group).
/// Army sizes (groups, soldiers each). The first is the testing ground's: eight squads of twelve in the base.
/// Squad call signs, in group order.
const SQUADS: [&str; 10] = ["Alpha", "Bravo", "Charlie", "Delta", "Echo", "Foxtrot", "Golf", "Hotel", "India", "Juliett"];
/// The testing ground's captains.
const CAPTAINS: [&str; 2] = ["Reyes", "Okafor"];
/// A captain's step lasts at least this long (ms), so covering fire and reports have time to happen.
const STEP_MIN_MS: f64 = 2_500.0;

/// A four-figure-each grid reference (10 m squares) for a point in metres, as the tablet shows it.
fn grid(x: f64, y: f64) -> String {
    let g = |m: f64| (((m / 10.0).round() as i64 % 10_000) + 10_000) % 10_000;
    format!("{:04} {:04}", g(x), g(y))
}

fn pose_of(s: Stance) -> Pose {
    match s {
        Stance::Stand => Pose::Stand,
        Stance::Crouch => Pose::Crouch,
        Stance::Cover => Pose::Cover,
        Stance::CoverAim => Pose::CoverAim,
        Stance::StandAim => Pose::StandAim,
    }
}

// The tablet cinematic: a quick swing round behind the commander (same zoom and angle), a short hold, then a swift
// zoom onto the tablet in his hands; closing zooms straight back out.
const CINE_TURN_MS: f64 = 250.0;
const CINE_HOLD_MS: f64 = 300.0;
const CINE_OPEN_MS: f64 = 950.0;
const CINE_CLOSE_MS: f64 = 500.0;
/// The tablet close-up image's real height (metres, the device plus its margins): the dive ends with the camera
/// face-on to the tablet, framing it so it covers exactly the close-up's box, so the close-up fades in seamlessly.
const TABLET_IMG_H: f32 = 0.2182;

const ARMIES: [(u32, u32); 5] = [(8, 12), (1, 2000), (10, 2000), (50, 2000), (250, 2000)];
/// The testing ground: a military base on a levelled pad, (BASE_LO..BASE_HI) metres on both axes.
const BASE_LO: f32 = -20.0;
const BASE_HI: f32 = 220.0;
/// Most soldiers drawn in one frame.
const MAX_DRAWN: usize = 60_000;
const CELLS: u32 = 64;
const KM: f64 = 1e6;
/// Above this view height the ground is drawn from coarse far tiles instead of 1 m chunks.
const NEAR_MAX: f32 = 160.0;
/// Above this view height soldiers are too small to draw one by one; groups show as markers.
const UNITS_MAX: f32 = 2_000.0;
/// Planet radius in metres (the 4,096 km wrapping map is 2πR around).
const PLANET_R: f32 = 651_898.6;
/// The ground view zooms out to the whole globe; past this it hands over to the star system.
const GROUND_TOP: f32 = 4.0 * PLANET_R;
/// The zoom ladder: view heights in metres, one wheel notch per step. Up to GROUND_TOP the ground scene draws
/// them (squad, platoon, company, battlefield, region, globe); above it the star system does (orbit to system).
const LADDER: [f32; 21] = [
    12.0, 25.0, 50.0, 100.0, 160.0, 400.0, 1_000.0, 3_000.0, 10_000.0, 30_000.0, 100_000.0, 300_000.0, 800_000.0,
    1_700_000.0, GROUND_TOP, 1e7, 5e7, 2.5e8, 1.2e9, 6e9, 1.5e11,
];
/// Ladder steps the scene buttons jump to.
const STEP_GROUND: usize = 2;
const STEP_PLANET: usize = 13;
const STEP_HANDOFF: usize = 14;
const STEP_SYSTEM: usize = 19;
/// Furthest zoom on foot, with the tablet away (the tablet's map zooms all the way).
const STEP_FIELD_MAX: usize = 2;
/// The commander's walking and sprinting speeds (m/s), and how long radioing an order takes (ms).
const CMD_WALK: f32 = 1.7;
const CMD_SPRINT: f32 = 4.6;

/// How quickly the camera glides to the next step (time constant, ms).
const GLIDE_MS: f64 = 90.0;
/// How long the previous terrain detail level lingers over the new one (ms).
const FADE_MS: f64 = 600.0;
/// Height between the ground's contour colour bands (metres).
const BAND_M: f32 = 3.0;
/// A game day lasts one real hour.
const DAY_RATE: f64 = 1.0 / 3_600_000.0;
/// Orders at or above this speed run (Shift + right click).
const RUN_SPEED_MM_S: u32 = 3_500;
const HOME_BODY: u8 = 0;
/// The ground scene sits on the demo planet's equator: render chunk (cx, cy) is planet chunk
/// (cx, cy + EQUATOR_CHUNK). Rendering stays near the origin so f32 positions keep millimetre precision.
const EQUATOR_CHUNK: i64 = 16_000;

#[derive(Copy, Clone, PartialEq)]
enum Scene {
    Ships,
    Ground,
    System,
}

struct ShipInfo {
    name: String,
    triangles: usize,
    lo: Vec3,
    hi: Vec3,
}

#[wasm_bindgen]
pub struct Viewer {
    gpu: Gpu,
    scene: Scene,
    ship: usize,
    full: bool,
    outline: bool,
    t0: Option<f64>,
    ships: Vec<Option<ShipInfo>>,
    world: World,
    derived: Vec<Derived>,
    chunks: HashMap<(i64, i64), Vec<f32>>,
    chunk_lod: HashMap<(i64, i64), usize>,
    /// Sample step the terrain is drawn with this frame (1, 2 or 4).
    terrain_step: usize,
    army: usize,
    ground_view: f32,
    focus: Vec3,
    system: System,
    sys_scale: f32,
    sys_follow: Option<u8>,
    sys_now: WorldMs,
    /// Far terrain tiles built so far: (level, tile x, tile y); level k has 2^k m cells.
    far_tiles: std::collections::HashSet<(u32, i64, i64)>,
    /// Level of far tiles being drawn.
    far_level: Option<u32>,
    /// Floating origin (metres): the ground scene draws everything relative to this point near the camera, so the
    /// f32 positions sent to the GPU stay small (and precise) anywhere on the planet.
    origin: (f64, f64),
    /// The testing ground: its props, nav grid, level height (metres), and the squads walking as agents in it.
    base_props: Vec<Prop>,
    base_grid: NavGrid,
    base_h: f32,
    base_built: bool,
    agents: HashMap<u32, AgentGroup>,
    agent_prev: HashMap<u32, Vec<(i64, i64)>>,
    agent_clock: f64,
    /// Smoothed camera: ground target (absolute metres) and yaw, eased towards where they are aimed each frame.
    /// The player's commander: position (metres, absolute), heading, movement input (screen right, screen up),
    /// sprinting, moving, and until when he is on the radio.
    cmd: (f64, f64),
    cmd_heading: f32,
    cmd_input: (f32, f32, bool),
    cmd_moving: bool,
    cmd_pose: (Pose, Pose, f32),
    radio_until: f64,
    /// The tablet is up: the camera leaves the commander for the map, which zooms all the way.
    tablet: bool,
    /// How orders are carried out: 0 calm, 1 aggressive.
    manner: u8,
    /// The tablet's map has its own camera (picture in picture); `map_active` while it is swapped in.
    map_cam: MapCam,
    map_active: bool,
    map_pass: bool,
    cmd_ms: f64,
    /// Per agent soldier: eased heading and walking-until time (smooth turns, no walk/stand flicker).
    agent_anim: HashMap<u32, (f32, f64)>,
    /// The stance each unit takes once its soldiers reach the ordered place.
    arrive_stance: HashMap<u32, Pose>,
    /// Units sent to a point that will call the commander when they get there: the grid and the order's kind.
    arrive_report: HashMap<u32, (String, u8)>,
    /// The commander's gait phase (0..1; -1 standing).
    cmd_phase: f32,
    /// The commander's instance record from this frame's main pass (the map draws him from it, solid).
    cmd_rec: [f32; 12],
    /// How much of the screen's height the tablet close-up covers (set by the page).
    tablet_fit: f32,
    /// The close-up's extra scale and sideways shift (fraction of the screen width), matching the overlay when it
    /// makes room for the walkie-talkie.
    tablet_frame: (f32, f32),
    /// The tablet cinematic: opening (true) or closing, when it began (ms), and the camera it started from.
    cine: Option<(bool, f64, (Vec3, f32, f32, f32))>,
    /// Who answers to whom: squads to captains, or straight to the commander.
    chain: Command,
    /// Each captain's current task, the squads it is waiting on, and when its step began.
    tasks: BTreeMap<u32, (Task, Vec<u32>, f64)>,
    /// Radio traffic from the units for the tablet, as JSON objects (taken by `radio_events`).
    radio_out: Vec<String>,
    cam_smooth: Option<Vec3>,
    yaw_smooth: f32,
    cam_ms: f64,
    agent_now: f64,
    /// Cross-fade from a previous far level: (level, start of the fade in ms).
    far_fade: Option<(u32, f64)>,
    /// Zoom: the ladder step aimed at and the view height (metres) the camera is gliding through.
    zoom_step: usize,
    zoom_m: f32,
    last_frame_ms: f64,
    planet: Planet,
    /// The home planet's globe for the system view, oriented to match the ground view it was entered from.
    home_globe: Vec<([Vec3; 3], f32)>,
    /// Unit globes for the star system view: one per body (its own map), and the sun.
    body_globes: Vec<Vec<([Vec3; 3], f32)>>,
    sun_globe: Vec<([Vec3; 3], f32)>,
    docked: u8,
    flight: Option<(FlightPlan, u8)>,
    trail: Vec<(f64, f32, f32)>,
    /// Free camera per scene: (target offset or free target, yaw).
    ground_cam: Option<Vec3>,
    sys_cam: Option<Vec3>,
    yaw: f32,
    ship_yaw: Option<f32>,
    pixel_scale: u32,
    /// Time of day (fraction) at sim time 0, and how many game days pass per real millisecond.
    day_start: f64,
    day_rate: f64,
    day_at: f64,
    last_view: (f32, f32),
    last_internal: (u32, u32),
    /// Camera of the last frame: target, yaw, view height (for picking).
    last_cam: (Vec3, f32, f32),
    selected: Option<u32>,
    /// Every selected group (box selection); `selected` is the first of them.
    selection: Vec<u32>,
    follow_group: u32,
    /// Idle stance per group (P cycles it for the selected group).
    stance: HashMap<u32, Pose>,
    /// Per group: (current pose, previous pose, time of the change in seconds) for blending.
    group_pose: HashMap<u32, (Pose, Pose, f32)>,
    /// Pose held in the soldier preview; None cycles through all of them.
    preview_pose: Option<usize>,
    /// Model viewer: soldier level of detail (0 full, 1 mid, 2 far) and colour variant.
    model_detail: u8,
    model_variant: u8,
    /// Model viewer camera: elevation (radians), zoom (1 frames the model), light (time of day 0..1), and the yaw
    /// last drawn (so grabbing a spinning model does not jump).
    model_elev: f32,
    model_zoom: f32,
    model_light: f32,
    model_yaw: f32,
    order_dest: Option<Vec3>,
    fps: f32,
    frames: u32,
    fps_t: f64,
    /// Time of the last frame (ms) and the cinematic camera it used, if any.
    frame_ms: f64,
    last_cine: Option<(Vec3, f32, f32, f32)>,
    size: (u32, u32),
}

/// Creates the viewer on a canvas. Fails with a readable message when WebGPU is unavailable.
#[wasm_bindgen]
pub async fn start_viewer(canvas: web_sys::HtmlCanvasElement) -> Result<Viewer, JsError> {
    let size = (canvas.width(), canvas.height());
    let gpu = Gpu::new(wgpu_target(canvas), size.0, size.1).await.map_err(|e: String| JsError::new(&e))?;
    let mut v = Viewer {
        gpu,
        scene: Scene::Ships,
        ship: 5,
        full: false,
        outline: false,
        t0: None,
        ships: (0..scalia_blueprint::shipped::SHIPPED_RON.len()).map(|_| None).collect(),
        world: build_army(ARMIES[0].0, ARMIES[0].1),
        derived: Vec::new(),
        chunks: HashMap::new(),
        chunk_lod: HashMap::new(),
        terrain_step: 1,
        army: 0,
        ground_view: 60.0,
        focus: Vec3::ZERO,
        system: System::demo(),
        sys_scale: 260_000.0,
        sys_follow: None,
        sys_now: WorldMs(0),
        far_tiles: std::collections::HashSet::new(),
        far_level: None,
        origin: (0.0, 0.0),
        base_props: base_props(),
        base_grid: NavGrid::new((BASE_LO * 1000.0) as i64, (BASE_LO * 1000.0) as i64, ((BASE_HI - BASE_LO) * 2.0) as usize, ((BASE_HI - BASE_LO) * 2.0) as usize),
        base_h: 0.0,
        base_built: false,
        agents: HashMap::new(),
        agent_prev: HashMap::new(),
        agent_clock: -1.0,
        cmd: (70_000.0 / 1000.0 + 20.0, 20.0),
        cmd_heading: 0.0,
        cmd_input: (0.0, 0.0, false),
        cmd_moving: false,
        cmd_pose: (Pose::Stand, Pose::Stand, -1000.0),
        radio_until: -1.0,
        tablet: false,
        manner: 0,
        map_cam: MapCam {
            scene: Scene::Ground,
            zoom_step: 6,
            zoom_m: LADDER[6],
            ground_view: LADDER[6],
            sys_scale: 1000.0,
            sys_cam: None,
            sys_follow: Some(HOME_BODY),
            ground_cam: None,
            cam_smooth: None,
            cam_ms: 0.0,
            yaw: 0.0,
            yaw_smooth: 0.0,
            last_cam: (Vec3::ZERO, 0.0, 1.0),
            last_view: (1.0, 1.0),
            size: (1, 1),
            last_frame_ms: 0.0,
            last_internal: (1, 1),
        },
        map_active: false,
        map_pass: false,
        cmd_ms: 0.0,
        agent_anim: HashMap::new(),
        arrive_stance: HashMap::new(),
        arrive_report: HashMap::new(),
        cmd_phase: -1.0,
        cmd_rec: [0.0; 12],
        tablet_fit: 0.6,
        tablet_frame: (1.0, 0.0),
        cine: None,
        chain: Command::default(),
        tasks: BTreeMap::new(),
        radio_out: Vec::new(),
        cam_smooth: None,
        yaw_smooth: 0.0,
        cam_ms: 0.0,
        agent_now: 0.0,
        far_fade: None,
        zoom_step: STEP_GROUND,
        zoom_m: LADDER[STEP_GROUND],
        last_frame_ms: 0.0,
        planet: Planet::demo(42),
        home_globe: Vec::new(),
        body_globes: vec![globe_triangles(42, 64, 32), globe_triangles(7, 64, 32)],
        sun_globe: sphere_triangles(32, 16, 14.0),
        docked: 0,
        flight: None,
        trail: Vec::new(),
        ground_cam: None,
        sys_cam: None,
        yaw: std::f32::consts::FRAC_PI_4,
        ship_yaw: None,
        pixel_scale: 2,
        day_start: 0.3,
        day_rate: DAY_RATE,
        day_at: 0.0,
        last_view: (60.0, 1.0),
        last_internal: (1, 1),
        last_cam: (Vec3::ZERO, 0.0, 60.0),
        selected: Some(0),
        selection: vec![0],
        follow_group: 0,
        stance: HashMap::new(),
        group_pose: HashMap::new(),
        preview_pose: None,
        model_detail: 0,
        model_variant: 0,
        model_elev: scalia_render::camera::ELEV,
        model_zoom: 1.0,
        model_light: 0.42,
        model_yaw: 0.0,
        order_dest: None,
        fps: 0.0,
        frames: 0,
        fps_t: 0.0,
        frame_ms: 0.0,
        last_cine: None,
        size,
    };
    v.add_captains();
    v.gpu.upload("figure", figure().bytes());
    v.gpu.upload("figure_lo", figure_lo().bytes());
    v.gpu.upload("figure_far", figure_far().bytes());
    v.gpu.upload("tuft", grass_tuft().bytes());
    Ok(v)
}

fn wgpu_target(canvas: web_sys::HtmlCanvasElement) -> scalia_render::SurfaceTarget<'static> {
    scalia_render::SurfaceTarget::Canvas(canvas)
}

/// The viewer's test army: groups 200 m apart, each with infantry, 4 tanks and 3 trucks;
/// every third group marches 500 m east at walking pace.
fn build_army(groups: u32, per_group: u32) -> World {
    let mut w = World::new(42);
    let side = (groups as f64).sqrt().ceil().max(1.0) as u32;
    let testing = per_group <= 20;
    for g in 0..groups {
        // Testing squads stand on the base's parade ground; big armies spread over the plain.
        let pos = if testing {
            Pos { region: RegionId(1), x: 60_000 + (g % 4) as i64 * 14_000, y: 40_000 + (g / 4) as i64 * 14_000, z: 0 }
        } else {
            Pos { region: RegionId(1), x: (g % side) as i64 * 200_000, y: (g / side) as i64 * 200_000, z: 0 }
        };
        let spec = GroupSpec { pos, composition: Composition([per_group, 0, 0, 0]), needs: Needs::full() }; // soldiers only for now; vehicles return one by one, each with a defined purpose
        let id = w.groups.spawn(spec, &mut w.roster);
        if g % 3 == 0 && !testing {
            let _ = w.start_march(id, pos.offset(500_000, 0, 0), MARCH_SPEED_MM_S, WorldMs(0));
        }
    }
    w
}

/// Grass tufts for one chunk: about one per 2 m^2 on green bands, none on bare (brown) ground.
/// Deterministic per chunk, so the same field grows back in the same places.
fn grass_for(hm: &[f32], cx: i64, cy: i64, origin: (f64, f64)) -> Vec<f32> {
    let n = CELLS as usize + 1;
    let mut out = Vec::new();
    for j in 0..CELLS as usize {
        for i in 0..CELLS as usize {
            let h = scalia_core::hash::mix64(((cx as u64) << 40) ^ ((cy as u64) << 20) ^ ((j as u64) << 8) ^ i as u64 ^ 0x9a55);
            if h % 100 >= 55 {
                continue;
            }
            let band = ((hm[j * n + i] + hm[j * n + i + 1] + hm[(j + 1) * n + i] + hm[(j + 1) * n + i + 1]) / 4.0 * 8.0).floor().rem_euclid(4.0);
            if band == 3.0 {
                continue; // bare ground
            }
            let fx = ((h >> 8) & 0xff) as f32 / 255.0;
            let fy = ((h >> 16) & 0xff) as f32 / 255.0;
            let colour = [7.0, 12.0, 11.0, 12.0][((h >> 24) % 4) as usize];
            let z0 = bilinear(hm, n, i, j, fx, fy);
            let x = ((cx * CELLS as i64 + i as i64) as f64 + fx as f64 - origin.0) as f32;
            let y = ((cy * CELLS as i64 + j as i64) as f64 + fy as f64 - origin.1) as f32;
            let heading = ((h >> 32) & 0xffff) as f32 / 65535.0 * std::f32::consts::TAU;
            let phase = ((h >> 48) & 0xff) as f32 / 255.0;
            out.extend_from_slice(&[x, y, z0, colour, heading, phase, 0.0, 0.0, 0.0, -1000.0, 0.0, 0.0]);
        }
    }
    out
}

/// Height inside cell (i, j) of a chunk at fractions (fx, fy), matching the drawn triangles
/// (each cell is split along its a-c diagonal).
fn bilinear(hm: &[f32], n: usize, i: usize, j: usize, fx: f32, fy: f32) -> f32 {
    let (a, b, c, d) = (hm[j * n + i], hm[j * n + i + 1], hm[(j + 1) * n + i + 1], hm[(j + 1) * n + i]);
    if fx >= fy {
        a + (b - a) * fx + (c - b) * fy
    } else {
        a + (d - a) * fy + (c - d) * fx
    }
}

fn bytes(v: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 4) }
}

#[wasm_bindgen]
impl Viewer {
    pub fn set_scene(&mut self, name: &str) {
        self.scene = match name {
            "ground" => self.jump(STEP_GROUND),
            "planet" => self.jump(STEP_PLANET),
            "system" => self.jump(STEP_SYSTEM),
            _ => Scene::Ships,
        };
    }
    pub fn set_ship(&mut self, i: u32) {
        // The last entry (one past the blueprints) is the soldier figure.
        self.ship = (i as usize).min(self.ships.len());
    }
    pub fn set_full(&mut self, on: bool) {
        self.full = on;
    }
    /// Screen pixels per internal pixel (the chunky look); 1 is full resolution.
    pub fn set_pixel_scale(&mut self, s: u32) {
        self.pixel_scale = s.clamp(1, 8);
    }
    /// Drags the view by screen pixels: pans the ground and the system, turns ships.
    pub fn pan(&mut self, dx: f64, dy: f64) {
        let (view_h, screen_h) = self.last_view;
        let k = view_h / screen_h;
        let (dx, dy) = (dx as f32 * k, dy as f32 * k);
        let (s, c) = self.yaw.sin_cos();
        let right = Vec3::new(c, s, 0.0);
        let fwd = Vec3::new(-s, c, 0.0) / scalia_render::camera::ELEV.sin();
        let delta = -right * dx + fwd * dy;
        match self.scene {
            Scene::Ground => {
                let base = self.ground_cam.unwrap_or(self.ground_focus());
                let mut c = base + delta;
                // North and south end at the poles; east and west wrap around the planet.
                c.y = c.y.clamp(-HALF_DEPTH + 1.0, HALF_DEPTH - 1.0);
                let w = std::f32::consts::TAU * PLANET_R;
                c.x = (c.x + w / 2.0).rem_euclid(w) - w / 2.0;
                self.ground_cam = Some(c);
            }
            Scene::System => {
                let base = self.sys_cam.unwrap_or(self.sys_follow_pos());
                self.sys_follow = None;
                self.sys_cam = Some(base + delta);
            }
            Scene::Ships => {
                // Orbit: sideways turns around the model, up and down tilts between front-on and top-down.
                self.ship_yaw = Some(self.model_yaw - dx / view_h * 3.0 / self.model_zoom);
                self.model_elev = (self.model_elev + dy / view_h * 2.0 / self.model_zoom).clamp(0.02, 1.55);
            }
        }
    }
    /// Turns the camera by `d` radians.
    pub fn rotate(&mut self, d: f64) {
        match self.scene {
            Scene::Ships => self.ship_yaw = Some(self.ship_yaw.unwrap_or(0.0) + d as f32),
            _ => self.yaw += d as f32,
        }
    }
    /// Camera follows the units (ground) or the ship (system) again.
    pub fn follow(&mut self) {
        match self.scene {
            Scene::Ground => self.ground_cam = None,
            Scene::System => {
                self.sys_cam = None;
                self.sys_follow = Some(HOME_BODY);
            }
            Scene::Ships => self.ship_yaw = None,
        }
    }
    /// Cycles the speed of the day: 60-minute days (the game's real pace), 12-second days (to watch), paused.
    pub fn cycle_day_speed(&mut self, now_ms: f64) -> String {
        let t = self.sim_ms(now_ms);
        self.day_start = self.time_of_day(t);
        self.day_at = t;
        self.day_rate = if self.day_rate == DAY_RATE {
            1.0 / 12_000.0
        } else if self.day_rate > 0.0 {
            0.0
        } else {
            DAY_RATE
        };
        match self.day_rate {
            r if r == 0.0 => "time paused",
            r if r > 1.0 / 100_000.0 => "fast day",
            _ => "normal day",
        }
        .into()
    }
    /// Cycles the army size; returns a description.
    pub fn cycle_army(&mut self) -> String {
        self.army = (self.army + 1) % ARMIES.len();
        let (g, n) = ARMIES[self.army];
        self.world = build_army(g, n);
        self.add_captains();
        self.derived.clear();
        self.ground_cam = None;
        self.selected = Some(0);
        self.follow_group = 0;
        self.group_pose.clear();
        self.stance.clear();
        self.order_dest = None;
        self.agents.clear();
        self.agent_prev.clear();
        self.arrive_stance.clear();
        self.arrive_report.clear();
        format!("army: {} soldiers in {g} groups", g as u64 * n as u64)
    }
    /// Left click on the ground: selects the group under the cursor (or clears the selection).
    pub fn click(&mut self, px: f64, py: f64, now_ms: f64) -> String {
        if self.scene != Scene::Ground {
            return String::new();
        }
        let t = WorldMs(self.sim_ms(now_ms) as u64);
        let p = self.pick(px, py);
        let mut best: Option<(u32, f32)> = None;
        for i in 0..self.world.groups.len() as u32 {
            let c = self.group_centre(i, t);
            let reach = (self.world.groups.people(GroupId(i)) as f32).sqrt() * 0.9 * 0.75 + 4.0;
            let d = (Vec3::new(c.x - p.x, c.y - p.y, 0.0)).length();
            if d <= reach && best.map_or(true, |b| d < b.1) {
                best = Some((i, d));
            }
        }
        self.selected = best.map(|b| b.0);
        self.selection = self.selected.into_iter().collect();
        match self.selected {
            Some(g) => format!("selected group {} ({} soldiers)", g, self.world.groups.people(GroupId(g))),
            None => "nothing selected".into(),
        }
    }

    /// Shift + drag: selects every group whose centre lies inside the screen rectangle (projected onto the ground).
    pub fn select_box(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, now_ms: f64) -> String {
        if self.scene != Scene::Ground {
            return String::new();
        }
        let t = WorldMs(self.sim_ms(now_ms) as u64);
        let quad = [self.pick(x0, y0), self.pick(x1, y0), self.pick(x1, y1), self.pick(x0, y1)];
        let inside = |q: Vec3| {
            let mut sign = 0.0f32;
            for k in 0..4 {
                let (a, b) = (quad[k], quad[(k + 1) % 4]);
                let cross = (b.x - a.x) * (q.y - a.y) - (b.y - a.y) * (q.x - a.x);
                if cross != 0.0 {
                    if sign != 0.0 && cross.signum() != sign {
                        return false;
                    }
                    sign = cross.signum();
                }
            }
            true
        };
        self.selection = (0..self.world.groups.len() as u32).filter(|&i| inside(self.group_centre(i, t))).collect();
        self.selected = self.selection.first().copied();
        match self.selection.len() {
            0 => "nothing selected".into(),
            1 => format!("selected group {}", self.selection[0]),
            n => format!("selected {n} groups"),
        }
    }

    /// Right click on the ground: the selected group marches there (redirecting if already marching).
    pub fn command(&mut self, px: f64, py: f64, now_ms: f64, run: bool) -> String {
        if self.scene != Scene::Ground {
            return String::new();
        }
        let p = self.pick(px, py);
        self.command_world(p, now_ms, run)
    }

    /// The map point under a pixel (absolute metres), for orders issued after the radio exchange.
    pub fn map_point(&mut self, px: f64, py: f64) -> Vec<f64> {
        let p = self.pick(px, py);
        vec![p.x as f64, p.y as f64]
    }

    /// The unit under a pixel of the current view, or -1.
    pub fn unit_at(&mut self, px: f64, py: f64) -> i32 {
        if self.scene != Scene::Ground {
            return -1;
        }
        let p = self.pick(px, py);
        let t = self.world.now;
        let mut best: Option<(u32, f32)> = None;
        for i in 0..self.world.groups.len() as u32 {
            let c = self.group_centre(i, t);
            let reach = (self.world.groups.people(GroupId(i)) as f32).sqrt() * 0.9 * 0.75 + 4.0;
            let d = Vec3::new(c.x - p.x, c.y - p.y, 0.0).length();
            if d <= reach && best.map_or(true, |b| d < b.1) {
                best = Some((i, d));
            }
        }
        best.map_or(-1, |b| b.0 as i32)
    }

    /// What the selected units can do at a map point, as JSON [{id, label, ok, reason}]. Each unit's AI checks
    /// the ground: inside the testing ground it must find a way there; what the radar shows, they know.
    pub fn actions_at(&mut self, x: f64, y: f64) -> String {
        let groups = if self.selection.is_empty() { self.selected.into_iter().collect() } else { self.selection.clone() };
        if groups.is_empty() {
            return "[]".into();
        }
        let dest = ((x * 1000.0) as i64, (y * 1000.0) as i64);
        let reach = if in_base(x as f32, y as f32) {
            let g = groups[0];
            let starts: Vec<(i64, i64)> = match self.agents.get(&g) {
                Some(a) => a.positions().to_vec(),
                None => {
                    let c = self.group_centre(g, self.world.now);
                    vec![((c.x * 1000.0) as i64, (c.y * 1000.0) as i64)]
                }
            };
            AgentGroup::new(&self.base_grid, starts, dest, 1_500, 300).is_some()
        } else {
            true
        };
        let why = if reach { "" } else { "no way through to there" };
        // A captain takes goals and plans the rest himself; squads take direct orders.
        let captain = self.chain.is_captain(GroupId(groups[0]));
        let (reach, why) = if captain && self.chain.squads_of(GroupId(groups[0])).is_empty() { (false, "no squads attached") } else { (reach, why) };
        let list: &[(u8, &str)] = if captain { &[(10, "Take"), (11, "Hold")] } else { &[(0, "Move"), (1, "Defend"), (2, "Assault"), (3, "Support")] };
        let items: Vec<String> = list.iter().map(|(id, label)| format!("{{\"id\":{id},\"label\":\"{label}\",\"ok\":{reach},\"reason\":\"{why}\"}}")).collect();
        format!("[{}]", items.join(","))
    }

    /// Carries out an order at a map point (after the radio exchange): 0 move, 1 defend, 2 assault, 3 support.
    /// Assaults go at a sprint; on arrival a unit takes the stance that fits the order.
    pub fn issue_at(&mut self, action: u8, x: f64, y: f64, now_ms: f64) -> String {
        if action >= 10 {
            let Some(c) = self.selected.filter(|&c| self.chain.is_captain(GroupId(c))) else { return "select a captain first".into() };
            let kind = if action == 10 { Kind::Take } else { Kind::Hold };
            return self.assign_task(c, kind, (x, y), now_ms);
        }
        let run = action == 2;
        let stance = match action {
            1 => Pose::Cover,
            2 => Pose::StandAim,
            3 => Pose::CoverAim,
            _ => Pose::Stand,
        };
        let groups = if self.selection.is_empty() { self.selected.into_iter().collect::<Vec<_>>() } else { self.selection.clone() };
        let spot = grid(x, y);
        for g in &groups {
            self.arrive_stance.insert(*g, stance);
            self.arrive_report.insert(*g, (spot.clone(), action));
        }
        self.command_world(Vec3::new(x as f32, y as f32, 0.0), now_ms, run)
    }

    /// Names of the selected units (for the radio).
    pub fn selection_name(&self) -> String {
        let groups = if self.selection.is_empty() { self.selected.into_iter().collect::<Vec<_>>() } else { self.selection.clone() };
        let n: Vec<String> = groups.iter().map(|&g| self.call_sign(g)).collect();
        n.join(", ")
    }

    /// Puts a squad under a captain (captain -1: it answers to the commander again).
    pub fn attach(&mut self, squad: u32, captain: i32) -> String {
        if captain < 0 {
            self.chain.detach(GroupId(squad));
            return format!("{} answers to you", self.unit_name(squad));
        }
        match self.chain.attach(GroupId(squad), GroupId(captain as u32)) {
            Ok(()) => format!("{} attached to {}", self.unit_name(squad), self.unit_name(captain as u32)),
            Err(e) => format!("{e:?}"),
        }
    }

    /// Radio traffic from the units since the last call, as a JSON array of {from, text, urgent}.
    pub fn radio_events(&mut self) -> String {
        format!("[{}]", std::mem::take(&mut self.radio_out).join(","))
    }

    /// The commander holds the radio to his ear (true) or lets it go.
    pub fn set_radio(&mut self, on: bool) {
        self.radio_until = if on { f64::MAX } else { -1.0 };
    }

    fn command_world(&mut self, p: Vec3, now_ms: f64, run: bool) -> String {
        let Some(g) = self.selected else { return "select a unit first".into() };
        let t = WorldMs(self.sim_ms(now_ms) as u64);
        let now = WorldMs(t.0.max(self.world.now.0));
        let _ = self.world.tick(now);
        let speed = if run { RUN_SPEED_MM_S } else { MARCH_SPEED_MM_S };
        // Several groups keep their spacing: each goes to the click plus its offset from the selection's centre.
        let groups = if self.selection.is_empty() { vec![g] } else { self.selection.clone() };
        let centres: Vec<Vec3> = groups.iter().map(|&i| self.group_centre(i, now)).collect();
        let mid = centres.iter().fold(Vec3::ZERO, |a, c| a + *c) / centres.len() as f32;
        let mut moved = 0;
        for (&i, c) in groups.iter().zip(&centres) {
            if self.move_group(i, p + (*c - mid), speed, now) {
                moved += 1;
            }
        }
        if moved == 0 {
            return "those groups cannot move right now".into();
        }
        self.order_dest = Some(p);
        // (The radio exchange happens before the order is carried out; see the tablet.)
        // The camera stays where the player left it (F follows again).
        if self.ground_cam.is_none() {
            self.ground_cam = Some(self.cam_smooth.unwrap_or(self.focus));
        }
        let verb = if run { "sprinting" } else { "marching" };
        if groups.len() == 1 { format!("group {g} {verb} to ({:.0}, {:.0})", p.x, p.y) } else { format!("{moved} groups {verb} to ({:.0}, {:.0})", p.x, p.y) }
    }

    /// Holds pose `i` in the soldier preview (walk, run, stand, crouch, cover aim, prone, standing aim); -1 cycles.
    /// Commander movement for this frame: x to the screen's right, y to its top (-1..1 each), and sprinting.
    pub fn move_commander(&mut self, x: f32, y: f32, sprint: bool) {
        self.cmd_input = (x.clamp(-1.0, 1.0), y.clamp(-1.0, 1.0), sprint);
    }
    /// Raises or lowers the tablet. The field view stays on the commander; the map has its own camera, which
    /// starts over the commander.
    pub fn set_tablet(&mut self, up: bool) {
        if up != self.tablet && self.scene == Scene::Ground {
            let from = self.last_cine.unwrap_or((self.last_cam.0, self.last_cam.1, self.last_cam.2, scalia_render::camera::ELEV));
            self.cine = Some((up, self.frame_ms, from));
        }
        self.tablet = up;
        // Putting it away lands back close on the commander (25 m across), however the field was zoomed before.
        if !up && self.scene == Scene::Ground && !self.map_active {
            self.zoom_step = 1;
            self.zoom_m = LADDER[1];
            self.ground_view = LADDER[1];
        }
        if up && self.map_cam.ground_cam.is_none() {
            self.map_cam.ground_cam = Some(Vec3::new(self.cmd.0 as f32, self.cmd.1 as f32, 0.0));
        }
    }
    /// The commander's gait phase (0..1), or -1 when he stands: a foot lands at 0.25 (left) and 0.75 (right).
    pub fn commander_phase(&self) -> f32 {
        self.cmd_phase
    }
    /// The commander's gait for his footsteps: 0 standing, 1 walking, 2 sprinting.
    pub fn commander_gait(&self) -> u8 {
        match (self.cmd_moving, self.cmd_input.2) {
            (false, _) => 0,
            (true, false) => 1,
            (true, true) => 2,
        }
    }
    /// The fraction of the screen's height the tablet close-up's box covers: the dive frames the tablet to match.
    pub fn set_tablet_fit(&mut self, f: f32) {
        self.tablet_fit = f.clamp(0.05, 1.0);
    }
    /// Scales the tablet close-up and shifts it sideways (fraction of the screen width, positive to the right).
    pub fn set_tablet_frame(&mut self, scale: f32, shift: f32) {
        self.tablet_frame = (scale.clamp(0.3, 1.5), shift.clamp(-0.5, 0.5));
    }
    /// Where the tablet cinematic is: 0 put away, 1 raising it (camera rising and diving), 2 in front of the eyes
    /// (show the tablet's screen), 3 putting it away.
    pub fn tablet_view(&self) -> u8 {
        match self.cine {
            Some((true, start, _)) if self.frame_ms - start < CINE_OPEN_MS => 1,
            Some((false, _, _)) => 3,
            _ if self.tablet => 2,
            _ => 0,
        }
    }
    /// Attaches the tablet's map canvas: drawn with the map camera while the tablet is up.
    pub fn attach_map(&mut self, canvas: web_sys::HtmlCanvasElement) -> Result<(), JsError> {
        let size = (canvas.width().max(1), canvas.height().max(1));
        self.gpu.add_second(wgpu_target(canvas), size.0, size.1).map_err(|e| JsError::new(&e))?;
        self.resize_map(size.0, size.1);
        Ok(())
    }
    pub fn resize_map(&mut self, w: u32, h: u32) {
        let size = (w.max(1), h.max(1));
        if self.map_active { self.size = size } else { self.map_cam.size = size }
        self.gpu.resize_second(size.0, size.1);
    }
    /// Routes input to the map's camera (true) or the field view (false). Calls between a `use_map(true)` and
    /// `use_map(false)` (wheel, pan, click, command) act on the map.
    pub fn use_map(&mut self, on: bool) {
        if on != self.map_active {
            self.swap_cam();
            self.map_active = on;
        }
    }
    /// How orders are carried out: 0 calm, 1 aggressive.
    pub fn set_manner(&mut self, m: u8) {
        self.manner = m.min(1);
    }
    /// The units under command, as JSON: [{id, name, soldiers, status, stance, selected}].
    pub fn roster(&self) -> String {
        let mut out = Vec::new();
        for i in 0..self.world.groups.len() as u32 {
            let name = self.unit_name(i);
            let captain = self.chain.is_captain(GroupId(i));
            let boss = self.chain.superior(GroupId(i)).map_or(-1, |c| c.0 as i64);
            let task = match self.tasks.get(&i) {
                Some((t, _, _)) => format!("{:?} grid {}: {}", t.kind, grid(t.objective.0 as f64 / 1000.0, t.objective.1 as f64 / 1000.0), t.current().map_or("done".into(), |s| format!("{s:?}"))),
                None => String::new(),
            };
            let soldiers = self.world.groups.people(GroupId(i));
            let status = match self.agents.get(&i) {
                Some(a) if !a.all_arrived() => "moving",
                _ if self.world.groups.process(GroupId(i)).is_running_at(self.world.now) => "marching",
                _ => "holding",
            };
            let stance = format!("{:?}", self.stance.get(&i).copied().unwrap_or(Pose::Stand)).to_lowercase();
            out.push(format!(
                "{{\"id\":{i},\"name\":\"{name}\",\"captain\":{captain},\"boss\":{boss},\"task\":\"{task}\",\"soldiers\":{soldiers},\"status\":\"{status}\",\"stance\":\"{stance}\",\"selected\":{}}}",
                self.selection.contains(&i)
            ));
        }
        format!("[{}]", out.join(","))
    }
    /// Every unit as seen by the camera that is active right now (the field camera, or the tablet map's inside
    /// `use_map(true)`): JSON [{id, cs, n, x, y, r, sel, you}] with x, y as fractions of the canvas (0..1, y down) and r
    /// the unit's radius as a fraction of the canvas height. The commander is id -1. Units off screen are left out.
    pub fn unit_marks(&mut self) -> String {
        let (target, yaw, view_h) = self.last_cam;
        let (r, u, _) = scalia_render::camera::basis(yaw);
        let aspect = self.size.0 as f32 / self.size.1.max(1) as f32;
        let mut units: Vec<(i64, String, u32, f64, f64, f32, bool)> = Vec::new();
        for i in 0..self.world.groups.len() as u32 {
            let c = self.group_centre(i, self.world.now);
            let rad = (self.world.groups.people(GroupId(i)) as f32).sqrt() * 0.675 + 1.5;
            units.push((i as i64, self.call_sign(i), i + 1, c.x as f64, c.y as f64, rad, self.selection.contains(&i)));
        }
        units.push((-1, "Actual".into(), 0, self.cmd.0, self.cmd.1, 1.2, false));
        let mut out = Vec::new();
        for (id, cs, n, ax, ay, rad_m, sel) in units {
            let rel = Vec3::new((ax - self.origin.0) as f32, (ay - self.origin.1) as f32, self.height_at(ax as f32, ay as f32));
            let d = rel - target;
            let (x, y) = (0.5 + d.dot(r) / (view_h * aspect), 0.5 - d.dot(u) / view_h);
            if !(-0.1..=1.1).contains(&x) || !(-0.1..=1.1).contains(&y) { continue; }
            out.push(format!("{{\"id\":{id},\"cs\":\"{cs}\",\"n\":{n},\"x\":{x:.4},\"y\":{y:.4},\"r\":{:.4},\"sel\":{sel},\"you\":{}}}", rad_m / view_h, id < 0));
        }
        format!("[{}]", out.join(","))
    }
    /// Selects exactly one unit without moving the camera (clicking its marker on the tablet map).
    pub fn select_only(&mut self, id: i32) {
        if id >= 0 && (id as u32) < self.world.groups.len() as u32 {
            self.selected = Some(id as u32);
            self.selection = vec![id as u32];
        } else {
            self.selected = None;
            self.selection.clear();
        }
    }
    /// The tablet map glides back over the commander, close in (from wherever it was, the planet or
    /// the star system included).
    pub fn map_center_commander(&mut self) {
        let was = self.map_active;
        self.use_map(true);
        self.ground_cam = Some(Vec3::new(self.cmd.0 as f32, self.cmd.1 as f32, 0.0));
        self.sys_follow = Some(HOME_BODY);
        // Close enough to see him standing in his ring (25 m across).
        self.zoom_step = 1;
        self.use_map(was);
    }
    /// Takes command from a unit: the commander moves to stand with it, facing the way it faces.
    pub fn command_unit(&mut self, id: u32) -> String {
        if id >= self.world.groups.len() as u32 {
            return String::new();
        }
        let c = self.group_centre(id, self.world.now);
        self.cmd = (c.x as f64, c.y as f64);
        self.map_center_commander();
        format!("commanding {}", self.call_sign(id))
    }
    /// Selects one unit (from the roster) and centres the tablet's map on it.
    pub fn select_unit(&mut self, id: u32) {
        if id < self.world.groups.len() as u32 {
            self.selected = Some(id);
            self.selection = vec![id];
            let c = self.group_centre(id, self.world.now);
            self.ground_cam = Some(c);
        }
    }
    /// Model viewer: spin on a turntable (true) or hold the angle it was left at.
    pub fn set_turntable(&mut self, on: bool) {
        self.ship_yaw = if on { None } else { Some(self.model_yaw) };
    }
    /// Model viewer: sun position as a time of day (0.25 sunrise, 0.5 noon, 0.75 sunset).
    pub fn set_model_light(&mut self, t: f32) {
        self.model_light = t.clamp(0.2, 0.8);
    }
    /// Model viewer: back to the isometric angle, framed, spinning.
    pub fn reset_model_view(&mut self) {
        self.model_elev = scalia_render::camera::ELEV;
        self.model_zoom = 1.0;
        self.ship_yaw = None;
    }
    /// Number of models the viewer can show (the blueprints, then the soldier last).
    pub fn model_count(&self) -> u32 {
        self.ships.len() as u32 + 1
    }
    /// Model viewer: level of detail of the soldier (0 full, 1 mid, 2 far).
    pub fn set_model_detail(&mut self, d: u8) {
        self.model_detail = d.min(2);
    }
    /// Model viewer: colour variant (an index into `variant_name`).
    pub fn set_model_variant(&mut self, i: u8) {
        self.model_variant = i;
    }
    pub fn variant_name(&self, i: u8) -> Option<String> {
        VARIANTS.get(i as usize).map(|v| v.0.to_string())
    }
    pub fn preview_pose(&mut self, i: i32) {
        self.preview_pose = (i >= 0).then_some(i as usize);
    }

    /// Cycles the selected group's idle stance: stand, crouch, cover aim, prone, standing aim.
    pub fn cycle_stance(&mut self) -> String {
        let Some(g) = self.selected else { return "select a group first".into() };
        let next = match self.stance.get(&g).copied().unwrap_or(Pose::Stand) {
            Pose::Stand => Pose::Crouch,
            Pose::Crouch => Pose::Cover,
            Pose::Cover => Pose::CoverAim,
            Pose::CoverAim => Pose::Prone,
            Pose::Prone => Pose::StandAim,
            _ => Pose::Stand,
        };
        self.stance.insert(g, next);
        format!("group {g} stance: {next:?}")
    }

    /// Numbers for the performance panel: [fps, internal width, internal height,
    /// gpu shadow ms, gpu scene ms, gpu upscale ms] (gpu values are -1 when the browser gives no timestamps).
    pub fn perf(&self) -> Vec<f32> {
        let g = self.gpu.gpu_ms().unwrap_or([-1.0; 3]);
        vec![self.fps, self.last_internal.0 as f32, self.last_internal.1 as f32, g[0], g[1], g[2]]
    }

    pub fn scene_name(&self) -> String {
        match self.scene {
            Scene::Ships => "ships",
            Scene::Ground if self.ground_view > 100_000.0 => "planet",
            Scene::Ground => "ground",
            Scene::System => "system",
        }
        .into()
    }
    pub fn set_outline(&mut self, on: bool) {
        self.outline = on;
    }
    pub fn resize(&mut self, w: u32, h: u32) {
        self.size = (w.max(1), h.max(1));
        self.gpu.resize(self.size.0, self.size.1);
    }
    /// Mouse wheel: one call is one step of the zoom ladder (positive zooms out); the camera glides there.
    /// One zoom step in or out; true when the view actually changed (not already at the end of the ladder).
    pub fn wheel(&mut self, delta: f64) -> bool {
        if self.scene == Scene::Ships {
            // The model viewer zooms freely.
            self.model_zoom = (self.model_zoom * if delta > 0.0 { 1.15 } else { 1.0 / 1.15 }).clamp(0.15, 6.0);
            return true;
        }
        if delta == 0.0 {
            return false;
        }
        // Away from home the ground is not under the camera: the system's closest step is as far in as it goes.
        let lowest = if self.scene == Scene::System && self.sys_follow != Some(HOME_BODY) { STEP_HANDOFF + 1 } else { 0 };
        // On foot the view stays close; the tablet's map zooms out to the stars.
        let highest = if self.map_active { LADDER.len() - 1 } else { STEP_FIELD_MAX };
        let was = self.zoom_step;
        self.zoom_step = if delta > 0.0 { (self.zoom_step + 1).min(highest) } else { self.zoom_step.saturating_sub(1).max(lowest) };
        self.zoom_step != was
    }
    pub fn flying(&self) -> bool {
        self.flight.is_some()
    }
    /// Starts a flight from the docked body to the other one; ignored while flying.
    pub fn launch(&mut self, now_ms: f64) {
        if self.flight.is_some() {
            return;
        }
        let t = WorldMs(self.sim_ms(now_ms) as u64);
        let to = 1 - self.docked;
        let from = match self.system.body_pos(BodyId(self.docked), t) {
            Some(p) => p.offset(PARK_OFFSET.0, PARK_OFFSET.1, PARK_OFFSET.2),
            None => return,
        };
        if let Ok(plan) = plan_flight(&self.system, from, BodyId(to), PARK_OFFSET, DEMO_PEAK_SPEED_MM_S, t) {
            self.flight = Some((plan, to));
            self.trail.clear();
        }
    }

    /// Draws one frame and returns the status line.
    pub fn frame(&mut self, now_ms: f64) -> String {
        self.frames += 1;
        if now_ms - self.fps_t > 500.0 {
            self.fps = (self.frames as f64 * 1000.0 / (now_ms - self.fps_t)) as f32;
            self.frames = 0;
            self.fps_t = now_ms;
        }
        self.frame_ms = now_ms;
        self.use_map(false);
        let line = self.render(now_ms, false);
        // Picture in picture: the tablet's map, drawn with its own camera onto its own canvas.
        if self.tablet && self.gpu.has_second() {
            self.use_map(true);
            self.map_pass = true;
            let _ = self.render(now_ms, true);
            self.map_pass = false;
            self.use_map(false);
        }
        line
    }
}

impl Viewer {
    fn render(&mut self, now_ms: f64, second: bool) -> String {
        let t = self.sim_ms(now_ms);
        let ps = if self.full { 1 } else { self.pixel_scale };
        let internal = ((self.size.0 / ps).max(1), (self.size.1 / ps).max(1));
        self.glide(now_ms);
        let (mut meshes, mut instanced, mut decor) = (Vec::new(), Vec::new(), Vec::new());
        let (target, yaw_aim, view_height, range, text) = match self.scene {
            Scene::Ships => self.ships_scene(t, &mut meshes, &mut instanced),
            Scene::Ground => self.ground_scene(t, &mut meshes, &mut instanced, &mut decor),
            Scene::System => self.system_scene(t, &mut meshes),
        };
        // Rotation eases too (the model viewer turns directly: it is dragged by hand).
        let yaw = if self.scene == Scene::Ships {
            yaw_aim
        } else {
            let k = 1.0 - (-(16.0f32) / 120.0).exp();
            self.yaw_smooth += (yaw_aim - self.yaw_smooth) * k;
            self.yaw_smooth
        };
        // The tablet cinematic takes the main camera over: up above the commander, then down onto his hands.
        let (target, yaw, view_height, cine_elev) = match (self.scene, second) {
            (Scene::Ground, false) => match self.cine_cam(now_ms, (target, yaw, view_height)) {
                Some(c) => {
                    self.last_cine = Some(c);
                    (c.0, c.1, c.2, Some(c.3))
                }
                None => {
                    self.last_cine = None;
                    (target, yaw, view_height, None)
                }
            },
            _ => (target, yaw, view_height, None),
        };
        self.last_view = (view_height, self.size.1 as f32);
        self.last_cam = (target, yaw, view_height);
        let tod = self.time_of_day(t) as f32;
        let (sky_now, atmosphere) = match self.scene {
            // Close to home the planet keeps the ground's light, so the hand-over does not flash.
            Scene::System if self.sys_follow == Some(HOME_BODY) && self.sys_scale < 50.0 => (sky(tod), false),
            Scene::System => (sky(0.5), false),
            Scene::Ground => (sky(tod), self.ground_view < 40_000.0),
            // The model viewer always shows late-morning light, whatever the time of day outside.
            Scene::Ships => (sky(self.model_light), true),
        };
        let bend = if self.scene == Scene::Ground { self.ground_bend() } else { Bend::FLAT };
        let elev = if self.scene == Scene::Ships { self.model_elev } else { cine_elev.unwrap_or(scalia_render::camera::ELEV) };
        // Outlines on terrain cells only a few pixels wide turn into shimmering stripes: off for far views.
        let far_view = match self.scene {
            Scene::Ground => self.ground_view > 20_000.0,
            Scene::System => self.sys_scale < 50.0,
            Scene::Ships => false,
        };
        // Wrap once a day: f32 seconds stay precise to about 8 ms, and the jump comes only every 24 hours.
        let time_s = ((t / 1000.0) % 86_400.0) as f32;
        let desc = FrameDesc { meshes: &meshes, instanced: &instanced, decor: &decor, target, yaw, view_height, range, internal, outline: self.outline && !far_view, sky: sky_now, atmosphere, time_s, bend, elev };
        let drawn = if second { self.gpu.frame_second(&desc) } else { self.gpu.frame(&desc) };
        if let Err(e) = drawn {
            return format!("Render error: {e}");
        }
        self.last_internal = internal;
        let time = if self.scene == Scene::System { String::new() } else { format!(" | {}", clock(tod)) };
        format!("{text}{time}")
    }
}

impl Viewer {
    /// The ground point under a screen pixel: through the orthographic camera, onto the terrain.
    fn pick(&mut self, px: f64, py: f64) -> Vec3 {
        let (target, yaw, view_h) = self.last_cam;
        let (w, h) = (self.size.0 as f32, self.size.1 as f32);
        let (nx, ny) = (2.0 * px as f32 / w - 1.0, 1.0 - 2.0 * py as f32 / h);
        let (r, u, to_cam) = scalia_render::camera::basis(yaw);
        let on_plane = target + r * (nx * view_h / 2.0 * w / h) + u * (ny * view_h / 2.0);
        // Slide along the view ray until it meets the ground (a few refinements over the heightfield).
        // The camera target is relative to the floating origin; heights and the answer are absolute.
        let o = Vec3::new(self.origin.0 as f32, self.origin.1 as f32, 0.0);
        let mut q = on_plane;
        for _ in 0..5 {
            let ground = self.height_at(q.x + o.x, q.y + o.y);
            let s = (on_plane.z - ground) / to_cam.z;
            q = on_plane - to_cam * s;
        }
        q + o
    }

    /// Where a group's formation is centred at time t (render metres).
    /// The testing ground's captains: two officers, each his own unit, answering to the commander.
    fn add_captains(&mut self) {
        self.chain = Command::default();
        self.tasks.clear();
        if self.world.groups.len() > 20 || self.world.groups.is_empty() {
            return;
        }
        for k in 0..CAPTAINS.len() as i64 {
            let pos = Pos { region: RegionId(1), x: 52_000 + k * 28_000, y: 30_000, z: 0 };
            let spec = GroupSpec { pos, composition: Composition([1, 0, 0, 0]), needs: Needs::full() };
            let id = self.world.groups.spawn(spec, &mut self.world.roster);
            self.chain.add_captain(id);
        }
    }

    /// The unit's name on the roster.
    fn unit_name(&self, g: u32) -> String {
        let caps = self.chain.captains();
        if let Some(k) = caps.iter().position(|c| c.0 == g) {
            return format!("Capt. {}", CAPTAINS[k % CAPTAINS.len()]);
        }
        match SQUADS.get(g as usize) { Some(n) => format!("{n} squad"), None => format!("Group {g}") }
    }

    /// The unit's call sign on the radio.
    fn call_sign(&self, g: u32) -> String {
        let caps = self.chain.captains();
        if let Some(k) = caps.iter().position(|c| c.0 == g) {
            return CAPTAINS[k % CAPTAINS.len()].to_string();
        }
        SQUADS.get(g as usize).map_or(format!("Group {g}"), |n| n.to_string())
    }

    fn radio(&mut self, from: u32, text: String, urgent: bool) {
        let from = self.call_sign(from);
        self.radio_out.push(format!("{{\"from\":\"{from}\",\"text\":\"{}\",\"urgent\":{urgent}}}", text.replace('"', "'")));
    }

    /// Sends one group to a point (metres), as agents inside the testing ground, marching outside it.
    fn move_group(&mut self, i: u32, d: Vec3, speed: u32, now: WorldMs) -> bool {
        let to = Pos { region: RegionId(1), x: (d.x as f64 * 1000.0) as i64, y: (d.y as f64 * 1000.0) as i64, z: 0 };
        // Inside the testing ground the soldiers walk as agents round the obstacles.
        if in_base(d.x, d.y) {
            let starts: Vec<(i64, i64)> = match self.agents.get(&i) {
                Some(a) => a.positions().to_vec(),
                None => {
                    let mut mine: Vec<&Derived> = self.derived.iter().filter(|s| s.group.0 == i).collect();
                    mine.sort_by_key(|s| s.slot);
                    let v: Vec<(i64, i64)> = mine.iter().map(|s| (s.pos.x, s.pos.y)).collect();
                    if v.is_empty() {
                        let p = self.world.groups.pos(GroupId(i));
                        vec![(p.x, p.y)]
                    } else {
                        v
                    }
                }
            };
            if starts.iter().all(|s| in_base(s.0 as f32 / 1000.0, s.1 as f32 / 1000.0)) {
                if let Some(a) = AgentGroup::new(&self.base_grid, starts, (to.x, to.y), speed as i64, 300) {
                    self.agent_prev.insert(i, a.positions().to_vec());
                    self.agents.insert(i, a);
                    return true;
                }
                // Inside the base soldiers only ever walk as agents: never a straight line through walls.
                return false;
            }
        }
        if self.world.order_march(GroupId(i), to, speed, now).is_ok() {
            self.agents.remove(&i);
            return true;
        }
        false
    }

    /// Where a group is now (mm).
    fn group_mm(&self, g: u32) -> (i64, i64) {
        match self.agents.get(&g) {
            Some(a) => a.centre(),
            None => {
                let c = self.group_centre(g, self.world.now);
                ((c.x as f64 * 1000.0) as i64, (c.y as f64 * 1000.0) as i64)
            }
        }
    }

    /// Hands a captain a goal: he plans it (GOAP), acknowledges, and starts the first step.
    fn assign_task(&mut self, c: u32, kind: Kind, at: (f64, f64), now_ms: f64) -> String {
        let squads = self.chain.squads_of(GroupId(c));
        if squads.is_empty() {
            return "that captain has no squads attached".into();
        }
        let objective = ((at.0 * 1000.0) as i64, (at.1 * 1000.0) as i64);
        // No enemy forces exist yet, so the radar shows none.
        let Some(task) = Task::new(kind, objective, false) else { return "the captain sees no way to do that".into() };
        let plan: Vec<String> = task.plan.iter().map(|s| format!("{s:?}")).collect();
        self.tasks.insert(c, (task, Vec::new(), now_ms));
        self.start_step(c, now_ms);
        format!("{} plans: {}", self.unit_name(c), plan.join(" > "))
    }

    /// Gives the captain's squads their orders for the current step, and moves the captain to his post behind them.
    fn start_step(&mut self, c: u32, now_ms: f64) {
        let Some((task, _, _)) = self.tasks.get(&c).cloned() else { return };
        let Some(step) = task.current() else { return };
        let squads: Vec<(GroupId, (i64, i64))> = self.chain.squads_of(GroupId(c)).into_iter().map(|g| (g, self.group_mm(g.0))).collect();
        let orders = chain_of::step_orders(step, task.objective, &squads);
        let now = self.world.now;
        let mut waiting = Vec::new();
        let mut targets = Vec::new();
        for o in &orders {
            targets.push(o.to);
            let here = self.group_mm(o.squad.0);
            let pose = pose_of(o.stance);
            if (o.to.0 - here.0).abs() + (o.to.1 - here.1).abs() > 2_000 {
                let speed = if o.run { RUN_SPEED_MM_S } else { MARCH_SPEED_MM_S };
                let d = Vec3::new(o.to.0 as f32 / 1000.0, o.to.1 as f32 / 1000.0, 0.0);
                if self.move_group(o.squad.0, d, speed, now) {
                    self.arrive_stance.insert(o.squad.0, pose);
                    waiting.push(o.squad.0);
                    continue;
                }
            }
            self.stance.insert(o.squad.0, pose);
        }
        // Squads that did not move (support, dig in) keep their place; the captain watches from behind.
        let post = chain_of::captain_post(task.objective, &targets);
        let here = self.group_mm(c);
        if (post.0 - here.0).abs() + (post.1 - here.1).abs() > 3_000 && self.move_group(c, Vec3::new(post.0 as f32 / 1000.0, post.1 as f32 / 1000.0, 0.0), MARCH_SPEED_MM_S, now) {
            self.arrive_stance.insert(c, Pose::Crouch);
        }
        let names: Vec<String> = self.chain.squads_of(GroupId(c)).iter().map(|g| self.call_sign(g.0)).collect();
        let g = grid(task.objective.0 as f64 / 1000.0, task.objective.1 as f64 / 1000.0);
        let support = names.first().cloned().unwrap_or_default();
        let report = match (step, task.at) {
            (Step::Approach, 0) | (Step::Occupy, 0) => None, // the acknowledgement covers it
            (Step::Advance, _) => Some((format!("Actual, {{me}}. At the line of departure, no contact. Advancing on grid {g}. Out."), false)),
            (Step::Suppress, _) => Some((format!("Actual, {{me}}. Contact at grid {g}. {support} laying down fire, assault going in. Out."), true)),
            (Step::Assault, _) => Some(("Assaulting now!".to_string(), true)),
            (Step::DigIn, _) => Some((format!("Actual, {{me}}. On the objective. Digging in at grid {g}. Out."), false)),
            _ => None,
        };
        if let Some((text, urgent)) = report {
            let me = self.call_sign(c);
            self.radio(c, text.replace("{me}", &me), urgent);
        }
        if let Some(entry) = self.tasks.get_mut(&c) {
            entry.1 = waiting;
            entry.2 = now_ms;
        }
    }

    /// Each captain checks on his squads: once they are all where the step sent them (and a moment has passed),
    /// the step is done and the next begins; at the end he reports to the commander.
    fn run_captains(&mut self, now_ms: f64) {
        let caps: Vec<u32> = self.tasks.keys().copied().collect();
        for c in caps {
            let (ready, done_after) = {
                let (task, waiting, since) = &self.tasks[&c];
                let arrived = waiting.iter().all(|g| self.agents.get(g).is_none_or(|a| a.all_arrived()) && !self.world.groups.process(GroupId(*g)).is_running_at(self.world.now));
                (arrived && now_ms - since >= STEP_MIN_MS, task.at + 1 >= task.plan.len())
            };
            if !ready {
                continue;
            }
            let entry = self.tasks.get_mut(&c).expect("present");
            entry.0.advance();
            if done_after {
                let (task, _, _) = self.tasks.remove(&c).expect("present");
                let g = grid(task.objective.0 as f64 / 1000.0, task.objective.1 as f64 / 1000.0);
                let me = self.call_sign(c);
                let text = match task.kind {
                    Kind::Take => format!("Actual, {me}. Grid {g} taken, no casualties. Squads holding. Over."),
                    Kind::Hold => format!("Actual, {me}. In position, holding grid {g}. Out."),
                };
                self.radio(c, text, false);
            } else {
                self.start_step(c, now_ms);
            }
        }
    }

    fn group_centre(&self, g: u32, t: WorldMs) -> Vec3 {
        if let Some(a) = self.agents.get(&g) {
            let p = a.positions();
            let (sx, sy) = p.iter().fold((0i64, 0i64), |s, q| (s.0 + q.0, s.1 + q.1));
            return Vec3::new(sx as f32 / p.len() as f32 / 1000.0, sy as f32 / p.len() as f32 / 1000.0, 0.0);
        }
        let id = GroupId(g);
        let p = scalia_core::process::process_pos(self.world.groups.process(id), t).unwrap_or(self.world.groups.pos(id));
        Vec3::new(p.x as f32 / 1000.0, p.y as f32 / 1000.0, 0.0)
    }

    /// Position (thousand km) of the followed body, or the star.
    fn sys_follow_pos(&self) -> Vec3 {
        match self.sys_follow.and_then(|b| self.system.body_pos(BodyId(b), self.sys_now)) {
            Some(p) => Vec3::new((p.x as f64 / KM) as f32, (p.y as f64 / KM) as f32, 0.0),
            None => Vec3::ZERO,
        }
    }

    fn time_of_day(&self, t: f64) -> f64 {
        (self.day_start + (t - self.day_at) * self.day_rate).rem_euclid(1.0)
    }

    fn ground_focus(&self) -> Vec3 {
        self.focus
    }

    fn sim_ms(&mut self, now_ms: f64) -> f64 {
        let t0 = *self.t0.get_or_insert(now_ms);
        (now_ms - t0).max(0.0)
    }

    fn ship_info(&mut self, i: usize) -> &ShipInfo {
        if self.ships[i].is_none() {
            let b = scalia_blueprint::parse(scalia_blueprint::shipped::SHIPPED_RON[i]).expect("shipped blueprints are valid");
            let m = scalia_blueprint::mesh::mesh(&b);
            let mut mb = MeshBuilder::default();
            let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
            for (k, tri) in m.indices.chunks(3).enumerate() {
                let p: Vec<[f32; 3]> = tri
                    .iter()
                    .map(|&j| {
                        let v = m.vertices[j as usize];
                        let q = Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32) / 1000.0;
                        lo = lo.min(q);
                        hi = hi.max(q);
                        q.to_array()
                    })
                    .collect();
                mb.tri(p[0], p[1], p[2], m.colors[k] as f32);
            }
            self.gpu.upload(&format!("ship{i}"), mb.bytes());
            self.ships[i] = Some(ShipInfo { name: b.name.clone(), triangles: m.indices.len() / 3, lo, hi });
        }
        self.ships[i].as_ref().expect("filled")
    }

    fn ships_scene(&mut self, t: f64, meshes: &mut Vec<String>, instanced: &mut Vec<(String, String)>) -> (Vec3, f32, f32, f32, String) {
        let i = self.ship;
        if i == self.ships.len() {
            // Soldier figure walking in place, on a small plinth.
            let mut plinth = MeshBuilder::default();
            plinth.cuboid([-1.0, -1.0, -0.1], [1.0, 1.0, 0.0], 12.0);
            self.gpu.upload("plinth", plinth.bytes());
            // Every 3 seconds the next pose: walk, run, stand, crouch, cover aim, prone, standing aim.
            const SHOW: [(Pose, f32); 9] = [
                (Pose::Walk, 0.9), (Pose::Run, 1.35), (Pose::Stand, 0.0), (Pose::Crouch, 0.0),
                (Pose::CoverAim, 0.0), (Pose::Prone, 0.0), (Pose::StandAim, 0.0), (Pose::Sprint, 1.45),
                (Pose::Cover, 0.0),
            ];
            // Entry 9 is "walk while aiming": the walk with an aiming upper body layered on top.
            let walk_aim = self.preview_pose == Some(9);
            let (pose, steps) = if walk_aim { (Pose::Walk, 0.9) } else { SHOW[self.preview_pose.unwrap_or((t / 3000.0) as usize) % SHOW.len()] };
            let ts = ((t / 1000.0) % 86_400.0) as f32;
            let prev = if self.preview_pose.is_none() { SHOW[((t / 3000.0) as usize + SHOW.len() - 1) % SHOW.len()].0 } else { pose };
            let changed = ((t / 3000.0).floor() * 3.0) as f32 % 86_400.0;
            let colour = VARIANTS[self.model_variant as usize % VARIANTS.len()].1;
            self.gpu.upload_instances("figure1", bytes(&[0.0, 0.0, 0.0, colour, 0.0, 0.0, steps, pose as u32 as f32, prev as u32 as f32, changed.min(ts), if walk_aim { Pose::StandAim as u32 as f32 + 1.0 } else { 0.0 }, 0.0]));
            meshes.push("plinth".into());
            let (mesh, tris, detail) = match self.model_detail {
                1 => ("figure_lo", figure_lo().vertices.len() / 3, "mid detail, 70-250 m"),
                2 => ("figure_far", figure_far().vertices.len() / 3, "far detail, beyond 250 m"),
                _ => ("figure", figure().vertices.len() / 3, "full detail, up to 70 m"),
            };
            instanced.push((mesh.into(), "figure1".into()));
            let yaw = self.ship_yaw.unwrap_or((t / 2500.0) as f32);
            let variant = VARIANTS[self.model_variant as usize % VARIANTS.len()].0;
            let cycling = if self.preview_pose.is_none() { " (poses change every 3 s)" } else { "" };
            // A prone soldier lies 2.5 m forward from where he stood: centre on him and pull back.
            let (centre, size) = if pose == Pose::Prone { (Vec3::new(0.0, 1.2, 0.3), 3.4) } else { (Vec3::new(0.0, 0.0, 0.9), 2.6) };
            self.model_yaw = yaw;
            return (centre, yaw, size * self.model_zoom, 10.0, format!("Soldier, {variant}: {tris} triangles, {detail}, pose {pose:?}{cycling}"));
        }
        let s = self.ship_info(i);
        let ext = s.hi - s.lo;
        let size = ext.max_element();
        let text = format!("{}: {} triangles, {:.1} m long", s.name, s.triangles, ext.y);
        let centre = (s.lo + s.hi) / 2.0;
        meshes.push(format!("ship{i}"));
        let yaw = self.ship_yaw.unwrap_or((t / 2500.0) as f32);
        self.model_yaw = yaw;
        (centre, yaw, size * self.model_zoom, size * 3.0, text)
    }

    /// Heights of a chunk in metres, generated once from the planet.
    fn chunk(&mut self, cx: i64, cy: i64) -> &Vec<f32> {
        self.chunks
            .entry((cx, cy))
            .or_insert_with(|| {
                let mut h: Vec<f32> = Planet::demo(42).chunk(cx, cy + EQUATOR_CHUNK, CELLS).heights.iter().map(|&h| h as f32 / 1000.0).collect();
                level_base(&mut h, cx, cy);
                h
            })
    }

    /// Builds (or rebuilds) a chunk's mesh with every `step`-th sample: 1 close up, 2 or 4 zoomed out.
    fn chunk_mesh(&mut self, cx: i64, cy: i64, step: usize) {
        let n = CELLS as usize + 1;
        let hm = self.chunk(cx, cy).clone();
        let mut m = MeshBuilder::default();
        let base = (0.0f32, 0.0f32); // chunk-local; an instance offset places it relative to the origin
        let p = |i: usize, j: usize| [base.0 + i as f32, base.1 + j as f32, hm[j * n + i]];
        for j in (0..CELLS as usize).step_by(step) {
            for i in (0..CELLS as usize).step_by(step) {
                let (a, b, c, d) = (p(i, j), p(i + step, j), p(i + step, j + step), p(i, j + step));
                // Contour bands from the quad's mean height, BAND_M apart, only at the close-up step: further out a
                // band is narrower than a pixel on any slope and the bands interfere into rings. There the ground
                // takes its broad height zone instead.
                let mean = (a[2] + b[2] + c[2] + d[2]) / 4.0;
                let col = if step <= 2 { 10.0 + (mean / BAND_M).floor().rem_euclid(4.0) } else { zone_colour(mean) };
                m.tri(a, b, c, col);
                m.tri(a, c, d, col);
            }
        }
        self.gpu.upload(&format!("terrain{cx},{cy}"), m.bytes());
        self.chunk_lod.insert((cx, cy), step);
        self.place_chunk(cx, cy);
        if !self.gpu.has(&format!("grass{cx},{cy}")) {
            self.gpu.upload_instances(&format!("grass{cx},{cy}"), bytes(&grass_for(&hm, cx, cy, self.origin)));
        }
    }

    /// Ground height under (x, y), on the same triangles the terrain is drawn with (coarser when zoomed out),
    /// so nothing floats above or sinks into the drawn ground.
    fn height_at(&mut self, x: f32, y: f32) -> f32 {
        let step = self.terrain_step.max(1);
        let (cx, cy) = ((x / CELLS as f32).floor() as i64, (y / CELLS as f32).floor() as i64);
        let n = CELLS as usize + 1;
        let (lx, ly) = (x - cx as f32 * CELLS as f32, y - cy as f32 * CELLS as f32);
        let cell = |v: f32| (((v / step as f32).floor() as usize) * step).min(CELLS as usize - step);
        let (i, j) = (cell(lx), cell(ly));
        let (fx, fy) = (((lx - i as f32) / step as f32).clamp(0.0, 1.0), ((ly - j as f32) / step as f32).clamp(0.0, 1.0));
        let hm = self.chunk(cx, cy);
        let h = |a: usize, b: usize| hm[b * n + a];
        let (a, b, c, d) = (h(i, j), h(i + step, j), h(i + step, j + step), h(i, j + step));
        if fx >= fy { a + (b - a) * fx + (c - b) * fy } else { a + (d - a) * fy + (c - d) * fx }
    }

    fn ground_scene(
        &mut self,
        t: f64,
        meshes: &mut Vec<String>,
        instanced: &mut Vec<(String, String)>,
        decor: &mut Vec<(String, String)>,
    ) -> (Vec3, f32, f32, f32, String) {
        let now = WorldMs(t as u64);
        let tick = WorldMs(now.0.max(self.world.now.0));
        let _ = self.world.tick(tick);
        // Keep the floating origin within half a kilometre of the camera.
        let tg = self.ground_cam.unwrap_or(self.focus);
        let o = ((tg.x as f64 / 1024.0).round() * 1024.0, (tg.y as f64 / 1024.0).round() * 1024.0);
        if o != self.origin && !self.map_pass {
            self.origin = o;
            self.reorigin();
        }
        // Only soldiers near the camera are derived; everyone else stays an aggregate.
        let look = self.ground_cam.unwrap_or(self.focus);
        let r = (self.ground_view as f64 * 1.6 * 1000.0) as i64;
        let (lx, ly) = ((look.x as f64 * 1000.0) as i64, (look.y as f64 * 1000.0) as i64);
        let view = ViewBox { region: RegionId(1), min_x: lx - r, max_x: lx + r, min_y: ly - r, max_y: ly + r };
        let mut out = std::mem::take(&mut self.derived);
        if self.ground_view <= UNITS_MAX {
            derive_visible(&self.world, tick, &view, MAX_DRAWN, &mut out);
        } else {
            out.clear();
        }
        let n = out.len();
        // Follow the first group: its formation centre at this moment.
        // Follow the chosen group: its formation centre at this moment.
        let lead = self.follow_group.min(self.world.groups.len().saturating_sub(1) as u32);
        let c0 = self.group_centre(lead, tick);
        let (cx, cy) = (c0.x as f64, c0.y as f64);
        let ts = ((t / 1000.0) % 86_400.0) as f32;
        let mut inst = Vec::with_capacity(n * 12);
        self.step_agents(t);
        for d in &out {
            if self.agents.contains_key(&d.group.0) {
                continue; // drawn from their agents below
            }
            let (ax, ay) = (d.pos.x as f64 / 1000.0, d.pos.y as f64 / 1000.0);
            let z = self.height_at(ax as f32, ay as f32);
            let (x, y) = ((ax - self.origin.0) as f32, (ay - self.origin.1) as f32);
            if !self.on_screen(x, y, z, 3.0) {
                continue;
            }
            // Facing: along the march, else the formation's front with a little personal variation.
            let jitter = (scalia_core::hash::mix64(d.slot as u64 ^ 0x5ca1) % 1000) as f32 / 1000.0;
            let heading = match self.world.groups.process(d.group).kind {
                ProcessKind::March { from, to, .. } => ((from.x - to.x) as f32).atan2((to.y - from.y) as f32),
                _ => (jitter - 0.5) * 0.4,
            };
            let walking = d.phase == Phase::Marching || d.phase == Phase::Walking || d.phase == Phase::Climbing;
            let running = walking && matches!(self.world.groups.process(d.group).kind, ProcessKind::March { speed_mm_s, .. } if speed_mm_s >= RUN_SPEED_MM_S);
            let (pose, steps) = if running {
                (Pose::Sprint, 1.45 + jitter * 0.1)
            } else if walking {
                (Pose::Walk, 0.9 + jitter * 0.1)
            } else {
                (self.stance.get(&d.group.0).copied().unwrap_or(Pose::Stand), 0.0)
            };
            let body = if self.selection.contains(&d.group.0) { 14.0 } else if self.chain.is_captain(d.group) { 4.0 } else if walking { 7.0 } else { 6.0 };
            // Pose changes are tracked per group and ripple through it: each soldier starts up to 0.4 s late.
            let entry = self.group_pose.entry(d.group.0).or_insert((pose, pose, -1000.0));
            if entry.0 != pose {
                *entry = (pose, entry.0, ts);
            }
            let (prev, since) = (entry.1, entry.2 + jitter * 0.4);
            // A group set to aim keeps aiming while it moves: walking legs under an aiming upper body.
            let aiming = matches!(self.stance.get(&d.group.0), Some(Pose::StandAim | Pose::CoverAim));
            let upper = if walking && !running && aiming { Pose::StandAim as u32 as f32 + 1.0 } else { 0.0 };
            inst.extend_from_slice(&[x, y, z, body, heading, jitter, steps, pose as u32 as f32, prev as u32 as f32, since, upper, 0.0]);
        }
        self.derived = out;
        if self.ground_view <= UNITS_MAX {
            self.agent_instances(ts, &mut inst);
        }
        self.gpu.upload_instances("soldiers", bytes(&inst));

        // Vehicles of groups near the camera, drawn as instances of the shipped tank and truck.
        let mut vehicles: Vec<DerivedVehicle> = Vec::new();
        for i in 0..self.world.groups.len() as u32 {
            let c = self.group_centre(i, tick);
            if self.ground_view <= UNITS_MAX && (c.x - look.x).abs() <= self.ground_view * 1.6 + 120.0 && (c.y - look.y).abs() <= self.ground_view * 1.6 + 120.0 {
                derive_vehicles(&self.world, GroupId(i), tick, &mut vehicles);
            }
        }
        let (mut tanks, mut trucks) = (Vec::new(), Vec::new());
        for v in &vehicles {
            let (ax, ay) = (v.pos.x as f64 / 1000.0, v.pos.y as f64 / 1000.0);
            let z = self.height_at(ax as f32, ay as f32);
            let (x, y) = ((ax - self.origin.0) as f32, (ay - self.origin.1) as f32);
            let heading = (-(v.facing.0 as f32)).atan2(v.facing.1 as f32);
            let rec = [x, y, z, 0.0, heading, 0.0, 0.0, 0.0, 0.0, -1000.0, 0.0, 0.0];
            if v.kind == UnitType::Tank { tanks.extend_from_slice(&rec) } else { trucks.extend_from_slice(&rec) }
        }
        self.ship_info(3);
        self.ship_info(4);
        self.gpu.upload_instances("tanks", bytes(&tanks));
        self.gpu.upload_instances("trucks", bytes(&trucks));
        instanced.push(("ship3".into(), "tanks".into()));
        instanced.push(("ship4".into(), "trucks".into()));
        let look = self.ground_cam.map(|c| (c.x as f64, c.y as f64)).unwrap_or((cx, cy));
        if self.ground_view > NEAR_MAX {
            self.far_terrain(Vec3::new(look.0 as f32, look.1 as f32, 0.0), t, meshes, instanced);
        } else {
        let (gx, gy) = ((look.0 / CELLS as f64).floor() as i64, (look.1 / CELLS as f64).floor() as i64);
        // Enough chunks to cover a wide screen at any yaw; at most a few new ones per frame to avoid hitches.
        let rad = (self.ground_view * 1.3 / CELLS as f32).ceil() as i64 + 1;
        // Coarser terrain when zoomed out; all chunks share one step, so neighbours always match.
        // Reasonably sized triangles: 2 m close up (the grass carries the fine detail), 4 m further out.
        let step = if self.ground_view <= 130.0 { 2 } else { 4 };
        self.terrain_step = step;
        let mut budget = 6;
        for dy in -rad..=rad {
            for dx in -rad..=rad {
                let key = (gx + dx, gy + dy);
                let (ccx, ccy) = (((key.0 * CELLS as i64) as f64 + 32.0 - self.origin.0) as f32, ((key.1 * CELLS as i64) as f64 + 32.0 - self.origin.1) as f32);
                if !self.on_screen(ccx, ccy, self.base_h, 48.0) {
                    continue; // off screen: neither built nor drawn
                }
                let current = self.chunk_lod.get(&key).copied();
                if current != Some(step) {
                    if budget == 0 && current.is_some() {
                        instanced.push((format!("terrain{},{}", key.0, key.1), format!("ot{},{}", key.0, key.1)));
                        continue; // keep the old mesh until there is time to rebuild it
                    }
                    if budget == 0 {
                        continue;
                    }
                    budget -= 1;
                    self.chunk_mesh(key.0, key.1, step);
                }
                instanced.push((format!("terrain{},{}", key.0, key.1), format!("ot{},{}", key.0, key.1)));
            }
        }
        // Grass only close up: within two chunks of the camera and below a 90 m view.
        if self.ground_view <= 90.0 {
            for dy in -2..=2 {
                for dx in -2..=2 {
                    let key = format!("grass{},{}", gx + dx, gy + dy);
                    if self.gpu.has(&key) {
                        decor.push(("tuft".into(), key));
                    }
                }
            }
        }
        let keep = rad + 2;
        let far: Vec<_> = if self.map_pass { Vec::new() } else { self.chunks.keys().copied().filter(|&(x, y)| (x - gx).abs() > keep || (y - gy).abs() > keep).collect() };
        for k in far {
            self.chunks.remove(&k);
            self.chunk_lod.remove(&k);
            self.gpu.drop_buffer(&format!("grass{},{}", k.0, k.1));
            self.gpu.drop_buffer(&format!("terrain{},{}", k.0, k.1));
            self.gpu.drop_buffer(&format!("ot{},{}", k.0, k.1));
        }
        }
        if self.ground_view > UNITS_MAX {
            // Groups as markers, each a few pixels big, sitting on the planet's surface.
            let pix = self.ground_view / self.size.1.max(1) as f32 * self.pixel_scale as f32;
            let mut m = MeshBuilder::default();
            for i in 0..self.world.groups.len() as u32 {
                let c = self.group_centre(i, tick);
                let h = self.surface_m(c.x, c.y);
                let (rx, ry) = ((c.x as f64 - self.origin.0) as f32, (c.y as f64 - self.origin.1) as f32);
                m.diamond([rx, ry, h + pix * 3.0], pix * 2.0, if self.selection.contains(&i) { 14.0 } else { 9.0 });
            }
            self.gpu.upload("markers", m.bytes());
            meshes.push("markers".into());
        }
        // Full figure close up, the lighter one when zoomed out.
        if !self.base_built {
            self.build_base();
        }
        if self.map_pass {
            // On the map he is always solid (the main view dissolves him behind the tablet), with a red
            // "you are here" ring and pointer that stay a few pixels big at any zoom.
            let mut me = self.cmd_rec;
            me[11] = 0.0;
            self.gpu.upload_instances("commander_map_at", bytes(&me));
            instanced.push(("commander".into(), "commander_map_at".into()));
            let pix = self.ground_view / self.size.1.max(1) as f32 * self.pixel_scale as f32;
            self.gpu.upload("you_marker", you_marker([me[0], me[1], me[2]], pix).bytes());
            meshes.push("you_marker".into());
        } else {
            self.update_commander(t, ts, instanced);
        }
        instanced.push(("base".into(), "base_at".into()));
        let (bx, by) = ((-self.origin.0) as f32, (-self.origin.1) as f32);
        self.gpu.upload_instances("base_at", bytes(&[bx, by, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1000.0, 0.0, 0.0]));
        // Three levels of detail. Past 250 m a soldier is a few pixels tall: the 24-triangle figure, and no
        // shadow pass for it (its shadow would be smaller than a pixel).
        if self.ground_view <= 70.0 {
            instanced.push(("figure".into(), "soldiers".into()));
        } else if self.ground_view <= 250.0 {
            instanced.push(("figure_lo".into(), "soldiers".into()));
        } else {
            decor.push(("figure_far".into(), "soldiers".into()));
        }
        if let Some(d) = self.order_dest {
            let arrived = self.selected.map_or(true, |g| !self.world.groups.process(GroupId(g)).is_running_at(tick));
            if arrived {
                self.order_dest = None;
            } else {
                let mut m = MeshBuilder::default();
                let z = self.height_at(d.x, d.y);
                let (dx, dy) = ((d.x as f64 - self.origin.0) as f32, (d.y as f64 - self.origin.1) as f32);
                m.diamond([dx, dy, z + 2.0], 1.2, 9.0);
                m.cuboid([dx - 0.1, dy - 0.1, z], [dx + 0.1, dy + 0.1, z + 2.0], 9.0);
                self.gpu.upload("order", m.bytes());
                meshes.push("order".into());
            }
        }
        self.focus = Vec3::new(cx as f32, cy as f32, 0.0);
        if !self.map_pass {
            self.ground_cam = Some(Vec3::new(self.cmd.0 as f32, self.cmd.1 as f32, 0.0));
        }
        let mut target = self.ground_cam.unwrap_or(self.focus);
        // Zoomed far out the camera sinks to the planet's centre, so the globe ends up in the middle of the screen.
        let sink = smooth_log(self.ground_view, 400_000.0, 2_400_000.0);
        let ground_z = if self.ground_view <= NEAR_MAX { self.height_at(target.x, target.y) } else { self.surface_m(target.x, target.y) };
        target.z = ground_z + (-PLANET_R - ground_z) * sink;
        // Ease towards the aimed target: following and panning glide instead of stepping.
        let dt = (t - self.cam_ms).clamp(0.0, 100.0);
        self.cam_ms = t;
        let k = 1.0 - (-dt / 140.0).exp() as f32;
        let smooth = match self.cam_smooth {
            Some(c) if (c - target).length() < self.ground_view * 3.0 => c + (target - c) * k,
            _ => target, // a jump (new army, far pan): cut straight there
        };
        self.cam_smooth = Some(smooth);
        let target = smooth;
        let target = Vec3::new((target.x as f64 - self.origin.0) as f32, (target.y as f64 - self.origin.1) as f32, target.z);
        let mode = if self.ground_cam.is_some() { "free camera (F follows)".to_string() } else { format!("following group {lead}") };
        let sel = match self.selected { Some(g) => format!(", group {g} selected"), None => String::new() };
        let total: u64 = (0..self.world.groups.len()).map(|i| self.world.groups.people(scalia_core::groups::GroupId(i as u32)) as u64).sum();
        let text = format!("Ground: {n} of {total} soldiers drawn, {mode}{sel}, {:.0} m view", self.ground_view);
        let range = 600f32.max(self.ground_view * 2.0 + PLANET_R * 2.0 * self.ground_bend().t);
        (target, self.yaw, self.ground_view, range, text)
    }

    /// The testing ground: rasterise the base's props into its nav grid and build their mesh (once).
    fn build_base(&mut self) {
        self.base_h = self.planet.height_mm(100_000, (100_000.0 + HALF_DEPTH as f64 * 1000.0) as i64) as f32 / 1000.0;
        self.base_grid.rebuild(&self.base_props);
        let m = base_mesh(&self.base_props, self.base_h);
        self.gpu.upload("base", m.bytes());
        self.base_built = true;
    }

    /// Advances the agent squads in fixed 50 ms steps (deterministic, whatever the frame rate).
    fn step_agents(&mut self, t: f64) {
        self.agent_now = t;
        if self.agent_clock < 0.0 || t - self.agent_clock > 1000.0 {
            self.agent_clock = t;
        }
        while t - self.agent_clock >= 50.0 {
            self.agent_clock += 50.0;
            let keys: Vec<u32> = self.agents.keys().copied().collect();
            let mut reports: Vec<(u32, String, u8)> = Vec::new();
            for g in keys {
                let a = self.agents.get_mut(&g).expect("present");
                self.agent_prev.insert(g, a.positions().to_vec());
                a.step(&self.base_grid, 50);
                if a.all_arrived() {
                    if let Some(p) = self.arrive_stance.remove(&g) {
                        self.stance.insert(g, p);
                    }
                    if let Some((spot, kind)) = self.arrive_report.remove(&g) {
                        reports.push((g, spot, kind));
                    }
                }
                // Keep the group's own record where its soldiers are: any later order starts from here, so the
                // squad never jumps back to where it stood before walking as agents.
                let (cx, cy) = a.centre();
                self.world.groups.set_process(GroupId(g), scalia_core::process::Process::idle());
                self.world.groups.set_pos(GroupId(g), Pos { region: RegionId(1), x: cx, y: cy, z: 0 });
            }
            for (g, spot, kind) in reports {
                let me = self.call_sign(g);
                let pick = (g as usize + self.radio_out.len() + (t as usize / 50)) % 3;
                let text = match (kind, pick) {
                    (1, 0) => format!("Actual, {me}. Dug in at grid {spot}, holding. Over."),
                    (1, 1) => format!("Actual, {me}. Grid {spot} secured, we hold here. Out."),
                    (1, _) => format!("Actual, {me}. In cover at {spot}, watching our front. Over."),
                    (2, 0) => format!("Actual, {me}. Assault element on grid {spot}. Awaiting orders. Over."),
                    (2, 1) => format!("Actual, {me}. Reached {spot}, area clear. Over."),
                    (2, _) => format!("Actual, {me}. At objective {spot}, ready. Out."),
                    (3, 0) => format!("Actual, {me}. Support set at grid {spot}. Over."),
                    (3, 1) => format!("Actual, {me}. In position at {spot}, eyes on. Out."),
                    (3, _) => format!("Actual, {me}. Overwatch established, grid {spot}. Over."),
                    (_, 0) => format!("Actual, {me}. Arrived at grid {spot}. Over."),
                    (_, 1) => format!("Actual, {me}. We are at {spot}, awaiting orders. Over."),
                    (_, _) => format!("Actual, {me}. At position, grid {spot}. Out."),
                };
                self.radio(g, text, false);
            }
        }
        self.run_captains(self.frame_ms);
    }

    /// Instances for every agent soldier, posed like the derived ones.
    fn agent_instances(&mut self, ts: f32, inst: &mut Vec<f32>) {
        let mut keys: Vec<u32> = self.agents.keys().copied().collect();
        keys.sort();
        for g in keys {
            let (pos, arrived) = {
                let a = &self.agents[&g];
                (a.positions().to_vec(), a.arrived().to_vec())
            };
            let prev = self.agent_prev.get(&g).cloned().unwrap_or_else(|| pos.clone());
            // Drawn between the last two 50 ms steps, so movement is smooth at any frame rate.
            let alpha = ((self.agent_now - self.agent_clock) / 50.0).clamp(0.0, 1.0);
            let stance = self.stance.get(&g).copied().unwrap_or(Pose::Stand);
            let sprint = self.agents[&g].speed_mm_s() >= RUN_SPEED_MM_S as i64;
            for (i, (&(x, y), &(px, py))) in pos.iter().zip(&prev).enumerate() {
                let jitter = (scalia_core::hash::mix64(((g as u64) << 20) ^ i as u64 ^ 0x5ca1) % 1000) as f32 / 1000.0;
                // Walking lasts a moment past the last step, so jostling never flickers between walk and stand;
                // the heading eases round instead of snapping to each step's direction.
                let key = g * 1000 + i as u32;
                let stepped = x != px || y != py;
                let entry = self.agent_anim.entry(key).or_insert(((jitter - 0.5) * 0.4, -1.0));
                if stepped && !arrived[i] {
                    entry.1 = self.agent_now + 300.0;
                    let want = ((px - x) as f32).atan2((y - py) as f32);
                    let mut d = want - entry.0;
                    while d > std::f32::consts::PI { d -= std::f32::consts::TAU; }
                    while d < -std::f32::consts::PI { d += std::f32::consts::TAU; }
                    entry.0 += d * 0.25;
                }
                let moving = self.agent_now < entry.1;
                let heading = entry.0;
                let (pose, steps) = if moving && sprint { (Pose::Sprint, 1.45 + jitter * 0.1) } else if moving { (Pose::Walk, 0.9 + jitter * 0.1) } else { (stance, 0.0) };
                let (ix, iy) = (px as f64 + (x - px) as f64 * alpha, py as f64 + (y - py) as f64 * alpha);
                let (ax, ay) = (ix / 1000.0, iy / 1000.0);
                let z = self.height_at(ax as f32, ay as f32);
                let (rx, ry) = ((ax - self.origin.0) as f32, (ay - self.origin.1) as f32);
                if !self.on_screen(rx, ry, z, 3.0) {
                    continue;
                }
                let body = if self.selection.contains(&g) { 14.0 } else if self.chain.is_captain(GroupId(g)) { 4.0 } else if moving { 7.0 } else { 6.0 };
                let entry = self.group_pose.entry(1_000_000 + key).or_insert((pose, pose, -1000.0));
                if entry.0 != pose {
                    *entry = (pose, entry.0, ts);
                }
                let (prev_pose, since) = (entry.1, entry.2);
                let aiming = matches!(stance, Pose::StandAim | Pose::CoverAim);
                let upper = if moving && pose == Pose::Walk && aiming { Pose::StandAim as u32 as f32 + 1.0 } else { 0.0 };
                inst.extend_from_slice(&[rx, ry, z, body, heading, jitter, steps, pose as u32 as f32, prev_pose as u32 as f32, since, upper, 0.0]);
            }
        }
    }

    fn swap_cam(&mut self) {
        std::mem::swap(&mut self.scene, &mut self.map_cam.scene);
        std::mem::swap(&mut self.zoom_step, &mut self.map_cam.zoom_step);
        std::mem::swap(&mut self.zoom_m, &mut self.map_cam.zoom_m);
        std::mem::swap(&mut self.ground_view, &mut self.map_cam.ground_view);
        std::mem::swap(&mut self.sys_scale, &mut self.map_cam.sys_scale);
        std::mem::swap(&mut self.sys_cam, &mut self.map_cam.sys_cam);
        std::mem::swap(&mut self.sys_follow, &mut self.map_cam.sys_follow);
        std::mem::swap(&mut self.ground_cam, &mut self.map_cam.ground_cam);
        std::mem::swap(&mut self.cam_smooth, &mut self.map_cam.cam_smooth);
        std::mem::swap(&mut self.cam_ms, &mut self.map_cam.cam_ms);
        std::mem::swap(&mut self.yaw, &mut self.map_cam.yaw);
        std::mem::swap(&mut self.yaw_smooth, &mut self.map_cam.yaw_smooth);
        std::mem::swap(&mut self.last_cam, &mut self.map_cam.last_cam);
        std::mem::swap(&mut self.last_view, &mut self.map_cam.last_view);
        std::mem::swap(&mut self.size, &mut self.map_cam.size);
        std::mem::swap(&mut self.last_frame_ms, &mut self.map_cam.last_frame_ms);
        std::mem::swap(&mut self.last_internal, &mut self.map_cam.last_internal);
    }

    /// Moves the commander by his input (relative to the camera), keeps him out of walls, and draws him: black
    /// clothing, a red armband, walking, sprinting or on the radio.
    fn update_commander(&mut self, t: f64, ts: f32, instanced: &mut Vec<(String, String)>) {
        if !self.gpu.has("commander") {
            self.gpu.upload("commander", commander_figure().bytes());
        }
        let dt = ((t - self.cmd_ms).clamp(0.0, 100.0) / 1000.0) as f32;
        self.cmd_ms = t;
        let (ix, iy, sprint) = self.cmd_input;
        let (s, c) = self.yaw_smooth.sin_cos();
        let (right, fwd) = ((c, s), (-s, c));
        let (mut vx, mut vy) = (right.0 * ix + fwd.0 * iy, right.1 * ix + fwd.1 * iy);
        let len = (vx * vx + vy * vy).sqrt();
        let holding = self.tablet || self.cine.is_some();
        self.cmd_moving = len > 0.01 && t >= self.radio_until && !holding;
        if self.cmd_moving {
            let speed = if sprint { CMD_SPRINT } else { CMD_WALK };
            vx = vx / len * speed * dt;
            vy = vy / len * speed * dt;
            let (nx, ny) = (self.cmd.0 + vx as f64, self.cmd.1 + vy as f64);
            let free = |x: f64, y: f64| !in_base(x as f32, y as f32) || self.base_grid.walkable_at((x * 1000.0) as i64, (y * 1000.0) as i64, 300);
            if free(nx, ny) {
                self.cmd = (nx, ny);
            } else if free(nx, self.cmd.1) {
                self.cmd.0 = nx;
            } else if free(self.cmd.0, ny) {
                self.cmd.1 = ny;
            }
            // Turn smoothly towards where he walks.
            let want = (-vx).atan2(vy);
            let mut d = want - self.cmd_heading;
            while d > std::f32::consts::PI { d -= std::f32::consts::TAU; }
            while d < -std::f32::consts::PI { d += std::f32::consts::TAU; }
            self.cmd_heading += d.clamp(-8.0 * dt, 8.0 * dt);
        }
        let pose = if holding { Pose::Tablet } else if t < self.radio_until { Pose::Radio } else if self.cmd_moving && sprint { Pose::Sprint } else if self.cmd_moving { Pose::Walk } else { Pose::Stand };
        if self.cmd_pose.0 != pose {
            self.cmd_pose = (pose, self.cmd_pose.0, ts);
        }
        let steps = match pose { Pose::Sprint => 1.5, Pose::Walk => 0.95, _ => 0.0 };
        // His gait phase, exactly as the shader computes it (instance phase offset 0.37), for the footsteps.
        self.cmd_phase = if steps > 0.0 { (ts * steps + 0.37).fract() } else { -1.0 };
        let z = self.height_at(self.cmd.0 as f32, self.cmd.1 as f32);
        let (rx, ry) = ((self.cmd.0 - self.origin.0) as f32, (self.cmd.1 - self.origin.1) as f32);
        let rec = [rx, ry, z, 9.0, self.cmd_heading, 0.37, steps, pose as u32 as f32, self.cmd_pose.1 as u32 as f32, self.cmd_pose.2, 0.0, 0.0];
        // The diving camera passes through his head on its way to the tablet: he dissolves as it gets close.
        self.cmd_rec = rec;
        let mut me = rec;
        me[11] = self.cine_fade(self.frame_ms);
        self.gpu.upload_instances("commander_at", bytes(&me));
        instanced.push(("commander".into(), "commander_at".into()));
        // The tablet appears in his hands once they have come up to hold it.
        if pose == Pose::Tablet && ts - self.cmd_pose.2 >= scalia_render::pose::BLEND_S {
            if !self.gpu.has("pda") {
                self.gpu.upload("pda", pda_held().bytes());
            }
            self.gpu.upload_instances("pda_at", bytes(&rec));
            instanced.push(("pda".into(), "pda_at".into()));
        }
    }

    /// How far the commander has dissolved for the tablet close-up (0 solid, 1 gone).
    fn cine_fade(&self, now_ms: f64) -> f32 {
        let s = |a: f64, b: f64, x: f64| { let k = ((x - a) / (b - a)).clamp(0.0, 1.0) as f32; k * k * (3.0 - 2.0 * k) };
        match self.cine {
            Some((true, start, _)) => s(CINE_OPEN_MS - 280.0, CINE_OPEN_MS - 80.0, now_ms - start),
            Some((false, start, _)) => 1.0 - s(0.0, CINE_CLOSE_MS * 0.45, now_ms - start),
            None => 0.0,
        }
    }

    /// The tablet cinematic's camera (target, yaw, view height, elevation) at this moment, or None when it is not
    /// running. `live` is the camera the game would use: closing returns to it.
    fn cine_cam(&mut self, now_ms: f64, live: (Vec3, f32, f32)) -> Option<(Vec3, f32, f32, f32)> {
        let (opening, start, from) = self.cine?;
        let el = scalia_render::camera::ELEV;
        let base = Vec3::new((self.cmd.0 - self.origin.0) as f32, (self.cmd.1 - self.origin.1) as f32, self.height_at(self.cmd.0 as f32, self.cmd.1 as f32));
        let h = self.cmd_heading;
        // Over his shoulder: the camera looks the way he faces, at the tablet in his hands.
        let at = scalia_render::pose::TABLET_AT;
        let hands = base + Vec3::new(at.x * h.cos() - at.y * h.sin(), at.x * h.sin() + at.y * h.cos(), at.z);
        // Behind him, looking the way he faces: first the camera swings round at the player's zoom, then dives.
        let behind = (base + Vec3::Z, h, from.2, from.3);
        // Face-on to the tablet (the camera's elevation is the tablet's tilt), sized like the close-up on screen.
        let vh = TABLET_IMG_H / (self.tablet_fit * self.tablet_frame.0).max(0.05);
        let aspect = self.size.0 as f32 / self.size.1.max(1) as f32;
        let (r, _, _) = scalia_render::camera::basis(h);
        let close = (hands - r * (self.tablet_frame.1 * vh * aspect), h, vh, scalia_render::pose::TABLET_TILT);
        // Smootherstep: starts slow, runs at full pace mid-way, lands slow; the duration is unchanged.
        let ease = |x: f64| { let x = x.clamp(0.0, 1.0) as f32; x * x * x * (x * (x * 6.0 - 15.0) + 10.0) };
        let mix = |a: (Vec3, f32, f32, f32), b: (Vec3, f32, f32, f32), w: f32| {
            let mut dy = b.1 - a.1;
            while dy > std::f32::consts::PI { dy -= std::f32::consts::TAU; }
            while dy < -std::f32::consts::PI { dy += std::f32::consts::TAU; }
            // Zoom eases in log space, so it keeps an even pace from far to near.
            let vh = (a.2.ln() + (b.2.ln() - a.2.ln()) * w).exp();
            (a.0 + (b.0 - a.0) * w, a.1 + dy * w, vh, a.3 + (b.3 - a.3) * w)
        };
        let e = now_ms - start;
        if opening {
            Some(if e < CINE_TURN_MS {
                mix(from, behind, ease(e / CINE_TURN_MS))
            } else if e < CINE_TURN_MS + CINE_HOLD_MS {
                behind
            } else if e < CINE_OPEN_MS {
                mix(behind, close, ease((e - CINE_TURN_MS - CINE_HOLD_MS) / (CINE_OPEN_MS - CINE_TURN_MS - CINE_HOLD_MS)))
            } else {
                close
            })
        } else if e < CINE_CLOSE_MS {
            Some(mix(from, (live.0, live.1, live.2, el), ease(e / CINE_CLOSE_MS)))
        } else {
            self.cine = None;
            None
        }
    }

    /// Whether a point (origin-relative metres) falls on screen, by last frame's camera, with `margin` metres
    /// to spare. The ground under an isometric screen is a tilted rectangle; anything outside it is skipped.
    fn on_screen(&self, x: f32, y: f32, z: f32, margin: f32) -> bool {
        let (target, yaw, view_h) = self.last_cam;
        let (r, u, _) = scalia_render::camera::basis(yaw);
        let d = Vec3::new(x, y, z) - target;
        let aspect = self.size.0 as f32 / self.size.1.max(1) as f32;
        (d.dot(r)).abs() <= view_h * 0.5 * aspect + margin && (d.dot(u)).abs() <= view_h * 0.5 + margin
    }

    /// Instance that places a chunk (built chunk-local) relative to the floating origin.
    fn place_chunk(&mut self, cx: i64, cy: i64) {
        let (dx, dy) = ((cx * CELLS as i64) as f64 - self.origin.0, (cy * CELLS as i64) as f64 - self.origin.1);
        self.gpu.upload_instances(&format!("ot{cx},{cy}"), bytes(&[dx as f32, dy as f32, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1000.0, 0.0, 0.0]));
    }

    /// Instance that places a far tile relative to the floating origin, with a fade (0 solid).
    fn place_far(&mut self, key: (u32, i64, i64), fade: f32, prefix: &str) {
        let size = 64.0 * (1u64 << key.0) as f64;
        let (dx, dy) = (key.1 as f64 * size - self.origin.0, key.2 as f64 * size - self.origin.1);
        let rec = [dx as f32, dy as f32, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1000.0, 0.0, fade];
        self.gpu.upload_instances(&format!("{prefix}{},{},{}", key.0, key.1, key.2), bytes(&rec));
    }

    /// The floating origin moved: re-place every chunk, far tile and grass patch (meshes stay as built).
    fn reorigin(&mut self) {
        let chunks: Vec<_> = self.chunk_lod.keys().copied().collect();
        for (cx, cy) in chunks {
            self.place_chunk(cx, cy);
            if self.gpu.has(&format!("grass{cx},{cy}")) {
                let hm = self.chunk(cx, cy).clone();
                self.gpu.upload_instances(&format!("grass{cx},{cy}"), bytes(&grass_for(&hm, cx, cy, self.origin)));
            }
        }
        let tiles: Vec<_> = self.far_tiles.iter().copied().collect();
        for key in tiles {
            self.place_far(key, 0.0, "of");
        }
    }

    /// How far the map is bent into a globe at the current zoom: flat below 30 km, a full globe from 1,500 km.
    fn ground_bend(&self) -> Bend {
        let tg = self.ground_cam.unwrap_or(self.focus);
        let (tx, ty) = ((tg.x as f64 - self.origin.0) as f32, (tg.y as f64 - self.origin.1) as f32);
        Bend { tx, ty, radius: PLANET_R, t: smooth_log(self.ground_view, 30_000.0, 1_500_000.0), lat_y: tg.y }
    }

    /// Planet surface height in metres at a ground-scene point (sea level where the ground is under water).
    fn surface_m(&self, x: f32, y: f32) -> f32 {
        let (xm, ym) = ((x as f64 * 1000.0) as i64, ((y as f64 + HALF_DEPTH as f64) * 1000.0) as i64);
        (self.planet.height_mm(xm, ym.clamp(0, self.planet.depth_mm() - 1)) as f32 / 1000.0).max(0.0)
    }

    /// Coarse terrain for zoomed-out views: square tiles of 64 x 64 cells, each cell 2^level metres, enough of
    /// them to cover the screen (or the whole planet once it is a globe). A level is drawn only when all its
    /// tiles are ready; until then the previous level stays, so zooming never shows holes.
    fn far_terrain(&mut self, tg: Vec3, now: f64, _meshes: &mut Vec<String>, instanced: &mut Vec<(String, String)>) {
        // Cells about five screen pixels wide: any finer and the globe shimmers as the camera moves. And at least
        // six internal pixels (the small tablet map draws at few of them): smaller facets get speckled normals.
        let pix = self.ground_view / self.size.1.max(1) as f32 * self.pixel_scale as f32;
        let by_view = (self.ground_view / 64.0).log2().ceil() as i32;
        let by_pixels = (pix * 6.0).log2().ceil() as i32;
        let want = by_view.max(by_pixels).clamp(1, 20) as u32;
        let view = self.ground_view;
        let needed = |level: u32| -> Vec<(u32, i64, i64)> {
            let ts = 64.0 * (1u64 << level) as f32;
            let half_lon = PLANET_R * std::f32::consts::PI;
            let cov = (1.6 * view).min(half_lon);
            let (y0, y1) = (((tg.y - cov).max(-HALF_DEPTH) / ts).floor() as i64, ((tg.y + cov).min(HALF_DEPTH - 1.0) / ts).floor() as i64);
            // Towards a pole the same ground spans more longitude (1 / cos latitude), and at the pole all of it
            // meets: each row of tiles covers what its poleward edge needs, up to the whole way round.
            (y0..=y1)
                .flat_map(|y| {
                    let edge = ((y as f32 * ts).abs()).max(((y + 1) as f32 * ts).abs()).min(HALF_DEPTH);
                    let lat = edge / PLANET_R;
                    let span = (cov / lat.cos().max(1e-3)).min(half_lon);
                    let (x0, x1) = (((tg.x - span) / ts).floor() as i64, ((tg.x + span) / ts).floor() as i64);
                    (x0..=x1).map(move |x| (level, x, y))
                })
                .collect()
        };
        let tiles = needed(want);
        let mut budget = 6;
        let mut missing = false;
        for &key in &tiles {
            if !self.far_tiles.contains(&key) {
                if budget == 0 {
                    missing = true;
                    continue;
                }
                budget -= 1;
                let m = far_tile_mesh(&self.planet, key.0, key.1, key.2);
                self.gpu.upload(&format!("far{},{},{}", key.0, key.1, key.2), m.bytes());
                self.far_tiles.insert(key);
                self.place_far(key, 0.0, "of");
            }
        }
        // A new level takes over only when complete, dissolving in over the old one.
        if !missing && self.far_level != Some(want) {
            if let Some(old) = self.far_level {
                self.far_fade = Some((old, now));
            }
            self.far_level = Some(want);
        }
        let shown = self.far_level.unwrap_or(want);
        let fade = self.far_fade.and_then(|(old, start)| {
            let p = ((now - start) / FADE_MS) as f32;
            (p < 1.0 && old != shown).then_some((old, p))
        });
        if fade.is_none() {
            self.far_fade = None;
        }
        let p = fade.map_or(1.0, |f| f.1);
        // The new level is solid at once; the old one lies over it at half strength and thins out to nothing,
        // a gentle blend rather than a flicker between the two.
        let new_tiles = if shown == want { tiles.clone() } else { needed(shown) };
        for key in &new_tiles {
            if self.far_tiles.contains(key) {
                instanced.push((format!("far{},{},{}", key.0, key.1, key.2), format!("of{},{},{}", key.0, key.1, key.2)));
            }
        }
        let mut keep: std::collections::HashSet<_> = new_tiles.iter().copied().chain(tiles).collect();
        if let Some((old, _)) = fade {
            for key in needed(old) {
                if self.far_tiles.contains(&key) {
                    self.place_far(key, 0.5 + 0.5 * p, "off");
                    instanced.push((format!("far{},{},{}", key.0, key.1, key.2), format!("off{},{},{}", key.0, key.1, key.2)));
                }
                keep.insert(key);
            }
        }
        // Forget tiles of other levels, and ones far off screen.
        let stale: Vec<_> = self.far_tiles.iter().copied().filter(|k| !keep.contains(k)).collect();
        for k in stale {
            self.far_tiles.remove(&k);
            self.gpu.drop_buffer(&format!("far{},{},{}", k.0, k.1, k.2));
            self.gpu.drop_buffer(&format!("of{},{},{}", k.0, k.1, k.2));
            self.gpu.drop_buffer(&format!("off{},{},{}", k.0, k.1, k.2));
        }
    }

    /// Jumps straight to a ladder step (scene buttons) and returns the scene that shows it.
    fn jump(&mut self, step: usize) -> Scene {
        self.zoom_step = step;
        self.zoom_m = LADDER[step];
        if step > STEP_HANDOFF {
            let tgt = self.ground_cam.unwrap_or(self.focus);
            self.home_globe = oriented_globe(&self.planet, tgt.x, tgt.y, 96, 48);
            self.sys_follow = Some(HOME_BODY);
            self.sys_cam = None;
            self.sys_scale = self.zoom_m / 1e6;
            Scene::System
        } else {
            self.ground_view = self.zoom_m;
            Scene::Ground
        }
    }

    /// Glides the camera towards the aimed ladder step (exponentially, on a log scale) and hands the view between
    /// the ground and the star system at GROUND_TOP, where both frame the planet identically.
    fn glide(&mut self, now_ms: f64) {
        let dt = (now_ms - self.last_frame_ms).clamp(0.0, 100.0);
        self.last_frame_ms = now_ms;
        if self.scene == Scene::Ships {
            return;
        }
        let goal = LADDER[self.zoom_step];
        let k = 1.0 - (-dt / GLIDE_MS).exp();
        let (a, b) = (self.zoom_m.ln(), goal.ln());
        let mut z = (a + (b - a) * k as f32).exp();
        if (z / goal - 1.0).abs() < 1e-3 {
            z = goal;
        }
        if self.scene == Scene::System && self.sys_follow != Some(HOME_BODY) {
            z = z.max(GROUND_TOP * 1.001);
        }
        self.zoom_m = z;
        if z <= GROUND_TOP {
            if self.scene == Scene::System {
                self.scene = Scene::Ground;
            }
            self.ground_view = z;
        } else {
            if self.scene == Scene::Ground {
                // Hand over to the star system with the same framing and the globe turned the same way.
                let tgt = self.ground_cam.unwrap_or(self.focus);
                self.home_globe = oriented_globe(&self.planet, tgt.x, tgt.y, 96, 48);
                self.scene = Scene::System;
                self.sys_cam = None;
                self.sys_follow = Some(HOME_BODY);
            }
            self.sys_scale = z / 1e6;
        }
    }

    fn system_scene(&mut self, t: f64, meshes: &mut Vec<String>) -> (Vec3, f32, f32, f32, String) {
        let now = WorldMs(t as u64);
        let mut m = MeshBuilder::default();
        let pix = self.sys_scale / 270.0;
        let names: Vec<&str> = self.system.bodies.iter().map(|b| b.name).collect();
        for (i, b) in self.system.bodies.iter().enumerate() {
            let p = self.system.body_pos(BodyId(i as u8), now).expect("known body");
            m.ring([0.0, 0.0], (b.orbit_radius_mm as f64 / KM) as f32, pix * 0.6, 128, 3.0);
            let c = Vec3::new((p.x as f64 / KM) as f32, (p.y as f64 / KM) as f32, 0.0);
            let r = pix * 5.0;
            let globe = if i == HOME_BODY as usize && !self.home_globe.is_empty() { &self.home_globe } else { &self.body_globes[i] };
            // True size when close, a visible marker size when far.
            let r = r.max(PLANET_R / 1e6 * if i == HOME_BODY as usize { 1.0 } else { 0.0 });
            for (tri, col) in globe {
                m.tri((tri[0] * r + c).to_array(), (tri[1] * r + c).to_array(), (tri[2] * r + c).to_array(), *col);
            }
        }
        let r = (pix * 9.0).max(2500.0);
        for (tri, col) in &self.sun_globe {
            m.tri((tri[0] * r).to_array(), (tri[1] * r).to_array(), (tri[2] * r).to_array(), *col);
        }
        let mut text;
        let (sx, sy) = match self.flight {
            Some((plan, dest)) if now.0 < plan.start.0 + plan.duration_ms => {
                let p = flight_pos(&plan, now);
                let el = now.0 - plan.start.0;
                text = format!("Flying {} to {}: {} s of {} s", names[self.docked as usize], names[dest as usize], el / 1000, plan.duration_ms / 1000);
                ((p.x as f64 / KM) as f32, (p.y as f64 / KM) as f32)
            }
            other => {
                if let Some((_, dest)) = other {
                    self.docked = dest;
                    self.flight = None;
                }
                let b = self.system.body_pos(BodyId(self.docked), now).expect("known body");
                text = format!("Docked at {}. Wheel zooms ({:.0} km across).", names[self.docked as usize], self.sys_scale * 1000.0);
                ((b.x as f64 / KM) as f32, ((b.y + PARK_OFFSET.1) as f64 / KM) as f32)
            }
        };
        if self.flight.is_some() && self.trail.last().map_or(true, |l| t - l.0 > 700.0) {
            self.trail.push((t, sx, sy));
        }
        if self.flight.is_none() && self.trail.last().is_some_and(|l| t - l.0 > 5000.0) {
            self.trail.clear();
        }
        for &(_, x, y) in &self.trail {
            m.diamond([x, y, 0.0], pix * 1.2, 8.0);
        }
        m.cuboid([sx - pix * 2.5, sy - pix * 2.5, 0.0], [sx + pix * 2.5, sy + pix * 2.5, pix * 5.0], 15.0);
        self.gpu.upload("system", m.bytes());
        meshes.push("system".into());
        if self.flight.is_some() {
            text.push_str(" (in flight)");
        }
        self.sys_now = now;
        let target = self.sys_cam.unwrap_or_else(|| self.sys_follow_pos());
        (target, self.yaw, self.sys_scale, self.sys_scale * 4.0, text)
    }
}

/// A planet as a unit-radius globe of coloured triangles: longitude from the map's x (it wraps), latitude from y,
/// relief exaggerated, flat seas, polar ice. The same map the ground scenes are cut from.
fn globe_triangles(seed: u64, cols: usize, rows: usize) -> Vec<([Vec3; 3], f32)> {
    let p = Planet::demo(seed);
    let mut h = vec![0i32; (cols + 1) * (rows + 1)];
    for j in 0..=rows {
        for i in 0..=cols {
            let x = p.width_mm() / cols as i64 * i as i64;
            let y = p.depth_mm() / rows as i64 * j as i64;
            h[j * (cols + 1) + i] = p.coarse_height_mm(x, y);
        }
    }
    let vert = |i: usize, j: usize| -> Vec3 {
        let lon = i as f32 / cols as f32 * std::f32::consts::TAU;
        let lat = (j as f32 / rows as f32 - 0.5) * std::f32::consts::PI;
        let z = h[j * (cols + 1) + i].max(0) as f32; // the sea is flat
        let r = 1.0 + z / 1_000_000.0 * 0.004; // relief exaggerated about 2.6x
        Vec3::new(r * lat.cos() * lon.cos(), r * lat.cos() * lon.sin(), r * lat.sin())
    };
    let mut out = Vec::with_capacity(cols * rows * 2);
    for j in 0..rows {
        for i in 0..cols {
            let mean = (h[j * (cols + 1) + i] + h[j * (cols + 1) + i + 1] + h[(j + 1) * (cols + 1) + i] + h[(j + 1) * (cols + 1) + i + 1]) / 4;
            let y = p.depth_mm() / rows as i64 * j as i64 + p.depth_mm() / rows as i64 / 2;
            let col = if p.is_ice(y) {
                15.0
            } else if mean < 0 {
                6.0
            } else if mean < 1_200_000 {
                11.0
            } else if mean < 2_500_000 {
                12.0
            } else if mean < 4_000_000 {
                13.0
            } else {
                2.0
            };
            let (a, b, c, d) = (vert(i, j), vert(i + 1, j), vert(i + 1, j + 1), vert(i, j + 1));
            out.push(([a, b, c], col));
            out.push(([a, c, d], col));
        }
    }
    out
}

/// A plain unit sphere of one colour (the sun).
fn sphere_triangles(cols: usize, rows: usize, col: f32) -> Vec<([Vec3; 3], f32)> {
    let vert = |i: usize, j: usize| {
        let lon = i as f32 / cols as f32 * std::f32::consts::TAU;
        let lat = (j as f32 / rows as f32 - 0.5) * std::f32::consts::PI;
        Vec3::new(lat.cos() * lon.cos(), lat.cos() * lon.sin(), lat.sin())
    };
    let mut out = Vec::new();
    for j in 0..rows {
        for i in 0..cols {
            let (a, b, c, d) = (vert(i, j), vert(i + 1, j), vert(i + 1, j + 1), vert(i, j + 1));
            out.push(([a, b, c], col));
            out.push(([a, c, d], col));
        }
    }
    out
}

/// Half the planet map's north-south extent in metres: the poles are at y = +-HALF_DEPTH in the ground scene.
const HALF_DEPTH: f32 = (EQUATOR_CHUNK * CELLS as i64) as f32;

/// Smoothstep on a log scale: 0 at or below `a`, 1 at or above `b`.
fn smooth_log(v: f32, a: f32, b: f32) -> f32 {
    let x = ((v.ln() - a.ln()) / (b.ln() - a.ln())).clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// Colour of a patch of ground from its mean height (metres) when seen from far away.
fn far_colour(planet: &Planet, mean_m: f32, y_planet_mm: i64, cell_m: f32) -> f32 {
    if planet.is_ice(y_planet_mm) {
        15.0
    } else if mean_m < 0.0 {
        6.0
    } else {
        let _ = cell_m;
        zone_colour(mean_m)
    }
}

/// Broad height zones: the ground's colour wherever contour bands would be finer than a pixel.
fn zone_colour(mean_m: f32) -> f32 {
    if mean_m < 150.0 {
        // Broad height zones further out: bands every few metres turned into stripes on every slope.
        10.0
    } else if mean_m < 450.0 {
        11.0
    } else if mean_m < 1200.0 {
        11.0
    } else if mean_m < 2500.0 {
        12.0
    } else if mean_m < 4000.0 {
        13.0
    } else {
        2.0
    }
}

/// One far terrain tile: 64 x 64 cells of 2^level metres, seas flat at sea level.
fn far_tile_mesh(planet: &Planet, level: u32, ix: i64, iy: i64) -> MeshBuilder {
    const N: usize = 64;
    let cell = (1u64 << level) as f32;
    let (x0, y0) = (ix as f64 * N as f64 * cell as f64, iy as f64 * N as f64 * cell as f64);
    let depth = planet.depth_mm();
    let ymm = |y: f64| (((y + HALF_DEPTH as f64) * 1000.0) as i64).clamp(0, depth - 1);
    let mut h = vec![0f32; (N + 1) * (N + 1)];
    for j in 0..=N {
        for i in 0..=N {
            let (x, y) = (x0 + i as f64 * cell as f64, y0 + j as f64 * cell as f64);
            let (xm, ym) = ((x * 1000.0) as i64, ymm(y));
            let mm = if cell < 256.0 { planet.height_mm(xm, ym) } else { planet.coarse_height_mm(xm, ym) };
            h[j * (N + 1) + i] = mm as f32 / 1000.0;
        }
    }
    let mut m = MeshBuilder::default();
    // Tile-local positions: an instance offset places the tile relative to the floating origin.
    let p = |i: usize, j: usize| {
        let y = (y0 + j as f64 * cell as f64).clamp(-HALF_DEPTH as f64, HALF_DEPTH as f64) - y0;
        [(i as f64 * cell as f64) as f32, y as f32, h[j * (N + 1) + i].max(0.0)]
    };
    for j in 0..N {
        for i in 0..N {
            let mean = (h[j * (N + 1) + i] + h[j * (N + 1) + i + 1] + h[(j + 1) * (N + 1) + i] + h[(j + 1) * (N + 1) + i + 1]) / 4.0;
            let col = far_colour(planet, mean, ymm(y0 + (j as f64 + 0.5) * cell as f64), cell);
            let (a, b, c, d) = (p(i, j), p(i + 1, j), p(i + 1, j + 1), p(i, j + 1));
            m.tri(a, b, c, col);
            m.tri(a, c, d, col);
        }
    }
    m
}

/// The home planet as a unit globe around its centre, turned exactly as the ground view's fully bent map is when
/// the camera target is at (tx, ty): the target faces up (towards the camera), north along +y.
fn oriented_globe(planet: &Planet, tx: f32, ty: f32, cols: usize, rows: usize) -> Vec<([Vec3; 3], f32)> {
    let w = std::f32::consts::TAU * PLANET_R;
    let depth = planet.depth_mm();
    let vert = |i: usize, j: usize| -> (Vec3, f32) {
        let x = tx + (i as f32 / cols as f32 - 0.5) * w;
        let y = -HALF_DEPTH + j as f32 / rows as f32 * 2.0 * HALF_DEPTH;
        let ym = (((y as f64 + HALF_DEPTH as f64) * 1000.0) as i64).clamp(0, depth - 1);
        let hm = planet.coarse_height_mm((x as f64 * 1000.0) as i64, ym) as f32 / 1000.0;
        let p = globe_point(x - tx, y - ty, hm.max(0.0), ty / PLANET_R, PLANET_R) + Vec3::new(0.0, 0.0, PLANET_R);
        (p / PLANET_R, hm)
    };
    let mut out = Vec::with_capacity(cols * rows * 2);
    for j in 0..rows {
        for i in 0..cols {
            let (a, ha) = vert(i, j);
            let (b, hb) = vert(i + 1, j);
            let (c, hc) = vert(i + 1, j + 1);
            let (d, hd) = vert(i, j + 1);
            let y = -HALF_DEPTH + (j as f32 + 0.5) / rows as f32 * 2.0 * HALF_DEPTH;
            let ym = (((y as f64 + HALF_DEPTH as f64) * 1000.0) as i64).clamp(0, depth - 1);
            let col = far_colour(planet, (ha + hb + hc + hd) / 4.0, ym, 1e9);
            out.push(([a, b, c], col));
            out.push(([a, c, d], col));
        }
    }
    out
}

/// Uniform colour variants for the model viewer: (name, palette index of the body).
const VARIANTS: [(&str, f32); 6] = [("blue", 6.0), ("green", 7.0), ("red", 9.0), ("ochre", 4.0), ("grey", 1.0), ("khaki", 13.0)];

/// Whether a point (metres) is inside the testing ground.
fn in_base(x: f32, y: f32) -> bool {
    (BASE_LO + 2.0..BASE_HI - 2.0).contains(&x) && (BASE_LO + 2.0..BASE_HI - 2.0).contains(&y)
}

/// Levels the testing ground: inside the base the ground is flat at the base's height, blending back to the
/// natural terrain over 30 m around it.
fn level_base(h: &mut [f32], cx: i64, cy: i64) {
    let n = CELLS as usize + 1;
    let planet = Planet::demo(42);
    let base_h = planet.height_mm(100_000, (100_000.0 + HALF_DEPTH as f64 * 1000.0) as i64) as f32 / 1000.0;
    for j in 0..n {
        for i in 0..n {
            let (x, y) = ((cx * CELLS as i64 + i as i64) as f32, (cy * CELLS as i64 + j as i64) as f32);
            let out = (BASE_LO - x).max(x - BASE_HI).max(BASE_LO - y).max(y - BASE_HI).max(0.0);
            if out < 30.0 {
                let k = out / 30.0;
                let k = k * k * (3.0 - 2.0 * k);
                h[j * n + i] = base_h + (h[j * n + i] - base_h) * k;
            }
        }
    }
}

/// The testing ground's military base (positions in millimetres). A walled compound with a gate, barracks,
/// guard towers, sandbag lines and containers, and outside it an obstacle lane: a rock field and a narrow gap.
fn base_props() -> Vec<Prop> {
    let mut p = Vec::new();
    let wall = |x: i64, y: i64, len: i64, thick: i64, turns: u8| Prop { kind: PropKind::Wall { length_mm: len, thickness_mm: thick }, x_mm: x, y_mm: y, quarter_turns: turns };
    // Perimeter: a 140 m square from (20, 0) to (160, 140) m, 0.6 m concrete, with a 12 m gate in the south wall.
    p.push(wall(20_000 + 32_000, 0, 64_000, 600, 0));
    p.push(wall(160_000 - 32_000, 0, 64_000, 600, 0));
    p.push(wall(90_000, 140_000, 140_000, 600, 0));
    p.push(wall(20_000, 70_000, 140_000, 600, 1));
    p.push(wall(160_000, 70_000, 140_000, 600, 1));
    // Barracks: three 24 x 8 m blocks along the north.
    for k in 0..3 {
        p.push(wall(45_000 + k * 40_000, 118_000, 24_000, 8_000, 0));
    }
    // Guard towers in the corners and by the gate (3 m footprints).
    for (x, y) in [(24_000, 4_000), (156_000, 4_000), (24_000, 136_000), (156_000, 136_000), (79_000, 5_000), (101_000, 5_000)] {
        p.push(Prop { kind: PropKind::Crate { size_mm: 3_000 }, x_mm: x, y_mm: y, quarter_turns: 0 });
    }
    // Sandbag lines behind the gate, staggered so the way in zig-zags.
    p.push(wall(78_000, 14_000, 10_000, 700, 0));
    p.push(wall(102_000, 22_000, 10_000, 700, 0));
    p.push(wall(68_000, 30_000, 8_000, 700, 1));
    // Container stacks (2.5 x 12 m) in the depot corner.
    for k in 0..4 {
        p.push(wall(140_000, 30_000 + k * 6_000, 12_000, 2_500, 0));
    }
    // Crates scattered by the depot.
    for k in 0..8i64 {
        p.push(Prop { kind: PropKind::Crate { size_mm: 1_200 }, x_mm: 120_000 + (k % 4) * 3_000, y_mm: 60_000 + (k / 4) * 4_000, quarter_turns: 0 });
    }
    // Outside, west: a rock field and a wall with a 2 m gap (soldiers squeeze through one at a time).
    for k in 0..24i64 {
        let h = scalia_core::hash::mix64(k as u64 ^ 0x70c4);
        p.push(Prop { kind: PropKind::Rock { radius_mm: 700 + (h % 1_400) as i64 }, x_mm: -10_000 + (h >> 16) as i64 % 24_000, y_mm: 20_000 + (h >> 32) as i64 % 90_000, quarter_turns: 0 });
    }
    p.push(wall(-2_500, 150_000, 31_000, 600, 0));
    p.push(wall(32_000 - 1_000, 150_000, 34_000, 600, 0));
    p
}

/// Low-poly mesh for the base's props, standing on the levelled pad at height `h` (same style as the soldiers).
fn base_mesh(props: &[Prop], h: f32) -> MeshBuilder {
    use scalia_render::mesh::kit;
    let mut m = MeshBuilder::default();
    let c = |p: u8| scalia_render::mesh::code(p, scalia_render::mesh::Part::Body);
    for p in props {
        let (x, y) = (p.x_mm as f32 / 1000.0, p.y_mm as f32 / 1000.0);
        match p.kind {
            PropKind::Rock { radius_mm } => {
                let r = radius_mm as f32 / 1000.0;
                m.prism([x, y, h - 0.2], [r, r * 0.9], [x + r * 0.1, y, h + r * 0.9], [r * 0.6, r * 0.55], 7, c(1));
            }
            PropKind::Wall { length_mm, thickness_mm } => {
                let (l, t) = (length_mm as f32 / 1000.0, thickness_mm as f32 / 1000.0);
                let (hx, hy) = if p.quarter_turns % 2 == 0 { (l / 2.0, t / 2.0) } else { (t / 2.0, l / 2.0) };
                if t <= 0.65 {
                    // Concrete wall with a cap.
                    m.cuboid([x - hx, y - hy, h], [x + hx, y + hy, h + 2.6], c(2));
                    m.cuboid([x - hx - 0.05, y - hy - 0.05, h + 2.6], [x + hx + 0.05, y + hy + 0.05, h + 2.75], c(1));
                } else if t <= 0.8 {
                    // Sandbags, two courses.
                    m.cuboid([x - hx, y - hy, h], [x + hx, y + hy, h + 0.55], c(kit::STRAP));
                    m.cuboid([x - hx + 0.1, y - hy + 0.05, h + 0.55], [x + hx - 0.1, y + hy - 0.05, h + 1.0], c(kit::PACK));
                } else if t <= 3.0 {
                    // Shipping container.
                    m.cuboid([x - hx, y - hy, h], [x + hx, y + hy, h + 2.6], c(9));
                } else {
                    // Barracks block with a flat roof.
                    m.cuboid([x - hx, y - hy, h], [x + hx, y + hy, h + 3.6], c(kit::UNIFORM));
                    m.cuboid([x - hx - 0.3, y - hy - 0.3, h + 3.6], [x + hx + 0.3, y + hy + 0.3, h + 3.9], c(kit::UNIFORM_DARK));
                }
            }
            PropKind::Crate { size_mm } => {
                let s = size_mm as f32 / 2000.0;
                if size_mm >= 2_500 {
                    // Guard tower: four legs, a platform and a roof.
                    for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                        m.cuboid([x + dx * s - 0.15 * (dx + 1.0), y + dy * s - 0.15 * (dy + 1.0), h], [x + dx * s - 0.15 * (dx - 1.0) - 0.3, y + dy * s - 0.15 * (dy - 1.0) - 0.3, h + 4.5], c(kit::COYOTE_DARK));
                    }
                    m.cuboid([x - s, y - s, h + 4.5], [x + s, y + s, h + 4.8], c(kit::PACK));
                    m.cuboid([x - s, y - s, h + 4.8], [x + s, y + s, h + 5.7], c(kit::STRAP));
                    m.cuboid([x - s - 0.3, y - s - 0.3, h + 6.4], [x + s + 0.3, y + s + 0.3, h + 6.6], c(kit::UNIFORM_DARK));
                } else {
                    m.cuboid([x - s, y - s, h], [x + s, y + s, h + s * 2.0], c(kit::PACK));
                }
            }
        }
    }
    m
}

/// The commander's figure: the soldier in black, with the armband patches in the instance (team) colour.
fn commander_figure() -> MeshBuilder {
    use scalia_render::mesh::{code, kit, palette_of, part_of};
    let mut f = figure();
    for v in &mut f.vertices {
        let to = match palette_of(v.color) as u8 {
            kit::UNIFORM => Some(kit::BALACLAVA),
            kit::UNIFORM_DARK | kit::PACK | kit::HELMET => Some(kit::PAD),
            kit::STRAP | kit::GLOVE => Some(kit::METAL),
            _ => None,
        };
        if let Some(p) = to {
            if v.color >= 0.0 {
                v.color = code(p, part_of(v.color));
            }
        }
    }
    f
}

/// The tablet map's camera: everything that differs between the field view and the map view.
struct MapCam {
    scene: Scene,
    zoom_step: usize,
    zoom_m: f32,
    ground_view: f32,
    sys_scale: f32,
    sys_cam: Option<Vec3>,
    sys_follow: Option<u8>,
    ground_cam: Option<Vec3>,
    cam_smooth: Option<Vec3>,
    cam_ms: f64,
    yaw: f32,
    yaw_smooth: f32,
    last_cam: (Vec3, f32, f32),
    last_view: (f32, f32),
    size: (u32, u32),
    last_frame_ms: f64,
    last_internal: (u32, u32),
}

/// The commander's map marker: a red ring on the ground around him and a diamond above his head, sized in
/// screen pixels (`pix` metres per pixel) but never smaller than him.
fn you_marker(at: [f32; 3], pix: f32) -> MeshBuilder {
    const RED: f32 = 5.0;
    let mut m = MeshBuilder::default();
    let r = (pix * 10.0).max(1.0);
    let w = (pix * 1.6).max(0.12);
    let z = at[2] + 0.05;
    let n = 32;
    let q = |a: f32, rr: f32| [at[0] + a.cos() * rr, at[1] + a.sin() * rr, z];
    for i in 0..n {
        let (a0, a1) = (i as f32 / n as f32 * std::f32::consts::TAU, (i + 1) as f32 / n as f32 * std::f32::consts::TAU);
        m.quad(q(a0, r - w), q(a1, r - w), q(a1, r + w), q(a0, r + w), RED);
    }
    m.diamond([at[0], at[1], at[2] + 2.2 + pix * 8.0], (pix * 4.0).max(0.3), RED);
    m
}
