//! The game viewer in Rust: three scenes drawn by `scalia-render`, driven by the core directly.
//! JavaScript only forwards the animation frame and input.

use std::collections::HashMap;

use glam::Vec3;
use scalia_core::derive::{derive_visible, Derived, Phase, ViewBox};
use scalia_core::flight::{flight_pos, plan_flight, FlightPlan, DEMO_PEAK_SPEED_MM_S};
use scalia_core::planet::Planet;
use scalia_core::groups::GroupId;
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
use scalia_render::mesh::{figure, figure_far, figure_lo, grass_tuft, MeshBuilder};
use scalia_render::pose::Pose;
use scalia_render::sky::{clock, sky};
use wasm_bindgen::prelude::*;

/// Army sizes the N key cycles through, as (groups, soldiers per group).
const ARMIES: [(u32, u32); 4] = [(1, 2000), (10, 2000), (50, 2000), (250, 2000)];
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
/// How quickly the camera glides to the next step (time constant, ms).
const GLIDE_MS: f64 = 90.0;
/// How long the previous terrain detail level lingers over the new one (ms).
const FADE_MS: f64 = 600.0;
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
    order_dest: Option<Vec3>,
    fps: f32,
    frames: u32,
    fps_t: f64,
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
        outline: true,
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
        follow_group: 0,
        stance: HashMap::new(),
        group_pose: HashMap::new(),
        preview_pose: None,
        model_detail: 0,
        model_variant: 0,
        order_dest: None,
        fps: 0.0,
        frames: 0,
        fps_t: 0.0,
        size,
    };
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
    for g in 0..groups {
        let pos = Pos { region: RegionId(1), x: (g % side) as i64 * 200_000, y: (g / side) as i64 * 200_000, z: 0 };
        let spec = GroupSpec { pos, composition: Composition([per_group, 0, 4, 3]), needs: Needs::full() };
        let id = w.groups.spawn(spec, &mut w.roster);
        if g % 3 == 0 {
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
                let y = self.ship_yaw.unwrap_or(0.0);
                self.ship_yaw = Some(y - dx / view_h * 3.0);
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
        self.derived.clear();
        self.ground_cam = None;
        self.selected = Some(0);
        self.follow_group = 0;
        self.group_pose.clear();
        self.stance.clear();
        self.order_dest = None;
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
        match self.selected {
            Some(g) => format!("selected group {} ({} soldiers)", g, self.world.groups.people(GroupId(g))),
            None => "nothing selected".into(),
        }
    }

    /// Right click on the ground: the selected group marches there (redirecting if already marching).
    pub fn command(&mut self, px: f64, py: f64, now_ms: f64, run: bool) -> String {
        if self.scene != Scene::Ground {
            return String::new();
        }
        let Some(g) = self.selected else { return "select a group first (left click)".into() };
        let t = WorldMs(self.sim_ms(now_ms) as u64);
        let _ = self.world.tick(WorldMs(t.0.max(self.world.now.0)));
        let p = self.pick(px, py);
        let to = Pos { region: RegionId(1), x: (p.x as f64 * 1000.0) as i64, y: (p.y as f64 * 1000.0) as i64, z: 0 };
        let speed = if run { RUN_SPEED_MM_S } else { MARCH_SPEED_MM_S };
        match self.world.order_march(GroupId(g), to, speed, WorldMs(t.0.max(self.world.now.0))) {
            Ok(()) => {
                self.order_dest = Some(p);
                self.ground_cam = None;
                self.follow_group = g;
                format!("group {g} {} to ({:.0}, {:.0})", if run { "running" } else { "marching" }, p.x, p.y)
            }
            Err(e) => format!("cannot order group {g}: {e:?}"),
        }
    }

    /// Holds pose `i` in the soldier preview (walk, run, stand, crouch, cover aim, prone, standing aim); -1 cycles.
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
            Pose::Crouch => Pose::CoverAim,
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
    pub fn wheel(&mut self, delta: f64) {
        if self.scene == Scene::Ships || delta == 0.0 {
            return;
        }
        // Away from home the ground is not under the camera: the system's closest step is as far in as it goes.
        let lowest = if self.scene == Scene::System && self.sys_follow != Some(HOME_BODY) { STEP_HANDOFF + 1 } else { 0 };
        self.zoom_step = if delta > 0.0 { (self.zoom_step + 1).min(LADDER.len() - 1) } else { self.zoom_step.saturating_sub(1).max(lowest) };
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
        let t = self.sim_ms(now_ms);
        self.frames += 1;
        if now_ms - self.fps_t > 500.0 {
            self.fps = (self.frames as f64 * 1000.0 / (now_ms - self.fps_t)) as f32;
            self.frames = 0;
            self.fps_t = now_ms;
        }
        let ps = if self.full { 1 } else { self.pixel_scale };
        let internal = ((self.size.0 / ps).max(1), (self.size.1 / ps).max(1));
        self.glide(now_ms);
        let (mut meshes, mut instanced, mut decor) = (Vec::new(), Vec::new(), Vec::new());
        let (target, yaw, view_height, range, text) = match self.scene {
            Scene::Ships => self.ships_scene(t, &mut meshes, &mut instanced),
            Scene::Ground => self.ground_scene(t, &mut meshes, &mut instanced, &mut decor),
            Scene::System => self.system_scene(t, &mut meshes),
        };
        self.last_view = (view_height, self.size.1 as f32);
        self.last_cam = (target, yaw, view_height);
        let tod = self.time_of_day(t) as f32;
        let (sky_now, atmosphere) = match self.scene {
            // Close to home the planet keeps the ground's light, so the hand-over does not flash.
            Scene::System if self.sys_follow == Some(HOME_BODY) && self.sys_scale < 50.0 => (sky(tod), false),
            Scene::System => (sky(0.5), false),
            Scene::Ground => (sky(tod), self.ground_view < 40_000.0),
            Scene::Ships => (sky(tod), true),
        };
        let bend = if self.scene == Scene::Ground { self.ground_bend() } else { Bend::FLAT };
        // Outlines on terrain cells only a few pixels wide turn into shimmering stripes: off for far views.
        let far_view = match self.scene {
            Scene::Ground => self.ground_view > 20_000.0,
            Scene::System => self.sys_scale < 50.0,
            Scene::Ships => false,
        };
        // Wrap once a day: f32 seconds stay precise to about 8 ms, and the jump comes only every 24 hours.
        let time_s = ((t / 1000.0) % 86_400.0) as f32;
        let desc = FrameDesc { meshes: &meshes, instanced: &instanced, decor: &decor, target, yaw, view_height, range, internal, outline: self.outline && !far_view, sky: sky_now, atmosphere, time_s, bend };
        if let Err(e) = self.gpu.frame(&desc) {
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
    fn group_centre(&self, g: u32, t: WorldMs) -> Vec3 {
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
            const SHOW: [(Pose, f32); 7] = [
                (Pose::Walk, 0.9), (Pose::Run, 1.35), (Pose::Stand, 0.0), (Pose::Crouch, 0.0),
                (Pose::CoverAim, 0.0), (Pose::Prone, 0.0), (Pose::StandAim, 0.0),
            ];
            let (pose, steps) = SHOW[self.preview_pose.unwrap_or((t / 3000.0) as usize) % SHOW.len()];
            let ts = ((t / 1000.0) % 86_400.0) as f32;
            let prev = if self.preview_pose.is_none() { SHOW[((t / 3000.0) as usize + SHOW.len() - 1) % SHOW.len()].0 } else { pose };
            let changed = ((t / 3000.0).floor() * 3.0) as f32 % 86_400.0;
            let colour = VARIANTS[self.model_variant as usize % VARIANTS.len()].1;
            self.gpu.upload_instances("figure1", bytes(&[0.0, 0.0, 0.0, colour, 0.0, 0.0, steps, pose as u32 as f32, prev as u32 as f32, changed.min(ts), 0.0, 0.0]));
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
            return (centre, yaw, size, 10.0, format!("Soldier, {variant}: {tris} triangles, {detail}, pose {pose:?}{cycling}"));
        }
        let s = self.ship_info(i);
        let ext = s.hi - s.lo;
        let size = ext.max_element();
        let text = format!("{}: {} triangles, {:.1} m long", s.name, s.triangles, ext.y);
        let centre = (s.lo + s.hi) / 2.0;
        meshes.push(format!("ship{i}"));
        let yaw = self.ship_yaw.unwrap_or((t / 2500.0) as f32);
        (centre, yaw, size, size * 3.0, text)
    }

    /// Heights of a chunk in metres, generated once from the planet.
    fn chunk(&mut self, cx: i64, cy: i64) -> &Vec<f32> {
        self.chunks
            .entry((cx, cy))
            .or_insert_with(|| Planet::demo(42).chunk(cx, cy + EQUATOR_CHUNK, CELLS).heights.iter().map(|&h| h as f32 / 1000.0).collect())
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
                // Contour bands from the quad's mean height; wider bands on coarse meshes so they stay smooth.
                let mean = (a[2] + b[2] + c[2] + d[2]) / 4.0;
                let col = 10.0 + ((mean * 8.0 / step as f32).floor().rem_euclid(4.0));
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
        if o != self.origin {
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
        for d in &out {
            let (ax, ay) = (d.pos.x as f64 / 1000.0, d.pos.y as f64 / 1000.0);
            let z = self.height_at(ax as f32, ay as f32);
            let (x, y) = ((ax - self.origin.0) as f32, (ay - self.origin.1) as f32);
            // Facing: along the march, else the formation's front with a little personal variation.
            let jitter = (scalia_core::hash::mix64(d.slot as u64 ^ 0x5ca1) % 1000) as f32 / 1000.0;
            let heading = match self.world.groups.process(d.group).kind {
                ProcessKind::March { from, to, .. } => ((from.x - to.x) as f32).atan2((to.y - from.y) as f32),
                _ => (jitter - 0.5) * 0.4,
            };
            let walking = d.phase == Phase::Marching || d.phase == Phase::Walking || d.phase == Phase::Climbing;
            let running = walking && matches!(self.world.groups.process(d.group).kind, ProcessKind::March { speed_mm_s, .. } if speed_mm_s >= RUN_SPEED_MM_S);
            let (pose, steps) = if running {
                (Pose::Run, 1.35 + jitter * 0.1)
            } else if walking {
                (Pose::Walk, 0.9 + jitter * 0.1)
            } else {
                (self.stance.get(&d.group.0).copied().unwrap_or(Pose::Stand), 0.0)
            };
            let body = if self.selected == Some(d.group.0) { 14.0 } else if walking { 7.0 } else { 6.0 };
            // Pose changes are tracked per group and ripple through it: each soldier starts up to 0.4 s late.
            let entry = self.group_pose.entry(d.group.0).or_insert((pose, pose, -1000.0));
            if entry.0 != pose {
                *entry = (pose, entry.0, ts);
            }
            let (prev, since) = (entry.1, entry.2 + jitter * 0.4);
            inst.extend_from_slice(&[x, y, z, body, heading, jitter, steps, pose as u32 as f32, prev as u32 as f32, since, 0.0, 0.0]);
        }
        self.derived = out;
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
        let far: Vec<_> = self.chunks.keys().copied().filter(|&(x, y)| (x - gx).abs() > keep || (y - gy).abs() > keep).collect();
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
                m.diamond([rx, ry, h + pix * 3.0], pix * 2.0, if self.selected == Some(i) { 14.0 } else { 9.0 });
            }
            self.gpu.upload("markers", m.bytes());
            meshes.push("markers".into());
        }
        // Full figure close up, the lighter one when zoomed out.
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
        let mut target = self.ground_cam.unwrap_or(self.focus);
        // Zoomed far out the camera sinks to the planet's centre, so the globe ends up in the middle of the screen.
        let sink = smooth_log(self.ground_view, 400_000.0, 2_400_000.0);
        let ground_z = if self.ground_view <= NEAR_MAX { self.height_at(target.x, target.y) } else { self.surface_m(target.x, target.y) };
        target.z = ground_z + (-PLANET_R - ground_z) * sink;
        let target = Vec3::new((target.x as f64 - self.origin.0) as f32, (target.y as f64 - self.origin.1) as f32, target.z);
        let mode = if self.ground_cam.is_some() { "free camera (F follows)".to_string() } else { format!("following group {lead}") };
        let sel = match self.selected { Some(g) => format!(", group {g} selected"), None => String::new() };
        let total: u64 = (0..self.world.groups.len()).map(|i| self.world.groups.people(scalia_core::groups::GroupId(i as u32)) as u64).sum();
        let text = format!("Ground: {n} of {total} soldiers drawn, {mode}{sel}, {:.0} m view", self.ground_view);
        let range = 600f32.max(self.ground_view * 2.0 + PLANET_R * 2.0 * self.ground_bend().t);
        (target, self.yaw, self.ground_view, range, text)
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
        // Cells about five screen pixels wide: any finer and the globe shimmers as the camera moves.
        let want = ((self.ground_view / 64.0).log2().ceil() as i32).clamp(1, 20) as u32;
        let view = self.ground_view;
        let needed = |level: u32| -> Vec<(u32, i64, i64)> {
            let ts = 64.0 * (1u64 << level) as f32;
            let cov = (1.6 * view).min(PLANET_R * std::f32::consts::PI);
            let (x0, x1) = (((tg.x - cov) / ts).floor() as i64, ((tg.x + cov) / ts).floor() as i64);
            let (y0, y1) = (((tg.y - cov).max(-HALF_DEPTH) / ts).floor() as i64, ((tg.y + cov).min(HALF_DEPTH - 1.0) / ts).floor() as i64);
            (y0..=y1).flat_map(|y| (x0..=x1).map(move |x| (level, x, y))).collect()
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
    } else if cell_m < 256.0 {
        // The same contour bands as the close-up chunks.
        10.0 + (mean_m * 8.0 / cell_m).floor().rem_euclid(4.0)
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
