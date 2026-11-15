//! The game viewer in Rust: three scenes drawn by `scalia-render`, driven by the core directly.
//! JavaScript only forwards the animation frame and input.

use std::collections::HashMap;

use glam::Vec3;

/// Acoustic obstruction uses the propagation field for an on-foot listener, where diffraction around
/// corners and open doors matters. The tablet listener is physically at camera height, so trace the actual
/// 3D line instead of incorrectly applying a ground-level field to an elevated listener.
fn sound_occlusion(
    sight: &scalia_ai::sight::SightGrid,
    field: Option<&scalia_audio::Field>,
    ear: (f32, f32),
    source: (f32, f32),
    listener_height: f32,
    camera_listener: bool,
) -> scalia_audio::Occlusion {
    let height = if camera_listener { listener_height } else { 1.6 };
    if camera_listener {
        let walls = scalia_audio::walls_between(sight, (ear.0, ear.1, height), (source.0, source.1, 1.4));
        return scalia_audio::Occlusion { wall_db: walls as f32 * 40.0, bend_db: 0.0, detour_m: 0.0 };
    }
    if let Some(occlusion) = field.and_then(|field| field.at(source)) {
        return occlusion;
    }
    let walls = scalia_audio::walls_between(sight, (ear.0, ear.1, height), (source.0, source.1, 1.4));
    scalia_audio::Occlusion { wall_db: walls as f32 * 40.0, bend_db: 0.0, detour_m: 0.0 }
}
use scalia_core::derive::{derive_visible, Derived, Phase, ViewBox};
use scalia_core::flight::{flight_pos, plan_flight, FlightPlan, DEMO_PEAK_SPEED_MM_S};
use scalia_core::nav::{Furnish, NavGrid, Prop, PropKind};
use crate::ground::*;
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
use scalia_render::mesh::{figure, figure_far, figure_lo, grass_tuft, MeshBuilder};
use scalia_render::pose::stride_m;
use scalia_render::pose::Pose;
use scalia_render::sky::{clock, sky};
use wasm_bindgen::prelude::*;

/// Army sizes the N key cycles through, as (groups, soldiers per group).
/// Army sizes (groups, soldiers each). The first is the testing ground's: two squads of twelve in the barracks.
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

const ARMIES: [(u32, u32); 5] = [(2, 12), (1, 2000), (10, 2000), (50, 2000), (250, 2000)];
/// The testing ground: a military base on a levelled pad, (BASE_LO..BASE_HI) metres on both axes.
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
/// How far round the ears sound is followed through and round things (m); beyond, straight lines.
const AUDIO_FIELD_M: f32 = 32.0;
/// From the commander, where gunfire starts to sound distant (m): the edge of the closest view.
const FAR_FROM_M: f32 = 12.0;
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
/// The commander's own squad number on the field battle (he is its only member) and his spare magazines (150
/// rounds behind the 30 in the rifle; later the inventory decides).
const COMMANDER_SQUAD: u16 = 1_000;
const COMMANDER_MAGS: i32 = 5;
const CMD_WALK: f32 = 1.7;
const CMD_AIM_WALK: f32 = 1.2;
const CMD_CROUCH: f32 = 1.4;
const CMD_CRAWL: f32 = 0.5;
const CMD_SPRINT: f32 = 4.6;
/// His usual pace on foot (WASD): a jog. Alt held walks (CMD_WALK), Shift sprints.
const CMD_JOG: f32 = 3.0;
/// The commander winds up (m/s²) and brakes.
/// Selection tag on an instance's colour (the shader outlines tagged figures): +64 stable, +128 wounded or
/// incapacitated, +192 killed.
const SEL_STABLE: f32 = 64.0;
/// His feet (motion.rs): sharp up to a jog, a build-up into a sprint, a quick stop, a gentle ease off a sprint.
const CMD_FEET: crate::motion::Footing = crate::motion::Footing { push: 10.0, kick: 3.0, brake: 10.0, coast: 3.0, cruise: CMD_JOG, top: CMD_SPRINT };

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
    /// Which of the field battle's doors are steel (the rest wood), in its door order.
    door_steel: Vec<bool>,
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
    /// Alt held: walk instead of jog.
    cmd_walk: bool,
    cmd_moving: bool,
    /// His real pace (m/s), winding up to the walk or sprint and braking to a stop; the way he last moved; and his
    /// gait phase, integrated from the cadence so the legs never jump as the pace changes.
    cmd_speed: f32,
    cmd_dir: (f32, f32),
    /// Skidding: a 180 cut (plant, pivot, push off) or a sprint let go of (a short slide to a stop).
    cmd_skid: crate::motion::Skid,
    /// How fast he is turning (rad/s, + to the left): turns build up and ease off.
    cmd_spin: f32,
    /// Console god mode: the commander cannot be hurt.
    god: bool,
    /// His upper body (the layer, or the pose's own): now, the one before, since when (shader clock, s).
    /// His animator (anim.rs): the same library every soldier is drawn with.
    cmd_fig: crate::anim::Animator,
    /// Which roofs are drawn (house footprint and size, house roof on, barracks roof on): rebuilt on change.
    roof_key: (i64, i64, i64, bool, bool),
    /// Each door's last change (battle ms) and the frame time it was first drawn, so its swing plays in real
    /// time whatever the battle clock is doing (stopped outside a battle, scaled in the lab).
    door_seen: Vec<(i64, f64)>,
    /// AI lab: the scenario laid out last and its layout seed (Deploy fills that same layout).
    lab_name: String,
    lab_seed: u64,
    cmd_gait: f64,
    cmd_pose: (Pose, Pose, f32),
    /// Per figure (key): last heading, smoothed turn rate (rad/s), when (ms), last speed (m/s), smoothed
    /// acceleration (m/s²): for banking into turns and leaning with a push off or a stop.
    turns: HashMap<u64, (f32, f32, f64, f32, f32)>,
    /// The commander's rifle and stance, worked by the player on foot.
    gun: Gun,
    /// The live killhouse assault, when one is running (replaces the static enemy posts).
    assault: Option<crate::assault::Assault>,
    /// The K training assault is running on the field.
    demo: bool,
    /// The AI lab's clock: battle time runs at `lab_speed` (0 paused, 0.25 slow motion, 1 real time).
    lab_speed: f64,
    lab_t: f64,
    lab_prev: f64,
    /// A lab scenario's casualty, applied once his squad has joined: (squad, which man, health).
    lab_wound: Option<(u16, usize, i32)>,
    /// The acoustic space per 2 m square of the testing ground (cleared when the map changes).
    audio_spaces: std::collections::HashMap<(i32, i32), scalia_audio::Env>,
    audio_reflectors: std::collections::HashMap<(i32, i32), Vec<(f32, f32)>>,
    /// How sound reaches the commander's ears through and round everything near him, and what it was built for
    /// (his metre square, the doors' states).
    audio_field: Option<(scalia_audio::Field, (i32, i32, u64))>,
    /// How the next squad move is to be made (chosen on the radio).
    order_mode: Option<scalia_ai::leader::Mode>,
    /// Muzzle flashes, tracers, hits and blasts on screen, and this frame's lights from them.
    fx: crate::effects::Effects,
    fx_lights: Vec<scalia_render::gpu::PointLight>,
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
    /// The tablet map's camera flying to a point: (from, to, start ms, duration ms, zoom step on arrival).
    map_fly: Option<(Vec3, Vec3, f64, f64, usize)>,
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
    inspect_zoom: Option<usize>,
    inspect_glide: bool,
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
    /// The main menu's intro flight, driven frame by frame from the page: view height (m), camera elevation
    /// (radians) and how far the commander has dissolved (the camera passing into him).
    intro: Option<(f32, f32, f32)>,
    size: (u32, u32),
}

/// Creates the viewer on a canvas. Fails with a readable message when WebGPU is unavailable.
#[wasm_bindgen]
extern "C" {
    /// The page's crash reporter (errors.js): told the panic's message and place before the module dies.
    #[wasm_bindgen(js_namespace = window, js_name = scaliaPanic, catch)]
    fn scalia_panic(msg: &str) -> Result<(), JsValue>;
}

#[wasm_bindgen]
pub async fn start_viewer(canvas: web_sys::HtmlCanvasElement) -> Result<Viewer, JsError> {
    // A panic says what and where (message, file:line) to the crash screen instead of a bare "unreachable".
    std::panic::set_hook(Box::new(|info| {
        let _ = scalia_panic(&info.to_string());
    }));
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
        door_steel: crate::ground::door_steel(),
        base_grid: NavGrid::new((BASE_LO * 1000.0) as i64, (BASE_LO * 1000.0) as i64, ((BASE_HI - BASE_LO) * 2.0) as usize, ((BASE_HI - BASE_LO) * 2.0) as usize),
        base_h: 0.0,
        base_built: false,
        agents: HashMap::new(),
        agent_prev: HashMap::new(),
        agent_clock: -1.0,
        cmd: (70_000.0 / 1000.0 + 20.0, 20.0),
        cmd_heading: 0.0,
        cmd_input: (0.0, 0.0, false),
        cmd_walk: false,
        cmd_moving: false,
        cmd_speed: 0.0,
        cmd_dir: (0.0, 1.0),
        cmd_skid: crate::motion::Skid::None,
        cmd_spin: 0.0,
        god: false,
        cmd_fig: crate::anim::Animator::new(0.0),
        roof_key: (0, 0, 0, false, false),
        door_seen: Vec::new(),
        lab_name: String::new(),
        lab_seed: 0,
        cmd_gait: 0.37,
        cmd_pose: (Pose::Stand, Pose::Stand, -1000.0),
        turns: HashMap::new(),
        gun: Gun::default(),
        assault: None,
        demo: false,
        lab_speed: 1.0,
        lab_t: 0.0,
        lab_prev: 0.0,
        lab_wound: None,
        audio_spaces: std::collections::HashMap::new(),
        audio_reflectors: std::collections::HashMap::new(),
        audio_field: None,
        order_mode: None,
        fx: crate::effects::Effects::default(),
        fx_lights: Vec::new(),
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
        map_fly: None,
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
        inspect_zoom: None,
        inspect_glide: false,
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
        // The day stands still for now (the time of day is not part of the game yet); T cycles it.
        day_rate: 0.0,
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
        intro: None,
        size,
    };
    v.add_captains();
    v.gpu.upload("figure", figure().bytes());
    v.gpu.upload("figure_lo", figure_lo().bytes());
    v.gpu.upload("figure_far", figure_far().bytes());
    v.gpu.upload("enemy", enemy_kit(figure()).bytes());
    v.gpu.upload("enemy_lo", enemy_kit(figure_lo()).bytes());
    v.gpu.upload("enemy_far", enemy_kit(figure_far()).bytes());
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
        // Testing squads live in the barracks (one bay each, more on the parade ground south of HQ); big armies
        // spread over the plain.
        let pos = if testing && g < 2 {
            let (x, y) = bay_centre(g as usize);
            Pos { region: RegionId(1), x, y, z: 0 }
        } else if testing {
            Pos { region: RegionId(1), x: 80_000 + (g % 4) as i64 * 10_000, y: 50_000 + (g / 4) as i64 * 10_000, z: 0 }
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

/// Benchmark army: `squads` squads of 12 in a grid around the commander's start (16 m apart),
/// every third one marching, so the near renderer draws every soldier at full detail.
fn bench_army(squads: u32) -> World {
    let mut w = World::new(42);
    let side = (squads as f64 + 1.0).sqrt().ceil() as i64;
    let (cx, cy) = (90_000i64, 20_000i64);
    let mut g = 0;
    let mut k = 0i64;
    while g < squads {
        let (i, j) = (k % side - side / 2, k / side - side / 2);
        k += 1;
        if i == 0 && j == 0 {
            continue; // leave the commander's square free
        }
        let pos = Pos { region: RegionId(1), x: cx + i * 16_000, y: cy + j * 16_000, z: 0 };
        let spec = GroupSpec { pos, composition: Composition([12, 0, 0, 0]), needs: Needs::full() };
        let id = w.groups.spawn(spec, &mut w.roster);
        if g % 3 == 0 {
            let _ = w.start_march(id, pos.offset(60_000, 0, 0), MARCH_SPEED_MM_S, WorldMs(0));
        }
        g += 1;
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
            // None through the concrete floors (the live-fire range, the barracks).
            let (wx, wy) = (((cx * CELLS as i64 + i as i64) as f64 + fx as f64) * 1000.0, ((cy * CELLS as i64 + j as i64) as f64 + fy as f64) * 1000.0);
            let on = |r: (i64, i64, i64, i64)| wx >= r.0 as f64 && wx <= r.2 as f64 && wy >= r.1 as f64 && wy <= r.3 as f64;
            if on(range_mm()) || on(barracks_mm()) {
                continue;
            }
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
    /// Starts the training assault: a squad of 12 forms up south of the killhouse and takes it from the 8
    /// enemy soldiers inside, all on the battle AI. Starting again restarts it.
    pub fn start_assault(&mut self, _seed: u32) {
        if self.assault.as_ref().is_some_and(|a| a.has_squad(crate::assault::DEMO_SQUAD)) {
            self.stop_assault();
        }
        let a = self.assault.get_or_insert_with(|| unreachable!("the field exists once the base is built"));
        // Outside the range by its door: the briefing, then in.
        let at: Vec<(i64, i64)> = (0..12i64).map(|i| (range_mm().2 + 8_000 + (i % 4) * 1_500, range_door().1 - 3_000 + (i / 4) * 1_500)).collect();
        a.spawn_squad(crate::assault::DEMO_SQUAD, 0, &at);
        a.order(crate::assault::DEMO_SQUAD, scalia_ai::leader::Order::Capture);
        self.demo = true;
    }
    /// Ends the assault: the testing ground starts over (the enemy back on their posts).
    pub fn stop_assault(&mut self) {
        self.assault = Some(self.new_field());
        self.demo = false;
    }

    /// The testing ground's battle: the whole compound and its props, the enemy squad in the training house.
    fn new_field(&self) -> crate::assault::Assault {
        let (lo, size) = ((BASE_LO * 1000.0) as i64, ((BASE_HI - BASE_LO) * 1000.0) as i64);
        let mut map = scalia_ai::save::MapSpec::new(lo, lo, size, size, &self.base_props);
        map.doors = battle_doors();
        map.killhouse = Some(killhouse_mm());
        map.live = Some(range_live_mm());
        crate::assault::Assault::field(map, killhouse_mm(), 7, self.lab_t)
    }
    /// Battle (lab clock) time to viewer time: what happens later in the battle happens later on screen, slower
    /// in slow motion.
    fn lab_real(&self, at: f64) -> f64 {
        if self.lab_speed <= 0.0 { self.frame_ms } else { self.frame_ms + (at - self.lab_t) / self.lab_speed }
    }
    /// Console: list the doors, or set one ("door 3 open|close|lock|breach").
    pub fn door_cmd(&mut self, which: i32, state: &str) -> String {
        use scalia_ai::doors::DoorState;
        let Some(a) = self.assault.as_mut() else { return "no battle on the testing ground".into() };
        if which < 0 {
            return a.battle.doors.iter().enumerate().map(|(i, d)| format!("{i:2} ({:.1}, {:.1}) {}{:?}", d.at.0 as f64 / 1000.0, d.at.1 as f64 / 1000.0, if self.door_steel.get(i) == Some(&true) { "steel " } else { "" }, d.state)).collect::<Vec<_>>().join("\n");
        }
        let st = match state {
            "open" => DoorState::Open,
            "close" | "closed" => DoorState::Closed,
            "lock" | "locked" => DoorState::Locked,
            "breach" | "breached" => DoorState::Breached,
            _ => return "door <n> open|close|lock|breach".into(),
        };
        if which as usize >= a.battle.doors.len() {
            return format!("no door {which} (0..{})", a.battle.doors.len());
        }
        a.battle.set_door(which as usize, st, None);
        format!("door {which}: {:?}", a.battle.doors[which as usize].state)
    }
    /// AI lab: how fast battle time runs (0 pauses).
    pub fn lab_set_speed(&mut self, speed: f64) {
        self.lab_speed = speed.clamp(0.0, 4.0);
    }
    /// AI lab: clears the testing ground and sets up a scenario; the commander is placed to watch it. Returns
    /// a line saying what to watch for.
    /// AI lab: lays a scenario out (a new compound layout for house clearing), with nobody on it yet.
    pub fn lab_start(&mut self, name: &str) -> String {
        self.lab_seed = scalia_core::hash::mix64(self.frame_ms.to_bits() ^ 0x1ab5);
        self.lab_name = name.to_string();
        self.lab_setup(name, false)
    }
    /// AI lab: sends the troops of the scenario laid out last onto the same layout.
    pub fn lab_deploy(&mut self) -> String {
        let name = self.lab_name.clone();
        self.lab_setup(&name, true)
    }
    fn lab_setup(&mut self, name: &str, deploy: bool) -> String {
        use crate::assault::{DEMO_SQUAD, LAB_RED};
        use scalia_ai::leader::{Mode, Order};
        let (lo, size) = ((BASE_LO * 1000.0) as i64, ((BASE_HI - BASE_LO) * 1000.0) as i64);
        // House clearing happens in the training compound east of the base, laid out afresh every run (the
        // house, its ways in, the furniture, the cover outside and where the enemy stands); the other scenarios
        // use the fixed testing ground.
        let comp = !matches!(name, "contact" | "wounded" | "standoff");
        let seed = self.lab_seed;
        let c = comp.then(|| crate::ground::compound(seed));
        let props = c.as_ref().map_or_else(base_props, |c| c.props.clone());
        let mut map = scalia_ai::save::MapSpec::new(lo, lo, size, size, &props);
        map.doors = battle_doors();
        map.killhouse = Some(killhouse_mm());
        // The fixed house's scenarios are on the live-fire range (live only inside its walls); the open-ground
        // ones are live everywhere.
        let house = matches!(name, "contact");
        map.live = if house { Some(range_live_mm()) } else { None };
        if let Some(c) = &c {
            map.doors = c.doors.clone();
            map.killhouse = Some(c.house.origin);
            map.killhouse_seed = Some(c.seed);
            map.live = Some(c.live);
        }
        let mut a = crate::assault::Assault::field_with(map, killhouse_mm(), 7, self.lab_t, house && deploy);
        // Which of the battle's doors are steel (by place: the compound's door list differs from the world's).
        let steel: Vec<(i64, i64)> = battle_doors().iter().zip(crate::ground::door_steel()).filter(|(_, s)| *s).map(|(d, _)| d.at).collect();
        self.door_steel = a.battle.doors.iter().map(|d| steel.contains(&d.at)).collect();
        // The ground drawn and walked on is the battle's map (the compound comes and goes with the scenario).
        self.base_props = a.battle.map_spec().map(|m| m.props()).unwrap_or_else(base_props);
        self.base_grid.rebuild(&self.base_props);
        self.audio_spaces.clear();
        self.audio_reflectors.clear();
        self.audio_field = None;
        let m = base_mesh(&self.base_props, self.base_h);
        self.gpu.upload("base", m.bytes());
        self.demo = false;
        self.lab_wound = None;
        let block = |x0: i64, y0: i64, n: i64| -> Vec<(i64, i64)> { (0..n).map(|i| (x0 + (i % 4) * 1_500, y0 + (i / 4) * 1_500)).collect() };
        let door = (range_mm().2, range_door().1);
        let (watch, note) = match name {
            "contact" => {
                // Inside the range, along its east side, past the house 6 m off its east wall.
                if deploy { a.spawn_squad(DEMO_SQUAD, 0, &block(range_mm().2 - 7_000, range_mm().1 + 2_000, 8)); }
                if deploy { a.order(DEMO_SQUAD, Order::Move { to: (range_mm().2 - 5_000, range_mm().3 - 3_000), mode: Mode::Plain }); }
                // The house's doors stand open: its defenders see (and are seen) through them as Blue goes by.
                for d in 0..a.battle.doors.len() {
                    a.battle.set_door(d, scalia_ai::doors::DoorState::Open, None);
                }
                ((range_mm().2 - 8_000, door.1), "On the range, Blue walks up past the house 6 m off its east wall; the defenders open up through the windows and the east door. Watch who takes cover, where, and when they move on.")
            }
            "wounded" => {
                if deploy { a.spawn_squad(DEMO_SQUAD, 0, &block(95_000, 40_000, 8)); }
                if deploy { a.order(DEMO_SQUAD, Order::Hold); }
                if deploy { a.spawn_squad(LAB_RED, 1, &block(130_000, 40_000, 3)); }
                if deploy { a.order(LAB_RED, Order::Hold); }
                self.lab_wound = deploy.then_some((DEMO_SQUAD, 2, 25));
                ((105_000, 42_000), "One Blue man starts down and bleeding with Red 35 m east. Watch who goes to him and when (not into fire), the drag to cover, and the aid.")
            }
            "standoff" => {
                if deploy { a.spawn_squad(DEMO_SQUAD, 0, &block(78_000, 92_000, 8)); }
                if deploy { a.order(DEMO_SQUAD, Order::Move { to: (142_000, 95_000), mode: Mode::Confirmed }); }
                if deploy { a.spawn_squad(LAB_RED, 1, &block(140_000, 92_000, 8)); }
                if deploy { a.order(LAB_RED, Order::Move { to: (80_000, 95_000), mode: Mode::Confirmed }); }
                ((110_000, 95_000), "Blue and Red, eight each with the same brain, advance on each other across the open north of the base. Watch bounding, cover and fire.")
            }
            _ => {
                let c = c.as_ref().expect("the compound is laid out for house clearing");
                if deploy { a.spawn_squad(crate::assault::ENEMY_SQUAD, 1, &c.posts); }
                if deploy { a.order(crate::assault::ENEMY_SQUAD, Order::Hold); }
                if deploy { a.spawn_squad(DEMO_SQUAD, 0, &c.start); }
                if deploy { a.order(DEMO_SQUAD, Order::Capture); }
                let (o, n) = (c.house.origin, c.house.rooms.len());
                let note = format!(
                    "Training compound on the range (layout {:x}): a new house every run, {n} rooms, {} ways in, {} defenders posted at random inside, cover all around. Blue (12) takes it from the south. Press Deploy to send the troops in.",
                    c.seed & 0xffff,
                    c.house.doors.iter().filter(|d| d.rooms.0.is_none() || d.rooms.1.is_none()).count(),
                    c.posts.len()
                );
                self.assault = Some(a);
                let _ = o;
                if deploy {
                    return "Deployed: Blue (12) moves on the house.".into();
                }
                self.cmd = ((range_mm().2 + 4_000) as f64 / 1000.0, (c.start[0].1 + 4_000) as f64 / 1000.0);
                return note;
            }
        };
        self.assault = Some(a);
        if deploy {
            return "Deployed.".into();
        }
        self.cmd = (watch.0 as f64 / 1000.0, watch.1 as f64 / 1000.0);
        format!("{note} Press Deploy to send the troops in.")
    }
    /// AI lab: the feed of leader steps and shouts since the last call, one per line.
    pub fn lab_feed(&mut self) -> String {
        self.assault.as_mut().map_or(String::new(), |a| a.lab_log.drain(..).collect::<Vec<_>>().join("\n"))
    }
    /// AI lab: every soldier on the testing ground on screen with what he is doing, as JSON
    /// [{x, y (screen fractions, over his head), t (team), txt}].
    pub fn lab_marks(&mut self) -> String {
        let Some(a) = self.assault.take() else { return "[]".into() };
        let (target, yaw, view_h) = self.last_cam;
        let (r, u, _) = scalia_render::camera::basis(yaw);
        let aspect = self.size.0 as f32 / self.size.1.max(1) as f32;
        let mut out = Vec::new();
        for i in 0..a.battle.soldiers.len() {
            let (x, y) = a.position(i);
            let (x, y) = (x / 1000.0, y / 1000.0);
            let up = if a.battle.soldiers[i].alive() && !a.battle.soldiers[i].down() { 2.1 } else { 0.6 };
            let rel = Vec3::new((x - self.origin.0) as f32, (y - self.origin.1) as f32, self.height_at(x as f32, y as f32) + up);
            let d = rel - target;
            let (sx, sy) = (0.5 + d.dot(r) / (view_h * aspect), 0.5 - d.dot(u) / view_h);
            if !(-0.05..=1.05).contains(&sx) || !(-0.05..=1.05).contains(&sy) {
                continue;
            }
            let txt = crate::assault::lab_state(&a.battle, i).replace('"', "'");
            out.push(format!("{{\"i\":{i},\"x\":{sx:.4},\"y\":{sy:.4},\"t\":{},\"txt\":\"#{i} {txt}\"}}", a.battle.soldiers[i].team));
        }
        self.assault = Some(a);
        format!("[{}]", out.join(","))
    }
    pub fn assault_state(&mut self) -> String {
        use crate::assault::{DEMO_SQUAD, ENEMY_SQUAD};
        let demo = self.demo;
        let Some(a) = self.assault.as_mut() else { return "{\"running\":false}".into() };
        if !demo {
            a.steps.clear();
            return "{\"running\":false}".into();
        }
        let (att, def) = (a.squad_alive(DEMO_SQUAD), a.squad_alive(ENEMY_SQUAD));
        let steps: Vec<String> = a.steps.drain(..).filter(|(sq, _)| *sq == DEMO_SQUAD as usize || *sq == ENEMY_SQUAD as usize).map(|(squad, step)| format!("{{\"squad\":{},\"step\":\"{step}\"}}", if squad == DEMO_SQUAD as usize { 0 } else { 1 })).collect();
        let over = att == 0 || def == 0;
        if over {
            self.demo = false;
        }
        format!("{{\"running\":true,\"over\":{over},\"attackers\":{att},\"defenders\":{def},\"steps\":[{}]}}", steps.join(","))
    }
    /// The field battle in words (debugging: squads, positions, steps).
    pub fn field_debug(&self) -> String {
        self.assault.as_ref().map_or("no field".into(), |a| a.describe())
    }
    /// Where the ears are: the camera (x, y, height; m). Above the middle of the view, as high as the view is
    /// big; while the tablet is up, above the middle of its map.
    fn audio_listener(&self) -> (f32, f32, f32) {
        if self.tablet {
            if let Some(c) = self.map_cam.ground_cam {
                return (c.x, c.y, scalia_audio::camera_height_m(self.map_cam.ground_view));
            }
        }
        let (target, _, view_h) = self.last_cam;
        let (x, y) = (target.x as f64 + self.origin.0, target.y as f64 + self.origin.1);
        (x as f32, y as f32, scalia_audio::camera_height_m(view_h))
    }
    /// The space at the listener's spot on the ground (ambience and the console):
    /// [enclosure 0..1, size (m), reverb time (s), corridor 0..1, through the tablet 0/1].
    pub fn audio_env(&mut self) -> Vec<f32> {
        let l = self.audio_listener();
        let env = self.space_at(l.0, l.1);
        vec![env.enclosure, env.size_m, env.rt60_s, env.corridor, if self.tablet { 1.0 } else { 0.0 }]
    }
    /// What a sound made at a spot bounces off, remembered per 2 m square until the map changes.
    fn reflectors_at(&mut self, x: f32, y: f32) -> Vec<(f32, f32)> {
        let key = ((x / 2.0).floor() as i32, (y / 2.0).floor() as i32);
        if let Some(r) = self.audio_reflectors.get(&key) {
            return r.clone();
        }
        let r = self.assault.as_ref().map_or(Vec::new(), |a| scalia_audio::reflectors(&a.battle.sight, (key.0 as f32 * 2.0 + 1.0, key.1 as f32 * 2.0 + 1.0)));
        if self.audio_reflectors.len() > 20_000 {
            self.audio_reflectors.clear();
        }
        self.audio_reflectors.insert(key, r.clone());
        r
    }
    /// Where the ears are for what stands between: the commander's head on foot, the camera on the tablet.
    fn ear(&self) -> (f32, f32) {
        if self.tablet {
            let l = self.audio_listener();
            return (l.0, l.1);
        }
        (self.cmd.0 as f32, self.cmd.1 as f32)
    }
    /// The propagation field round the ears, rebuilt when they move a metre or a door opens or shuts.
    fn ensure_field(&mut self, ear: (f32, f32)) {
        let Some(a) = self.assault.as_ref() else { return };
        let doors = a.battle.sight.doors.iter().enumerate().fold(0u64, |h, (i, d)| h.wrapping_mul(31).wrapping_add(i as u64 * 7 + d.shut() as u64));
        let key = (ear.0.floor() as i32, ear.1.floor() as i32, doors);
        if self.audio_field.as_ref().is_some_and(|f| f.1 == key) {
            return;
        }
        self.audio_field = Some((scalia_audio::Field::build(&a.battle.sight, (key.0 as f32 + 0.5, key.1 as f32 + 0.5), AUDIO_FIELD_M), key));
    }
    /// The space at a spot (open field, room, corridor), remembered per 2 m square until the map changes.
    fn space_at(&mut self, x: f32, y: f32) -> scalia_audio::Env {
        let open = scalia_audio::Env { enclosure: 0.0, size_m: 30.0, rt60_s: 0.25, corridor: 0.0 };
        let key = ((x / 2.0).floor() as i32, (y / 2.0).floor() as i32);
        if let Some(e) = self.audio_spaces.get(&key) {
            return *e;
        }
        let env = self.assault.as_ref().map_or(open, |a| scalia_audio::listener_env(&a.battle.sight, (key.0 as f32 * 2.0 + 1.0, key.1 as f32 * 2.0 + 1.0)));
        if self.audio_spaces.len() > 20_000 {
            self.audio_spaces.clear();
        }
        self.audio_spaces.insert(key, env);
        env
    }
    /// Sounds since the last call, 27 floats each, worked out by the acoustics for the ears at the camera:
    /// kind (0 shot, 1 blast, 2 footfall), loudness (footfalls: 0 walk .. 1 sprint), delay from now (s, travel
    /// time included), gain, low-pass (Hz), reverb of its own space 0..1, position from the ears (m: right,
    /// ahead, below), distance (m), walls between, its space (enclosure 0..1, size m, corridor 0..1), then four
    /// echoes off what stands around it (extra delay s, gain against the direct sound, pan -1..1; zeros when
    /// there is nothing to bounce off), and last how distant it sounds (0 near .. 1 distant fire).
    pub fn take_sounds(&mut self) -> Vec<f32> {
        let l = self.audio_listener();
        let (lab_t, lab_speed) = (self.lab_t, self.lab_speed);
        let (r, _, _) = scalia_render::camera::basis(self.last_cam.1);
        let Some(mut a) = self.assault.take() else { return Vec::new() };
        let sounds: Vec<crate::assault::Sound> = a.sounds.drain(..).collect();
        self.assault = Some(a);
        let mut out = Vec::new();
        for snd in sounds {
            let (sx, sy) = (snd.x as f32, snd.y as f32);
            let (dx, dy) = (sx - l.0, sy - l.1);
            // Footfalls carry 60 m at most (the mix drops them earlier when anything louder is going on).
            if snd.step.is_some() && (dx * dx + dy * dy + l.2 * l.2).sqrt() > 60.0 {
                continue;
            }
            // Weapon handling is heard close by only.
            if snd.cue.quiet() && (dx * dx + dy * dy + l.2 * l.2).sqrt() > 30.0 {
                continue;
            }
            let wait = (if lab_speed <= 0.0 { 0.0 } else { (snd.at - lab_t) / lab_speed }).max(0.0) as f32 / 1000.0;
            let (kind, loud) = match (snd.blast, snd.step) {
                _ if snd.cue.kind().is_some() => (snd.cue.kind().unwrap_or(0.0), 1.0),
                (true, _) => (1.0, 1.0),
                // Footfalls: 2 on grass, 3 on concrete.
                (false, Some(l)) => (if crate::ground::concrete_at(snd.x, snd.y) { 3.0 } else { 2.0 }, l),
                _ => (0.0, 1.0),
            };
            self.sound_record(&mut out, l, r, (sx, sy), kind, loud, wait);
        }
        out
    }
    /// One footfall of the commander's, `loud` 0..1, as one record of `take_sounds`: the room he is in rings
    /// with it and nearby walls throw it back, like the soldiers' steps.
    pub fn commander_step(&mut self, loud: f32) -> Vec<f32> {
        let l = self.audio_listener();
        let (r, _, _) = scalia_render::camera::basis(self.last_cam.1);
        let kind = if crate::ground::concrete_at(self.cmd.0, self.cmd.1) { 3.0 } else { 2.0 };
        let mut out = Vec::new();
        self.sound_record(&mut out, l, r, (self.cmd.0 as f32, self.cmd.1 as f32), kind, loud, 0.0);
        // His own steps come from the middle (the camera follows him, so any offset only says where the camera
        // sits): no left, right or behind. Loudness, dullness and the room's ring still follow the camera's
        // distance, so zooming out quietens them.
        if out.len() >= 10 {
            out[2] = 0.0; // no travel delay
            // Walls between him and the point under the camera are not between him and the ears: distance only.
            let d = out[9];
            out[3] = 1.0 / (1.0 + d / 10.0);
            out[4] = (18_000.0 / (1.0 + d / 70.0)).clamp(250.0, 20_000.0);
            out[6] = 0.0; // right
            out[7] = 0.0; // ahead
            out[8] = 0.0; // below
        }
        out
    }
    /// Appends one sound record (see `take_sounds`) for a sound made at `at` (metres), heard at the listener `l`.
    #[allow(clippy::too_many_arguments)]
    fn sound_record(&mut self, out: &mut Vec<f32>, l: (f32, f32, f32), r: Vec3, at: (f32, f32), kind: f32, loud: f32, wait: f32) {
        let (sx, sy) = at;
        let (dx, dy) = (sx - l.0, sy - l.1);
        let space = self.space_at(sx, sy);
        let refl = self.reflectors_at(sx, sy);
        let dist = (dx * dx + dy * dy + l.2 * l.2).sqrt();
        // Spreading (about -6 dB a doubling past 10 m, from the ears at the camera: zooming out quietens) and
        // the air's own toll on the highs over the real distance.
        let spread = 1.0 / (1.0 + dist / 10.0);
        let air_hz = scalia_audio::air_lowpass_hz(dist);
        // What stands between: walls, shut doors, the way round through open ones, all as heard at the
        // commander's ears (inside a building the outside is muffled; zoomed out you still hear from where he
        // stands). Beyond the flooded field: the walls on the straight line, at ear height.
        let ear = self.ear();
        if !self.tablet {
            self.ensure_field(ear);
        }
        let occ = self.assault.as_ref().map_or(scalia_audio::Occlusion::NONE, |a| {
            sound_occlusion(
                &a.battle.sight,
                self.audio_field.as_ref().map(|field| &field.0),
                ear,
                (sx, sy),
                l.2,
                self.tablet,
            )
        });
        let gain = spread * occ.gain();
        let lowpass = air_hz.min(occ.lowpass_hz());
        // The space it is made in rings; through walls you hear more of that ring than of the sound itself.
        let walls = (occ.wall_db / 30.0).round();
        let wet_room = (space.enclosure * 0.6 + (occ.wall_db + occ.bend_db) / 40.0 * 0.15).clamp(0.0, 0.85);
        // Distant fire: off the screen at the closest zoom (about 12 m from him) it starts to sound far off,
        // fully so by four times that.
        let from_him = ((sx - ear.0).powi(2) + (sy - ear.1).powi(2)).sqrt();
        let far = ((from_him - FAR_FROM_M) / (FAR_FROM_M * 3.0)).clamp(0.0, 1.0);
        let echoes = scalia_audio::echoes(l, (sx, sy), &refl);
        // Right and ahead as the camera looks (ahead = up the screen, on the ground); below = the height.
        let (rx, ry) = (dx * r.x + dy * r.y, -dx * r.y + dy * r.x);
        out.extend_from_slice(&[kind, loud, wait + dist / 343.0, gain, lowpass, wet_room, rx, ry, l.2, dist, walls, space.enclosure, space.size_m, space.corridor]);
        for k in 0..4 {
            match echoes.get(k) {
                Some(e) => out.extend_from_slice(&[e.delay_s, e.gain, (e.dir.0 * r.x + e.dir.1 * r.y).clamp(-1.0, 1.0)]),
                None => out.extend_from_slice(&[0.0, 0.0, 0.0]),
            }
        }
        out.push(far);
    }
    /// Benchmark scene: `squads` squads of 12 around the commander (see `bench_army`). Returns the soldier count.
    pub fn bench_army(&mut self, squads: u32) -> u32 {
        self.world = bench_army(squads);
        self.chain = Command::default();
        self.tasks.clear();
        self.derived.clear();
        self.ground_cam = None;
        self.selected = None;
        self.selection.clear();
        self.follow_group = 0;
        self.group_pose.clear();
        self.stance.clear();
        self.order_dest = None;
        self.agents.clear();
        self.agent_prev.clear();
        self.arrive_stance.clear();
        self.arrive_report.clear();
        self.cmd = (90.0, 20.0);
        squads * 12
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
        // What was clicked decides what can be done there: a building offers entering, clearing and garrisoning;
        // open ground is a move.
        let building = killhouse().room_at(dest.0, dest.1).is_some();
        let list: &[(u8, &str)] = if captain {
            &[(10, "Take"), (11, "Hold")]
        } else if building {
            &[(20, "Move into"), (21, "Clear building"), (22, "Garrison")]
        } else {
            &[(0, "Move")]
        };
        let items: Vec<String> = list.iter().map(|(id, label)| format!("{{\"id\":{id},\"label\":\"{label}\",\"ok\":{reach},\"reason\":\"{why}\"}}")).collect();
        format!("[{}]", items.join(","))
    }

    /// Carries out an order at a map point (after the radio exchange): 0 move, 1 defend, 2 assault, 3 support.
    /// Assaults go at a sprint; on arrival a unit takes the stance that fits the order.
    pub fn issue_at(&mut self, action: u8, x: f64, y: f64, now_ms: f64) -> String {
        if action == 10 || action == 11 {
            let Some(c) = self.selected.filter(|&c| self.chain.is_captain(GroupId(c))) else { return "select a captain first".into() };
            let kind = if action == 10 { Kind::Take } else { Kind::Hold };
            return self.assign_task(c, kind, (x, y), now_ms);
        }
        let groups = if self.selection.is_empty() { self.selected.into_iter().collect::<Vec<_>>() } else { self.selection.clone() };
        // Into a building (move into or clear it): the squad joins the field and its leader plans the entry:
        // approach through dead ground, stack, entry, room by room. Nobody walks into a building cold.
        if action == 21 || action == 20 {
            let spot = grid(x, y);
            for &g in &groups {
                self.field_join(g);
                if let Some(a) = self.assault.as_mut() {
                    a.order(g as u16, scalia_ai::leader::Order::Capture);
                }
                self.arrive_report.insert(g, (spot.clone(), 2));
            }
            self.order_mode = None;
            return format!("{} {} the building", groups.len(), if action == 20 { "moving into" } else { "clearing" });
        }
        if action == 22 {
            self.order_mode = Some(scalia_ai::leader::Mode::Garrison);
        }
        let run = action == 2;
        let stance = match action {
            1 => Pose::Cover,
            2 => Pose::StandAim,
            3 => Pose::CoverAim,
            _ => Pose::Stand,
        };
        let spot = grid(x, y);
        for g in &groups {
            self.arrive_stance.insert(*g, stance);
            self.arrive_report.insert(*g, (spot.clone(), if action >= 20 { 1 } else { action }));
        }
        let msg = self.command_world(Vec3::new(x as f32, y as f32, 0.0), now_ms, run);
        self.order_mode = None;
        msg
    }

    /// How the next move is to be made, as chosen on the radio: 0 just move, 1 with caution (possible hostiles),
    /// 2 garrison the area, 3 secure the area (hostiles confirmed).
    pub fn set_order_mode(&mut self, m: u8) {
        use scalia_ai::leader::Mode;
        self.order_mode = Some(match m {
            1 => Mode::Caution,
            2 => Mode::Garrison,
            3 => Mode::Confirmed,
            _ => Mode::Plain,
        });
    }

    /// Puts a squad on the testing ground's field battle (its men where they stand), if it is not on it yet and
    /// is on the ground. Returns whether it is on the field.
    fn field_join(&mut self, i: u32) -> bool {
        if self.assault.as_ref().is_some_and(|a| a.has_squad(i as u16)) {
            return true;
        }
        let gp = self.world.groups.pos(GroupId(i));
        let on_ground = |x: i64, y: i64| (BASE_LO..BASE_HI).contains(&(x as f32 / 1000.0)) && (BASE_LO..BASE_HI).contains(&(y as f32 / 1000.0));
        if self.assault.is_none() || !on_ground(gp.x, gp.y) {
            return false;
        }
        let mut mine: Vec<&Derived> = self.derived.iter().filter(|s| s.group.0 == i).collect();
        mine.sort_by_key(|s| s.slot);
        let mut starts: Vec<(i64, i64)> = match self.agents.get(&i) {
            Some(a) => a.positions().to_vec(),
            None => mine.iter().map(|s| (s.pos.x, s.pos.y)).collect(),
        };
        if starts.is_empty() {
            // Not drawn right now: everyone at the group's place (they spread out as they go).
            let n = self.world.groups.people(GroupId(i)).max(1) as i64;
            starts = (0..n).map(|k| (gp.x + (k % 4) * 1_200, gp.y + (k / 4) * 1_200)).collect();
        }
        self.agents.remove(&i);
        if let Some(a) = self.assault.as_mut() {
            a.spawn_squad(i as u16, 0, &starts);
        }
        true
    }

    /// Names of the selected units (for the radio).
    /// The selected units' ids, as JSON.
    pub fn selection_ids(&self) -> String {
        let groups = if self.selection.is_empty() { self.selected.into_iter().collect::<Vec<_>>() } else { self.selection.clone() };
        format!("{groups:?}")
    }
    /// Replaces the selection (the radio hands each answering unit its order on its own).
    pub fn set_selection(&mut self, ids: Vec<u32>) {
        let n = self.world.groups.len() as u32;
        self.selection = ids.into_iter().filter(|&g| g < n).collect();
        self.selected = self.selection.first().copied();
    }
    /// Drops one unit from the selection (it did not answer the radio).
    pub fn deselect(&mut self, id: u32) {
        self.selection.retain(|&g| g != id);
        if self.selected == Some(id) {
            self.selected = self.selection.first().copied();
        }
    }
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
    /// Alt held: the commander walks instead of jogging.
    pub fn commander_walk(&mut self, walk: bool) {
        self.cmd_walk = walk;
    }
    /// The commander's trigger and aim (held or not), and where the mouse is (device px).
    pub fn commander_weapon(&mut self, trigger: bool, aim: bool, px: f64, py: f64) {
        // A click shorter than a frame still fires a round.
        if trigger && !self.gun.trigger && !self.gun.slung {
            self.gun.pull = true;
        }
        self.gun.trigger = trigger && !self.gun.slung;
        self.gun.aim = aim && !self.gun.slung;
        self.gun.at_px = Some((px, py));
    }
    /// Console: puts the commander at (x, y) metres on the ground (his man in the battle follows next frame).
    pub fn commander_teleport(&mut self, x: f64, y: f64) {
        self.cmd = (x, y);
        self.cmd_speed = 0.0;
    }
    /// Console: a full magazine and `mags` spares.
    pub fn commander_refill(&mut self, mags: i32) {
        let at = ((self.cmd.0 * 1000.0) as i64, (self.cmd.1 * 1000.0) as i64);
        if let Some(a) = self.assault.as_mut() {
            let i = commander_soldier(a, at);
            let s = &mut a.battle.soldiers[i];
            s.ammo = scalia_ai::battle::MAG;
            s.mags = mags.max(0);
            s.reloading_ms = 0;
        }
    }
    /// Console: back on his feet at full health.
    pub fn commander_heal(&mut self) {
        let at = ((self.cmd.0 * 1000.0) as i64, (self.cmd.1 * 1000.0) as i64);
        if let Some(a) = self.assault.as_mut() {
            let i = commander_soldier(a, at);
            a.battle.revive(i);
        }
    }
    /// Console: rounds pass through him harmlessly (true) or not; returns the new setting.
    pub fn commander_god(&mut self) -> bool {
        self.god = !self.god;
        self.god
    }
    /// Console: everyone on `team` (0 ours, 1 the enemy) drops dead; how many.
    pub fn battle_kill(&mut self, team: u8) -> u32 {
        let Some(a) = self.assault.as_mut() else { return 0 };
        let mut n = 0;
        for s in a.battle.soldiers.iter_mut().filter(|s| s.team == team && s.alive() && !s.player) {
            s.health = 0;
            n += 1;
        }
        n
    }
    /// Console: where the commander is and what he is doing.
    pub fn commander_info(&self) -> String {
        let s = self.assault.as_ref().and_then(|a| a.battle.soldiers.iter().find(|s| s.player));
        format!(
            "at ({:.1}, {:.1}) m, heading {:.0} deg, speed {:.1} m/s, pose {:?}, stance {}, health {}, rounds {} + {}{}",
            self.cmd.0, self.cmd.1, self.cmd_heading.to_degrees().rem_euclid(360.0), self.cmd_speed, self.cmd_pose.0,
            ["standing", "crouched", "prone"][self.gun.stance.min(2) as usize],
            s.map_or(100, |s| s.health.max(0)), s.map_or(30, |s| s.ammo), s.map_or(150, |s| s.mags * scalia_ai::battle::MAG),
            if self.god { ", god mode" } else { "" }
        )
    }
    /// Crouch (1) or go prone (2), or back up to standing when already so.
    pub fn commander_stance(&mut self, k: u8) {
        self.gun.stance = if self.gun.stance == k { 0 } else { k.min(2) };
    }
    /// Slings the rifle on his back, or brings it back to his hands.
    pub fn commander_holster(&mut self) -> bool {
        self.gun.slung = !self.gun.slung;
        if self.gun.slung {
            self.gun.trigger = false;
            self.gun.aim = false;
        }
        self.gun.slung
    }
    /// A fresh magazine, if he has a spare.
    pub fn commander_reload(&mut self) {
        let at = ((self.cmd.0 * 1000.0) as i64, (self.cmd.1 * 1000.0) as i64);
        if let Some(a) = self.assault.as_mut() {
            let i = commander_soldier(a, at);
            a.battle.player_reload(i);
        }
    }
    /// How wide the commander's rounds go now (0 steady .. 1 wild): moving, standing up and the hip open it;
    /// crouched or prone and still close it. The battle uses it for his shots; the page sizes the reticle by it.
    fn gun_spread(&self) -> f32 {
        (0.2 + 0.5 * (self.cmd_speed / CMD_SPRINT).min(1.0) + [0.15, 0.05, 0.0][self.gun.stance.min(2) as usize] + if self.gun.aim { 0.0 } else { 0.3 }).min(1.0)
    }
    /// The commander's weapon for the page, as JSON: rounds in the rifle and spare, reloading, slung, stance,
    /// rounds fired so far, health (0..100), and the aim line [x0, y0, x1, y1] (screen fractions) while aiming.
    pub fn commander_weapon_state(&mut self) -> String {
        let (ammo, spare, reloading, health) = match self.assault.as_ref().and_then(|a| a.battle.soldiers.iter().find(|s| s.player)) {
            Some(s) => (s.ammo, s.mags * scalia_ai::battle::MAG, s.reloading_ms > 0, s.health.max(0)),
            None => (scalia_ai::battle::MAG, COMMANDER_MAGS * scalia_ai::battle::MAG, false, 100),
        };
        let line = self.gun.line.map_or("null".to_string(), |l| format!("[{:.4},{:.4},{:.4},{:.4}]", l[0], l[1], l[2], l[3]));
        // How wide the rounds go now (0 steady .. 1 wild): moving, standing up and the hip open it; crouched or
        // prone and still close it. The page sizes the reticle by it.
        let spread = self.gun_spread();
        format!("{{\"ammo\":{ammo},\"spare\":{spare},\"reloading\":{reloading},\"slung\":{},\"stance\":{},\"shots\":{},\"health\":{health},\"spread\":{spread:.3},\"line\":{line}}}", self.gun.slung, self.gun.stance, self.gun.shots)
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
    /// Where the commander is on screen (fractions of the width and height from the top left; his chest).
    pub fn commander_screen(&mut self) -> Vec<f32> {
        let z = self.height_at(self.cmd.0 as f32, self.cmd.1 as f32) + 1.2;
        let (target, yaw, view_h) = self.last_cam;
        let (r, u, _) = scalia_render::camera::basis(yaw);
        let d = Vec3::new((self.cmd.0 - self.origin.0) as f32, (self.cmd.1 - self.origin.1) as f32, z) - target;
        let aspect = self.size.0 as f32 / self.size.1.max(1) as f32;
        vec![0.5 + d.dot(r) / (view_h * aspect), 0.5 - d.dot(u) / view_h]
    }

    pub fn commander_phase(&self) -> f32 {
        self.cmd_phase
    }
    /// The commander's gait for his footsteps: 0 standing, 1 walking, 2 sprinting.
    /// What the commander walks on: 1 concrete, 0 grass.
    pub fn commander_surface(&self) -> u8 {
        crate::ground::concrete_at(self.cmd.0, self.cmd.1) as u8
    }
    pub fn commander_gait(&self) -> u8 {
        // 0 still, 1 walking, 2 jogging, 3 sprinting.
        if !self.cmd_moving { 0 } else if self.cmd_speed > 3.8 { 3 } else if self.cmd_speed > 2.2 { 2 } else { 1 }
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
            let c = self.group_centre(i, self.world.now);
            let (cs, at) = (self.call_sign(i), grid(c.x as f64, c.y as f64));
            out.push(format!(
                "{{\"id\":{i},\"cs\":\"{cs}\",\"n\":{},\"grid\":\"{at}\",\"name\":\"{name}\",\"captain\":{captain},\"boss\":{boss},\"task\":\"{task}\",\"soldiers\":{soldiers},\"status\":\"{status}\",\"stance\":\"{stance}\",\"selected\":{}}}",
                i + 1,
                self.selection.contains(&i)
            ));
        }
        format!("[{}]", out.join(","))
    }
    /// Active interactables projected through the same orthographic camera as unit labels.
    /// The UI receives metadata, not object-specific presentation rules.
    fn active_hint_data(&self) -> Vec<crate::hints::HintData> {
        use crate::hints::{Interactable, JukeboxTarget};
        if self.scene != Scene::Ground || self.tablet {
            return Vec::new();
        }

        let player = (self.cmd.0, self.cmd.1);
        let mut hints = Vec::new();

        // Entities publish their own interaction metadata through the Interactable contract.
        if let Some((x, y)) = jukebox_mm() {
            let jukebox = JukeboxTarget { at_mm: (x, y) };
            if let Some(hint) = jukebox.interaction_hint("jukebox".into(), player) {
                hints.push(hint);
            }
        }
        if let Some(assault) = self.assault.as_ref() {
            for (index, door) in assault.battle.doors.iter().enumerate() {
                if let Some(hint) = door.interaction_hint(format!("door:{index}"), player) {
                    hints.push(hint);
                }
            }
        }

        crate::hints::prioritize(hints)
    }

    /// JSON interaction hints: id, caption, input token, projected anchor, range and display policy.
    /// Coordinates are normalized to the active canvas (0..1, y down).
    pub fn hint_marks(&mut self) -> String {
        let (target, yaw, view_h) = self.last_cam;
        let (right, up, _) = scalia_render::camera::basis(yaw);
        let aspect = self.size.0 as f32 / self.size.1.max(1) as f32;
        let origin = self.origin;
        let mut out = Vec::new();

        for hint in self.active_hint_data() {
            let x = hint.target_transform[0];
            let y = hint.target_transform[1];
            let z = self.height_at(x as f32, y as f32) + hint.target_transform[2] as f32 + hint.local_vertical_offset;
            let relative = Vec3::new((x - origin.0) as f32, (y - origin.1) as f32, z) - target;
            let sx = 0.5 + relative.dot(right) / (view_h * aspect);
            let sy = 0.5 - relative.dot(up) / view_h;
            out.push(format!(
                "{{\"id\":{},\"caption\":{},\"input\":{},\"action\":{},\"icon\":{},\"offscreenIcon\":{},\"x\":{sx:.5},\"y\":{sy:.5},\"radius\":{:.2},\"priority\":{},\"allowMultiple\":{},\"clampToScreen\":{},\"showOffscreenArrow\":{}}}",
                crate::json_str(&hint.id),
                crate::json_str(&hint.caption_text),
                crate::json_str(&hint.input_token),
                crate::json_str(&hint.action),
                crate::json_str(&hint.on_screen_icon),
                crate::json_str(&hint.off_screen_icon),
                hint.trigger_radius,
                hint.priority,
                hint.allow_multiple,
                hint.clamp_to_screen,
                hint.show_offscreen_arrow,
            ));
        }
        format!("[{}]", out.join(","))
    }

    /// Every unit as seen by the camera that is active right now (the field camera, or the tablet map's inside
    /// `use_map(true)`): JSON [{id, cs, n, x, y, r, sel, you, men: [[x, y, readiness, health]], ready: [stable, wounded,
    /// incapacitated, deceased], u}] with x, y as fractions of the canvas (0..1, y down), u a metre as a fraction of
    /// its height and r
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
        let origin = self.origin;
        let screen = |me: &mut Self, ax: f64, ay: f64| {
            let rel = Vec3::new((ax - origin.0) as f32, (ay - origin.1) as f32, me.height_at(ax as f32, ay as f32));
            let d = rel - target;
            (0.5 + d.dot(r) / (view_h * aspect), 0.5 - d.dot(u) / view_h)
        };
        let mut out = Vec::new();
        for (id, cs, n, ax, ay, rad_m, sel) in units {
            // Each man where he is drawn, with his readiness (0 stable, 1 wounded, 2 incapacitated, 3 deceased)
            // and health; the squad's place is their middle when they are known.
            let men = if id >= 0 { self.men_mm(id as u32) } else { Vec::new() };
            let (ax, ay) = if men.is_empty() {
                (ax, ay)
            } else {
                let k = men.len() as f64;
                (men.iter().map(|m| m.0).sum::<f64>() / k / 1000.0, men.iter().map(|m| m.1).sum::<f64>() / k / 1000.0)
            };
            let (x, y) = screen(self, ax, ay);
            if !(-0.1..=1.1).contains(&x) || !(-0.1..=1.1).contains(&y) { continue; }
            let mut ready = [0u32; 4];
            let mut list: Vec<String> = Vec::with_capacity(men.len());
            for &(mx, my, st, hp) in &men {
                ready[st as usize] += 1;
                let (sx, sy) = screen(self, mx / 1000.0, my / 1000.0);
                list.push(format!("[{sx:.4},{sy:.4},{st},{hp}]"));
            }
            out.push(format!(
                "{{\"id\":{id},\"cs\":\"{cs}\",\"n\":{n},\"x\":{x:.4},\"y\":{y:.4},\"r\":{:.4},\"sel\":{sel},\"you\":{},\"men\":[{}],\"ready\":[{},{},{},{}],\"u\":{:.5}}}",
                rad_m / view_h, id < 0, list.join(","), ready[0], ready[1], ready[2], ready[3], 1.0 / view_h
            ));
        }
        format!("[{}]", out.join(","))
    }

    /// A group's men where they are drawn (mm), leader first, each with his readiness (0 stable, 1 wounded,
    /// 2 incapacitated, 3 deceased) and health (0..100). Empty when the men are not drawn anywhere.
    fn men_mm(&self, g: u32) -> Vec<(f64, f64, u8, i32)> {
        if let Some(a) = self.assault.as_ref().filter(|a| a.has_squad(g as u16)) {
            return (0..a.battle.soldiers.len()).filter(|&k| a.battle.soldiers[k].squad == g as u16).map(|k| {
                let s = &a.battle.soldiers[k];
                let (x, y) = a.position(k);
                (x, y, crate::assault::readiness(s.alive(), s.down(), s.health), s.health.max(0))
            }).collect();
        }
        if let Some(a) = self.agents.get(&g) {
            return a.positions().iter().map(|p| (p.0 as f64, p.1 as f64, 0, 100)).collect();
        }
        let mut mine: Vec<&Derived> = self.derived.iter().filter(|s| s.group.0 == g).collect();
        mine.sort_by_key(|s| s.slot);
        mine.iter().map(|s| (s.pos.x as f64, s.pos.y as f64, 0, 100)).collect()
    }
    /// What is under a pixel (the active camera; the tablet map inside `use_map(true)`), as JSON:
    /// {kind: unit|commander|barracks|tower|wall|sandbags|container|crate|rock|ground, id, grid, ...}.
    pub fn object_at(&mut self, px: f64, py: f64) -> String {
        if self.scene != Scene::Ground {
            return "{\"kind\":\"none\"}".into();
        }
        let p = self.pick(px, py);
        let g = grid(p.x as f64, p.y as f64);
        if (Vec3::new(self.cmd.0 as f32, self.cmd.1 as f32, 0.0) - Vec3::new(p.x, p.y, 0.0)).length() < 1.6 {
            return format!("{{\"kind\":\"commander\",\"id\":-1,\"grid\":\"{g}\"}}");
        }
        let unit = self.unit_at(px, py);
        if unit >= 0 {
            let i = unit as u32;
            let captain = self.chain.is_captain(GroupId(i));
            return format!("{{\"kind\":\"unit\",\"id\":{i},\"cs\":\"{}\",\"n\":{},\"captain\":{captain},\"soldiers\":{},\"grid\":\"{g}\"}}",
                self.call_sign(i), i + 1, self.world.groups.people(GroupId(i)));
        }
        // Props: the nearest box the view ray passes through.
        let (o, d) = self.ray(px, py);
        let mut best: Option<(usize, f32)> = None;
        for (k, prop) in self.base_props.iter().enumerate() {
            let (lo, hi) = prop_box(prop, self.base_h);
            let (mut t0, mut t1) = (f32::MIN, f32::MAX);
            for a in 0..3 {
                if d[a].abs() < 1e-6 {
                    if o[a] < lo[a] || o[a] > hi[a] { t0 = f32::MAX; }
                    continue;
                }
                let (ta, tb) = ((lo[a] - o[a]) / d[a], (hi[a] - o[a]) / d[a]);
                t0 = t0.max(ta.min(tb));
                t1 = t1.min(ta.max(tb));
            }
            if t0 <= t1 && best.is_none_or(|b| t0 < b.1) {
                best = Some((k, t0));
            }
        }
        if let Some((k, _)) = best {
            return format!("{{\"kind\":\"{}\",\"id\":{k},\"grid\":\"{g}\"}}", prop_kind(&self.base_props[k]));
        }
        format!("{{\"kind\":\"ground\",\"id\":-1,\"grid\":\"{g}\",\"inside\":{}}}", in_base(p.x, p.y))
    }

    /// A structure's scan overlay for the active camera: its faces, doors, windows and room partitions as screen
    /// polygons (fractions of the canvas, y down), JSON {faces:[[x,y,...]], doors, windows, rooms} (rooms are lines).
    pub fn structure_overlay(&mut self, id: u32) -> String {
        let Some(prop) = self.base_props.get(id as usize).copied() else { return "{}".into() };
        let s = structure_of(&prop, self.base_h);
        let (target, yaw, view_h) = self.last_cam;
        let (r, u, _) = scalia_render::camera::basis(yaw);
        let aspect = self.size.0 as f32 / self.size.1.max(1) as f32;
        let o = Vec3::new(self.origin.0 as f32, self.origin.1 as f32, 0.0);
        let poly = |pts: &[Vec3]| {
            let v: Vec<String> = pts.iter().map(|&q| {
                let dd = q - o - target;
                format!("{:.4},{:.4}", 0.5 + dd.dot(r) / (view_h * aspect), 0.5 - dd.dot(u) / view_h)
            }).collect();
            format!("[{}]", v.join(","))
        };
        let list = |v: &[Vec<Vec3>]| v.iter().map(|q| poly(q)).collect::<Vec<_>>().join(",");
        format!("{{\"faces\":[{}],\"doors\":[{}],\"windows\":[{}],\"rooms\":[{}]}}", list(&s.faces), list(&s.doors), list(&s.windows), list(&s.rooms))
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
        let to = Vec3::new(self.cmd.0 as f32, self.cmd.1 as f32, 0.0);
        self.sys_follow = Some(HOME_BODY);
        self.fly_map_to(to);
        self.use_map(was);
    }
    /// The tablet map's camera travels fast to a point, then zooms all the way in (on the ground; from the planet
    /// or the star system it cuts straight there as before).
    fn fly_map_to(&mut self, to: Vec3) {
        let from = self.cam_smooth.or(self.ground_cam);
        match from {
            Some(f) if self.scene == Scene::Ground => {
                let f = Vec3::new(f.x, f.y, 0.0);
                let dur = (300.0 + (to - f).length() as f64 * 2.5).min(900.0);
                self.map_fly = Some((f, to, self.frame_ms, dur, 0));
                self.ground_cam = Some(to);
            }
            _ => {
                self.ground_cam = Some(to);
                self.zoom_step = 0;
            }
        }
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
            if self.map_active { self.fly_map_to(Vec3::new(c.x, c.y, 0.0)) } else { self.ground_cam = Some(c) }
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

    /// The menu's intro flight holds the camera: `view_m` metres of view height (from the star system all the
    /// way down to the commander), looking down at `elev` radians, centred on his head; `fade` dissolves him.
    pub fn set_intro(&mut self, view_m: f32, elev: f32, fade: f32) {
        if self.scene == Scene::Ships {
            return;
        }
        self.ground_cam = None;
        self.intro = Some((view_m.clamp(0.5, LADDER[LADDER.len() - 1]), elev.clamp(0.2, 1.55), fade.clamp(0.0, 1.0)));
    }
    /// The intro is over: the camera is the player's again, at the widest field zoom.
    pub fn end_intro(&mut self) {
        self.intro = None;
        self.zoom_step = STEP_FIELD_MAX;
    }

    /// Where the jukebox is ([x, y] metres; empty if there is none).
    pub fn jukebox_at(&self) -> Vec<f64> {
        jukebox_mm().map_or(Vec::new(), |(x, y)| vec![x as f64 / 1000.0, y as f64 / 1000.0])
    }
    /// How a sound made at (x, y) metres reaches the ears now: one record laid out as `take_sounds` (kind 0,
    /// loudness 1). For a sound that keeps playing (the jukebox): the page re-asks a few times a second.
    pub fn hear_at(&mut self, x: f64, y: f64) -> Vec<f32> {
        let l = self.audio_listener();
        let (r, _, _) = scalia_render::camera::basis(self.last_cam.1);
        let mut out = Vec::new();
        self.sound_record(&mut out, l, r, (x as f32, y as f32), 0.0, 1.0, 0.0);
        out
    }
    /// E, on foot: the commander uses what is in reach. Returns "jukebox" when he is at the jukebox (the page
    /// opens its player), "door" when he opened or shut the nearest door, or "" with nothing in reach.
    pub fn interact(&mut self) -> String {
        let Some(target) = self.active_hint_data().into_iter().next() else {
            return String::new();
        };

        match target.action.as_str() {
            "jukebox" => "jukebox".into(),
            "door" => {
                let Some(index) = target.id.strip_prefix("door:").and_then(|id| id.parse::<usize>().ok()) else {
                    return String::new();
                };
                let Some(assault) = self.assault.as_mut() else {
                    return String::new();
                };
                let Some(door) = assault.battle.doors.get(index) else {
                    return String::new();
                };
                use scalia_ai::doors::DoorState;
                let to = match door.state {
                    DoorState::Closed => DoorState::Open,
                    DoorState::Open => DoorState::Closed,
                    DoorState::Locked | DoorState::Breached => return String::new(),
                };
                let me_soldier = assault.battle.soldiers.iter().position(|soldier| soldier.player);
                assault.battle.set_door(index, to, me_soldier);
                "door".into()
            }
            _ => String::new(),
        }
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
    /// Temporarily bring the camera in for the commander's self-check, restoring the exact prior zoom.
    pub fn commander_inspect(&mut self, on: bool) {
        if on {
            if self.inspect_zoom.is_none() && self.scene == Scene::Ground && !self.tablet {
                self.inspect_zoom = Some(self.zoom_step);
                self.zoom_step = 1;
                self.inspect_glide = true;
            }
        } else if let Some(step) = self.inspect_zoom.take() {
            self.zoom_step = step;
            self.inspect_glide = true;
        }
    }
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
            (Scene::Ground, false) if self.intro.is_some() => {
                let (view_m, elev, _) = self.intro.unwrap_or((25.0, scalia_render::camera::ELEV, 0.0));
                let head = Vec3::new((self.cmd.0 - self.origin.0) as f32, (self.cmd.1 - self.origin.1) as f32, self.height_at(self.cmd.0 as f32, self.cmd.1 as f32) + 1.5);
                // Far out the view is centred where the game would centre it; closing in, it settles on his head.
                let w = (1.0 - ((view_m / 60.0).ln() / (2_000.0f32 / 60.0).ln()).clamp(0.0, 1.0)).powi(2);
                let c = (target + (head - target) * w, yaw, view_m, elev);
                self.last_cine = Some(c);
                (c.0, c.1, c.2, Some(c.3))
            }
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
        let desc = FrameDesc { meshes: &meshes, instanced: &instanced, decor: &decor, target, yaw, view_height, range, internal, outline: self.outline && !far_view, sky: sky_now, atmosphere, time_s, bend, elev, lights: &self.fx_lights };
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

    /// The view ray through a pixel: a point on it (absolute metres) and its direction, away from the camera.
    fn ray(&self, px: f64, py: f64) -> (Vec3, Vec3) {
        let (target, yaw, view_h) = self.last_cam;
        let (w, h) = (self.size.0 as f32, self.size.1 as f32);
        let (nx, ny) = (2.0 * px as f32 / w - 1.0, 1.0 - 2.0 * py as f32 / h);
        let (r, u, to_cam) = scalia_render::camera::basis(yaw);
        let o = Vec3::new(self.origin.0 as f32, self.origin.1 as f32, 0.0);
        (target + r * (nx * view_h / 2.0 * w / h) + u * (ny * view_h / 2.0) + o, -to_cam)
    }

    /// The barracks place of a soldier of the testing ground's barracks squads while his squad is at home (not
    /// marching, not out as agents or on the field): standing at the foot of his bunk.
    fn billeted(&self, g: u32, slot: u32) -> Option<scalia_world::Spot> {
        if self.army != 0 || g >= 2 || self.agents.contains_key(&g) || self.assault.as_ref().is_some_and(|a| a.has_squad(g as u16)) {
            return None;
        }
        let id = GroupId(g);
        let home = bay_centre(g as usize);
        let p = self.world.groups.pos(id);
        if !matches!(self.world.groups.process(id).kind, ProcessKind::Idle) || (p.x, p.y) != home {
            return None;
        }
        billet(g as usize, slot as usize)
    }

    /// Puts the barracks squads' men at their bunks (instead of in formation round the bay's middle).
    fn billet(&self, out: &mut [Derived]) {
        for d in out.iter_mut() {
            if let Some(b) = self.billeted(d.group.0, d.slot) {
                d.pos.x = b.at.0;
                d.pos.y = b.at.1;
            }
        }
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
            // Outside headquarters.
            let pos = Pos { region: RegionId(1), x: 86_000 + k * 8_000, y: 62_000, z: 0 };
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
        let id = from;
        let from = self.call_sign(from);
        self.radio_out.push(format!("{{\"id\":{id},\"n\":{},\"from\":\"{from}\",\"text\":\"{}\",\"urgent\":{urgent}}}", id + 1, text.replace('"', "'")));
    }

    /// Sends one group to a point (metres), as agents inside the testing ground, marching outside it.
    fn move_group(&mut self, i: u32, d: Vec3, speed: u32, now: WorldMs) -> bool {
        let to = Pos { region: RegionId(1), x: (d.x as f64 * 1000.0) as i64, y: (d.y as f64 * 1000.0) as i64, z: 0 };
        // Inside the testing ground the squad fights on the field battle: it routes round walls and through doors,
        // takes cover and makes the place safe when it gets there.
        if in_base(d.x, d.y) && self.field_join(i) {
            let mode = self.order_mode.unwrap_or(scalia_ai::leader::Mode::Plain);
            if let Some(a) = self.assault.as_mut() {
                a.order(i as u16, scalia_ai::leader::Order::Move { to: (to.x, to.y), mode });
            }
            return true;
        }
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
        let _g = grid(task.objective.0 as f64 / 1000.0, task.objective.1 as f64 / 1000.0);
        let report = match (step, task.at) {
            (Step::Approach, 0) | (Step::Occupy, 0) => None, // the acknowledgement covers it
            (Step::Advance, _) => Some((format!("Commander, {{me}}. At the line of departure, no contact. Advancing on the coordinates. Out."), false)),
            (Step::Suppress, _) => Some((format!("Commander, {{me}}. Contact at the coordinates! Support element laying down fire, assault going in! Out!"), true)),
            (Step::Assault, _) => Some(("Assaulting now!".to_string(), true)),
            (Step::DigIn, _) => Some((format!("Commander, {{me}}. On the objective. Digging in at the coordinates. Out."), false)),
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
                let _g = grid(task.objective.0 as f64 / 1000.0, task.objective.1 as f64 / 1000.0);
                let me = self.call_sign(c);
                let text = match task.kind {
                    Kind::Take => format!("Commander, {me}. Objective taken, no casualties. Squads holding at the coordinates. Over."),
                    Kind::Hold => format!("Commander, {me}. In position, holding the coordinates. Out."),
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
                (Pose::Walk, 1.5 / scalia_render::pose::WALK_STRIDE), (Pose::Run, 3.0 / scalia_render::pose::RUN_STRIDE), (Pose::Stand, 0.0), (Pose::Crouch, 0.0),
                (Pose::CoverAim, 0.0), (Pose::Prone, 0.0), (Pose::StandAim, 0.0), (Pose::Sprint, 4.6 / scalia_render::pose::SPRINT_STRIDE),
                (Pose::Cover, 0.0),
            ];
            // Entry 9 is "walk while aiming": the walk with an aiming upper body layered on top.
            // Previews past the pose list (legs pose, cycles a second, upper body layered on top, moving direction):
            // walking and strafing while aiming, crouched walk, cover, close-quarters moves and actions, and two
            // combinations the AI will use (checking a corner: crouched sidestep with the aim on top; dynamic entry:
            // combat-run legs under the aim).
            use std::f32::consts::{FRAC_PI_2, PI};
            use scalia_render::pose::stride_m_dir;
            let walk = |dir: f32| 1.2 / stride_m_dir(Pose::Walk, dir);
            let combo: Option<(Pose, f32, Option<Pose>, f32)> = match self.preview_pose {
                Some(9) => Some((Pose::Walk, walk(0.0), Some(Pose::StandAim), 0.0)),
                Some(10) => Some((Pose::Walk, walk(FRAC_PI_2), Some(Pose::StandAim), FRAC_PI_2)),
                Some(11) => Some((Pose::Walk, walk(-FRAC_PI_2), Some(Pose::StandAim), -FRAC_PI_2)),
                Some(12) => Some((Pose::Walk, walk(PI), Some(Pose::StandAim), PI)),
                Some(13) => Some((Pose::CrouchWalk, 1.0 / stride_m(Pose::CrouchWalk), None, 0.0)),
                Some(14) => Some((Pose::LeanLeft, 0.0, None, 0.0)),
                Some(15) => Some((Pose::LeanRight, 0.0, None, 0.0)),
                Some(16) => Some((Pose::CrouchAim, 0.0, None, 0.0)),
                Some(17) => Some((Pose::Stack, 0.0, None, 0.0)),
                Some(18) => Some((Pose::Sweep, 0.3, None, 0.0)),
                Some(19) => Some((Pose::Throw, 0.6, None, 0.0)),
                Some(20) => Some((Pose::Reload, 0.4, None, 0.0)),
                Some(21) => Some((Pose::Signal, 0.5, None, 0.0)),
                Some(22) => Some((Pose::Drone, 0.4, None, 0.0)),
                Some(23) => Some((Pose::CrouchWalk, 0.6 / stride_m_dir(Pose::CrouchWalk, -FRAC_PI_2), Some(Pose::StandAim), -FRAC_PI_2)),
                Some(24) => Some((Pose::Run, 3.0 / stride_m(Pose::Run), Some(Pose::StandAim), 0.0)),
                Some(25) => Some((Pose::Wounded, 0.3, None, 0.0)),
                Some(26) => Some((Pose::CrouchWalk, 0.8 / stride_m_dir(Pose::CrouchWalk, PI), Some(Pose::Drag), PI)),
                Some(27) => Some((Pose::Aid, 1.1, None, 0.0)),
                Some(28) => Some((Pose::Dead, 0.0, None, 0.0)),
                Some(29) => Some((Pose::FallBack, 0.9, None, 0.0)),
                Some(30) => Some((Pose::FallForward, 1.0, None, 0.0)),
                Some(31) => Some((Pose::DeadBack, 0.0, None, 0.0)),
                _ => None,
            };
            let (pose, steps, upper, dir) = combo.unwrap_or_else(|| {
                let (p, st) = SHOW[self.preview_pose.unwrap_or((t / 3000.0) as usize) % SHOW.len()];
                (p, st, None, 0.0)
            });
            let ts = ((t / 1000.0) % 86_400.0) as f32;
            let prev = if self.preview_pose.is_none() { SHOW[((t / 3000.0) as usize + SHOW.len() - 1) % SHOW.len()].0 } else { pose };
            let changed = ((t / 3000.0).floor() * 3.0) as f32 % 86_400.0;
            let colour = VARIANTS[self.model_variant as usize % VARIANTS.len()].1;
            self.gpu.upload_instances("figure1", bytes(&[0.0, 0.0, 0.0, colour, 0.0, 0.0, steps, pose as u32 as f32, prev as u32 as f32, changed.min(ts), scalia_render::pose::pack_layer(upper, dir), 0.0]));
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
            self.billet(&mut out);
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
            if self.agents.contains_key(&d.group.0) || self.assault.as_ref().is_some_and(|a| a.has_squad(d.group.0 as u16)) {
                continue; // drawn from their agents below
            }
            let (ax, ay) = (d.pos.x as f64 / 1000.0, d.pos.y as f64 / 1000.0);
            let z = self.height_at(ax as f32, ay as f32);
            let (x, y) = ((ax - self.origin.0) as f32, (ay - self.origin.1) as f32);
            if !self.on_screen(x, y, z, 3.0) {
                continue;
            }
            // Facing: along the march, at his bunk the aisle, else the formation's front with a little personal
            // variation.
            let jitter = (scalia_core::hash::mix64(d.slot as u64 ^ 0x5ca1) % 1000) as f32 / 1000.0;
            let heading = match self.world.groups.process(d.group).kind {
                ProcessKind::March { from, to, .. } => ((from.x - to.x) as f32).atan2((to.y - from.y) as f32),
                _ => match self.billeted(d.group.0, d.slot) {
                    Some(b) => (-(b.face.0 as f32)).atan2(b.face.1 as f32) + (jitter - 0.5) * 0.25,
                    None => (jitter - 0.5) * 0.4,
                },
            };
            let walking = d.phase == Phase::Marching || d.phase == Phase::Walking || d.phase == Phase::Climbing;
            let running = walking && matches!(self.world.groups.process(d.group).kind, ProcessKind::March { speed_mm_s, .. } if speed_mm_s >= RUN_SPEED_MM_S);
            let march = match self.world.groups.process(d.group).kind { ProcessKind::March { speed_mm_s, .. } => speed_mm_s as f32 / 1000.0, _ => MARCH_SPEED_MM_S as f32 / 1000.0 };
            let (pose, steps) = if running {
                (Pose::Sprint, march / stride_m(Pose::Sprint))
            } else if walking {
                (Pose::Walk, march / stride_m(Pose::Walk))
            } else {
                (self.stance.get(&d.group.0).copied().unwrap_or(Pose::Stand), 0.0)
            };
            // Selected: outlined in his readiness colour (+64 per level: the shader's selection tag), not recoloured.
            let body = if self.chain.is_captain(d.group) { 4.0 } else if walking { 7.0 } else { 6.0 } + if self.selection.contains(&d.group.0) { SEL_STABLE } else { 0.0 };
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
        // Roofs over the training house and the barracks, until the camera (or the commander) goes in.
        {
            let house = self.assault.as_ref().and_then(|a| a.battle.house()).map(|k| (k.origin, ((k.size_m.0 * 1000.0) as i64, (k.size_m.1 * 1000.0) as i64)));
            let (cx, cy) = (self.last_cam.0.x as f64 + self.origin.0, self.last_cam.0.y as f64 + self.origin.1);
            let within = |r: (i64, i64, i64, i64), x: f64, y: f64| { let (x, y) = ((x * 1000.0) as i64, (y * 1000.0) as i64); x > r.0 - 1_000 && x < r.2 + 1_000 && y > r.1 - 1_000 && y < r.3 + 1_000 };
            let inside = |r: (i64, i64, i64, i64)| within(r, cx, cy) || within(r, self.cmd.0, self.cmd.1);
            let house_r = house.map(|(o, s)| (o.0, o.1, o.0 + s.0, o.1 + s.1));
            let barracks = barracks_mm();
            let key = (house_r.map_or(0, |r| r.0 ^ (r.1 << 20)), house_r.map_or(0, |r| r.2), house_r.map_or(0, |r| r.3), house_r.is_some_and(|r| !inside(r)), !inside(barracks));
            if key != self.roof_key {
                self.roof_key = key;
                let mut m = MeshBuilder::default();
                let mut roof = |r: (i64, i64, i64, i64), top: f32| {
                    let f = |v: i64| v as f32 / 1000.0;
                    m.cuboid([f(r.0) - 0.25, f(r.1) - 0.25, self.base_h + top], [f(r.2) + 0.25, f(r.3) + 0.25, self.base_h + top + 0.12], scalia_render::mesh::code(2, scalia_render::mesh::Part::Body));
                };
                if let (true, Some(r)) = (key.3, house_r) {
                    roof(r, scalia_core::killhouse::WALL_TOP_M);
                }
                if key.4 {
                    roof(barracks, 2.78);
                }
                if key.3 || key.4 {
                    self.gpu.upload("roofs", m.bytes());
                }
            }
            if self.roof_key.3 || self.roof_key.4 {
                instanced.push(("roofs".into(), "base_at".into()));
            }
        }
        if let Some(a) = self.assault.as_ref() {
            let now = self.frame_ms;
            self.door_seen.resize(a.battle.doors.len(), (0, 0.0));
            for (seen, d) in self.door_seen.iter_mut().zip(&a.battle.doors) {
                if seen.0 != d.changed_ms {
                    *seen = (d.changed_ms, now);
                }
            }
            let seen = &self.door_seen;
            let m = door_mesh(&a.battle.doors, &self.door_steel, self.base_h, |i| now - seen[i].1);
            self.gpu.upload("doors", m.bytes());
            instanced.push(("doors".into(), "base_at".into()));
        }
        let (bx, by) = ((-self.origin.0) as f32, (-self.origin.1) as f32);
        self.gpu.upload_instances("base_at", bytes(&[bx, by, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1000.0, 0.0, 0.0]));
        // The enemy squad in the training house: on their posts, or fighting the live assault.
        let mut foes = Vec::with_capacity(ENEMY_POSTS.len() * 12);
        let mut assaulters = Vec::new();
        if let Some(mut a) = self.assault.take() {
            // Battle time runs on the lab's clock (real time unless the AI lab slows or pauses it).
            self.lab_t += (self.frame_ms - self.lab_prev).clamp(0.0, 500.0) * self.lab_speed;
            self.lab_prev = self.frame_ms;
            a.advance(self.lab_t);
            if let Some((sq, k, hp)) = self.lab_wound {
                if let Some(i) = a.battle.soldiers.iter().enumerate().filter(|(_, s)| s.squad == sq).map(|(i, _)| i).nth(k) {
                    a.battle.soldiers[i].health = hp;
                    self.lab_wound = None;
                }
            }
            // Sandbags laid on the field: part of the base from now on (drawn, and in the agents' grid).
            if let Some(props) = a.battle.map_spec().map(|m| m.props()).filter(|p| p.len() > self.base_props.len()) {
                self.base_props = props;
                self.base_grid.rebuild(&self.base_props);
                self.audio_spaces.clear();
                self.audio_field = None;
                self.audio_reflectors.clear();
                let m = base_mesh(&self.base_props, self.base_h);
                self.gpu.upload("base", m.bytes());
            }
            let ts = ((t / 1000.0) % 86_400.0) as f32;
            let origin = self.origin;
            // Our squads keep their roster colours: selected, captain, or plain.
            let colours: Vec<(u16, f32)> = (0..self.world.groups.len() as u32).map(|g| (g as u16, if self.chain.is_captain(GroupId(g)) { 4.0 } else { 6.0 })).collect();
            // Selected squads are outlined man by man in their readiness colour (the battle knows who is hurt).
            a.selected = self.selection.iter().map(|&g| g as u16).collect();
            let mut place = |x: f64, y: f64| {
                let z = self.height_at(x as f32, y as f32);
                let (rx, ry) = ((x - origin.0) as f32, (y - origin.1) as f32);
                self.on_screen(rx, ry, z, 3.0).then_some((rx, ry, z))
            };
            foes = a.instances(1, ts, |_| ENEMY_RED as f32, &mut place);
            assaulters = a.instances(0, ts, |sq| colours.iter().find(|c| c.0 == sq).map_or(6.0, |c| c.1), &mut place);
            let clouds = a.battle.smoke.iter().filter(|c| a.battle.time_ms >= c.at_ms).map(|c| {
                let (x, y) = (c.pos.0 as f64 / 1000.0, c.pos.1 as f64 / 1000.0);
                ([x, y, self.height_at(x as f32, y as f32) as f64], (a.battle.time_ms - c.at_ms) as f64, c.at_ms as u64 ^ (c.pos.0 as u64) << 20)
            }).collect();
            self.fx.set_smoke(clouds);
            for f in a.fx.drain(..) {
                let ground = |me: &mut Self, p: (i64, i64, i64)| {
                    let (x, y) = (p.0 as f64 / 1000.0, p.1 as f64 / 1000.0);
                    [x, y, me.height_at(x as f32, y as f32) as f64 + p.2 as f64 / 1000.0]
                };
                let (from, to) = (ground(self, f.from), ground(self, f.to));
                if f.blast {
                    self.fx.blast(self.lab_real(f.at), from);
                } else {
                    self.fx.shot(self.lab_real(f.at), from, to, f.bore.map(|b| [b[0] as f64, b[1] as f64, b[2] as f64]), f.hit, f.tracer);
                }
            }
            self.assault = Some(a);
        }
        // Flashes, tracers and blasts: glowing geometry (no shadow) and this frame's point lights.
        self.fx.prune(self.frame_ms);
        self.fx_lights = self.fx.lights(self.frame_ms, self.origin);
        if !self.fx.is_empty() {
            self.gpu.upload("fx", self.fx.mesh(self.frame_ms, self.origin).bytes());
            self.gpu.upload_instances("fx_at", bytes(&[0.0f32, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1000.0, 0.0, 0.0]));
            decor.push(("fx".into(), "fx_at".into()));
        }
        let (kx, ky) = (killhouse_mm().0 as f32 / 1000.0, killhouse_mm().1 as f32 / 1000.0);
        let posts: &[((f32, f32), (f32, f32), Pose)] = if self.assault.is_none() { &ENEMY_POSTS } else { &[] };
        for (i, &((px, py), (wx, wy), pose)) in posts.iter().enumerate() {
            let (ax, ay) = (kx + px, ky + py);
            let z = self.height_at(ax, ay);
            let heading = (px - wx).atan2(wy - py);
            let (x, y) = ((ax as f64 - self.origin.0) as f32, (ay as f64 - self.origin.1) as f32);
            foes.extend_from_slice(&[x, y, z, ENEMY_RED as f32, heading, i as f32 * 0.137 % 1.0, 0.0, pose as u32 as f32, pose as u32 as f32, -1000.0, 0.0, 0.0]);
        }
        self.gpu.upload_instances("enemies", bytes(&foes));
        self.gpu.upload_instances("assaulters", bytes(&assaulters));
        // Three levels of detail. Past 250 m a soldier is a few pixels tall: the 24-triangle figure, and no
        // shadow pass for it (its shadow would be smaller than a pixel).
        if self.ground_view <= 70.0 {
            instanced.push(("figure".into(), "soldiers".into()));
            instanced.push(("enemy".into(), "enemies".into()));
            instanced.push(("figure".into(), "assaulters".into()));
        } else if self.ground_view <= 250.0 {
            instanced.push(("figure_lo".into(), "soldiers".into()));
            instanced.push(("enemy_lo".into(), "enemies".into()));
            instanced.push(("figure_lo".into(), "assaulters".into()));
        } else {
            decor.push(("figure_far".into(), "soldiers".into()));
            decor.push(("enemy_far".into(), "enemies".into()));
            decor.push(("figure_far".into(), "assaulters".into()));
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
        let mut smooth = match self.cam_smooth {
            Some(c) if (c - target).length() < self.ground_view * 3.0 => c + (target - c) * k,
            _ => target, // a jump (new army, far pan): cut straight there
        };
        // The tablet map flying to a unit or the commander: fast and eased, then the zoom glides all the way in.
        if self.map_active {
            if let Some((from, to, start, dur, zoom)) = self.map_fly {
                let p = ((t - start) / dur).clamp(0.0, 1.0) as f32;
                let e = p * p * p * (p * (p * 6.0 - 15.0) + 10.0);
                let at = from + (to - from) * e;
                smooth = Vec3::new(at.x, at.y, target.z);
                if p >= 1.0 {
                    self.map_fly = None;
                    self.zoom_step = zoom;
                }
            }
        }
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
        self.assault = Some(self.new_field());
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
            // Squads on the field battle: their group record follows them; arrival (secured) reports.
            if let Some(a) = self.assault.as_mut() {
                for sq in a.done.drain(..).collect::<Vec<u16>>() {
                    let g = sq as u32;
                    if let Some(p) = self.arrive_stance.remove(&g) {
                        self.stance.insert(g, p);
                    }
                    if let Some((spot, kind)) = self.arrive_report.remove(&g) {
                        reports.push((g, spot, kind));
                    }
                }
                let n = self.world.groups.len() as u32;
                for g in 0..n {
                    if let Some((cx, cy)) = a.has_squad(g as u16).then(|| a.centre(g as u16)).flatten() {
                        self.world.groups.set_process(GroupId(g), scalia_core::process::Process::idle());
                        self.world.groups.set_pos(GroupId(g), Pos { region: RegionId(1), x: cx, y: cy, z: 0 });
                    }
                }
            }
            for (g, _spot, kind) in reports {
                let me = self.call_sign(g);
                let pick = (g as usize + self.radio_out.len() + (t as usize / 50)) % 3;
                let text = match (kind, pick) {
                    (1, 0) => format!("Commander, {me}. Dug in at the coordinates, holding. Over."),
                    (1, 1) => format!("Commander, {me}. Coordinates secured, we hold here. Out."),
                    (1, _) => format!("Commander, {me}. In cover at the coordinates, watching our front. Over."),
                    (2, 0) => format!("Commander, {me}. Assault element at the coordinates. Awaiting orders. Over."),
                    (2, 1) => format!("Commander, {me}. Reached the coordinates, area clear. Over."),
                    (2, _) => format!("Commander, {me}. At the objective, ready. Out."),
                    (3, 0) => format!("Commander, {me}. Support set at the coordinates. Over."),
                    (3, 1) => format!("Commander, {me}. In position at the coordinates, eyes on. Out."),
                    (3, _) => format!("Commander, {me}. Overwatch established at the coordinates. Over."),
                    (_, 0) => format!("Commander, {me}. Arrived at the coordinates. Over."),
                    (_, 1) => format!("Commander, {me}. We are at the coordinates, awaiting orders. Over."),
                    (_, _) => format!("Commander, {me}. At position, at the coordinates. Out."),
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
                let mps = self.agents[&g].speed_mm_s() as f32 / 1000.0;
                let (pose, steps) = if moving && sprint { (Pose::Sprint, mps / stride_m(Pose::Sprint)) } else if moving { (Pose::Walk, mps / stride_m(Pose::Walk)) } else { (stance, 0.0) };
                let (ix, iy) = (px as f64 + (x - px) as f64 * alpha, py as f64 + (y - py) as f64 * alpha);
                let (ax, ay) = (ix / 1000.0, iy / 1000.0);
                let z = self.height_at(ax as f32, ay as f32);
                let (rx, ry) = ((ax - self.origin.0) as f32, (ay - self.origin.1) as f32);
                if !self.on_screen(rx, ry, z, 3.0) {
                    continue;
                }
                let body = if self.chain.is_captain(GroupId(g)) { 4.0 } else if moving { 7.0 } else { 6.0 } + if self.selection.contains(&g) { SEL_STABLE } else { 0.0 };
                let entry = self.group_pose.entry(1_000_000 + key).or_insert((pose, pose, -1000.0));
                if entry.0 != pose {
                    *entry = (pose, entry.0, ts);
                }
                let (prev_pose, since) = (entry.1, entry.2);
                let aiming = matches!(stance, Pose::StandAim | Pose::CoverAim);
                let upper = if moving && pose == Pose::Walk && aiming { Pose::StandAim as u32 as f32 + 1.0 } else { 0.0 };
                let lean = self.bank_for(1u64 << 40 | key as u64, heading, if moving { mps } else { 0.0 });
                inst.extend_from_slice(&[rx, ry, z, body + lean, heading, jitter, steps, pose as u32 as f32, prev_pose as u32 as f32, since, upper, 0.0]);
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
        // Where the rifle points: the ground under the mouse.
        if let Some((px, py)) = self.gun.at_px.filter(|_| !holding) {
            let p = self.pick(px, py);
            let (dx, dy) = (p.x - self.cmd.0 as f32, p.y - self.cmd.1 as f32);
            if dx * dx + dy * dy > 0.25 {
                self.gun.toward = Some((dx, dy));
            }
        }
        if self.god {
            if let Some(s) = self.assault.as_mut().and_then(|a| a.battle.soldiers.iter_mut().find(|s| s.player)) {
                s.health = 100;
            }
        }
        let ready = (self.gun.aim || self.gun.trigger) && !self.gun.slung && !holding && self.gun.toward.is_some();
        // His man on the field battle: shot down or killed, he no longer moves or fires.
        let (down, dead) = self.assault.as_ref().and_then(|a| a.battle.soldiers.iter().find(|s| s.player)).map_or((false, false), |s| (s.down(), !s.alive()));
        let wants = len > 0.01 && t >= self.radio_until && !holding && !down && !dead;
        // Like a person, not a cursor: about half a second up to a walk, a second up to a sprint, and a few
        // steps to brake (GTA-style). Changing direction keeps most of the pace. Low he goes slower; on his
        // belly he crawls; with the rifle up he walks, never sprints.
        // Shift from a crouch or kneel: he gets up and drives off into a sprint (building up, not at once).
        if sprint && wants && self.gun.stance != 0 && !ready {
            self.gun.stance = 0;
        }
        let pace = match self.gun.stance {
            2 => CMD_CRAWL,
            1 => CMD_CROUCH,
            _ if self.gun.aim && ready => CMD_AIM_WALK,
            _ if sprint && !ready => CMD_SPRINT,
            _ if self.cmd_walk || ready => CMD_WALK,
            _ => CMD_JOG,
        };
        let target = if wants { pace } else { 0.0 };
        // Momentum (motion.rs): his feet can only push, brake and turn him so fast, less so the faster he goes. A
        // sprint bends round in an arc; reversing (A then D) is a skid to a stop, then a push off the other way.
        let want = if wants { (vx / len * target, vy / len * target) } else { (0.0, 0.0) };
        let vel = (self.cmd_dir.0 * self.cmd_speed, self.cmd_dir.1 * self.cmd_speed);
        self.cmd_skid = crate::motion::skidding(vel, want);
        let vel = crate::motion::step_velocity(vel, want, dt, CMD_FEET);
        let was_still = self.cmd_speed < 0.05;
        self.cmd_speed = (vel.0 * vel.0 + vel.1 * vel.1).sqrt();
        if self.cmd_speed > 1e-3 {
            self.cmd_dir = (vel.0 / self.cmd_speed, vel.1 / self.cmd_speed);
        } else if wants {
            self.cmd_dir = (vx / len, vy / len);
        }
        // Setting off from still, he steps out with a foot straight away (not from the middle of a stride).
        if was_still && self.cmd_speed >= 0.05 {
            self.cmd_gait = 0.12;
        }
        self.cmd_moving = self.cmd_speed > 0.05;
        if self.cmd_moving {
            vx = self.cmd_dir.0 * self.cmd_speed * dt;
            vy = self.cmd_dir.1 * self.cmd_speed * dt;
            let (nx, ny) = (self.cmd.0 + vx as f64, self.cmd.1 + vy as f64);
            // Against the props themselves (not the coarse nav grid, whose half-metre cells close a 1.6 m door to
            // a man of his width): free when no prop's footprint comes within his body's radius.
            // Shut doors are in his way too; walking into a closed (not locked) one, he pushes it open.
            let shut: Vec<(usize, Prop, bool)> = self.assault.as_ref().map(|a| a.battle.doors.iter().enumerate().filter(|(_, d)| d.shut()).map(|(i, d)| (i, d.as_prop(), d.state == scalia_ai::doors::DoorState::Locked)).collect()).unwrap_or_default();
            let door_props: Vec<Prop> = shut.iter().map(|d| d.1).collect();
            let mm = |v: f64| (v * 1000.0) as i64;
            let blocking_door = shut.iter().find(|d| !body_clear(std::slice::from_ref(&d.1), mm(nx), mm(ny), CMD_RADIUS_MM)).map(|d| (d.0, d.2));
            if let (Some((door, false)), Some(a)) = (blocking_door, self.assault.as_mut()) {
                a.battle.set_door(door, scalia_ai::doors::DoorState::Open, None);
            }
            let free = |x: f64, y: f64| !in_base(x as f32, y as f32) || body_clear(&self.base_props, mm(x), mm(y), CMD_RADIUS_MM) && body_clear(&door_props, mm(x), mm(y), CMD_RADIUS_MM);
            if free(nx, ny) {
                self.cmd = (nx, ny);
            } else if free(nx, self.cmd.1) {
                self.cmd.0 = nx;
            } else if free(self.cmd.0, ny) {
                self.cmd.1 = ny;
            } else {
                self.cmd_speed = 0.0;
            }
        }
        // How he looks doing it comes from the animation library (anim.rs), the same one every soldier uses: this
        // only says what he is doing.
        let travel = (-self.cmd_dir.0).atan2(self.cmd_dir.1);
        let reloading = self.assault.as_ref().and_then(|a| a.battle.soldiers.iter().find(|s| s.player)).is_some_and(|s| s.reloading_ms > 0);
        let mut d = crate::anim::Drive::new(dt, ts);
        d.vel = (self.cmd_dir.0 * self.cmd_speed, self.cmd_dir.1 * self.cmd_speed);
        d.want = Some(want);
        // A body turns with weight: towards where the rifle points while it is up (keeping pace with the cursor),
        // pivoting round in a cut, else the way he goes (less sharply the faster he runs).
        d.face = match self.gun.toward.filter(|_| ready) {
            Some((tx, ty)) => crate::anim::Face::Aim { heading: (-tx).atan2(ty) },
            None if self.cmd_skid == crate::motion::Skid::Cut && len > 0.01 => crate::anim::Face::Turn { heading: (-vx).atan2(vy), top: 8.0 },
            None if self.cmd_moving => crate::anim::Face::Turn { heading: travel, top: 6.0 - 2.0 * (self.cmd_speed / CMD_SPRINT).min(1.0) },
            None => crate::anim::Face::Keep,
        };
        d.stance = match self.gun.stance { 0 => crate::anim::Stance::Stand, 1 => crate::anim::Stance::Crouch, _ => crate::anim::Stance::Prone };
        d.rifle = if reloading { crate::anim::Rifle::Reload } else if ready { crate::anim::Rifle::Aim } else if self.gun.slung { crate::anim::Rifle::Slung } else { crate::anim::Rifle::Ready };
        d.sprint = !ready;
        d.down = down;
        d.dead = dead;
        d.action = if holding { Some((Pose::Tablet, 0.0, None)) } else if t < self.radio_until { Some((Pose::Radio, 0.0, None)) } else { None };
        let f = self.cmd_fig.step(&d);
        self.cmd_heading = f.heading;
        self.cmd_spin = self.cmd_fig.spin;
        self.cmd_pose.0 = f.pose;
        let pose = f.pose;
        // His gait phase (exactly what the shader is handed), for the footsteps.
        self.cmd_phase = if f.cadence < 0.0 && f.speed > 0.05 || f.footfall.is_some() { f.phase } else { -1.0 };
        self.commander_fight(ready, pose);
        let z = self.height_at(self.cmd.0 as f32, self.cmd.1 as f32);
        let (rx, ry) = ((self.cmd.0 - self.origin.0) as f32, (self.cmd.1 - self.origin.1) as f32);
        let (kick, flinch) = self.assault.as_ref().and_then(|a| a.battle.soldiers.iter().position(|s| s.player).map(|i| (a.kick(i), a.flinch(i)))).unwrap_or((0.0, 0.0));
        let rec = self.cmd_fig.record(&f, (rx, ry, z), 9.0, kick, flinch, 0.0);
        // His muzzle as drawn, so his rounds leave from the end of his barrel.
        let (tip, bore) = self.cmd_fig.barrel(&f, ((self.cmd.0 * 1000.0) as f32, (self.cmd.1 * 1000.0) as f32));
        let gun = tip.filter(|_| !self.gun.slung).map(|w| (w[0] as i64, w[1] as i64, w[2] as i64));
        if let Some(a) = self.assault.as_mut() {
            if let Some(i) = a.battle.soldiers.iter().position(|s| s.player) {
                a.set_muzzle(i, gun, bore);
            }
        }
        // The diving camera passes through his head on its way to the tablet: he dissolves as it gets close.
        self.cmd_rec = rec;
        let me = self.cmd_fig.record(&f, (rx, ry, z), 9.0, kick, flinch, self.cine_fade(self.frame_ms));
        self.gpu.upload_instances("commander_at", bytes(&me));
        instanced.push(("commander".into(), "commander_at".into()));
        // The tablet appears in his hands once they have come up to hold it.
        // The tablet itself is the page's close-up, which slides up once the camera is in place: the 3D model
        // is not drawn (it would show behind the close-up whenever that moves).
    }

    /// The commander on the field battle: his man there follows him (place, facing, stance, pace); with the
    /// trigger held and the rifle up he fires where it points, on the battle's rules; while aiming, the line his
    /// rounds would take is worked out for the page (to the first solid thing, 120 m at most).
    fn commander_fight(&mut self, ready: bool, pose: Pose) {
        let at = ((self.cmd.0 * 1000.0) as i64, (self.cmd.1 * 1000.0) as i64);
        let stance = match self.gun.stance { 0 => scalia_ai::battle::Stance::Stand, 1 => scalia_ai::battle::Stance::Crouch, _ => scalia_ai::battle::Stance::Prone };
        let (h, pace) = (self.cmd_heading, (self.cmd_speed * 1000.0) as i64);
        // The rifle points the way he faces, not straight at the mouse: while he swings round, his rounds (and the
        // reticle) lag behind the cursor, as in Helldivers. The mouse only says how far out to look.
        let mouse_m = self.gun.toward.map_or(30.0, |(x, y)| (x * x + y * y).sqrt());
        let toward = self.gun.toward.map(|_| (-h.sin(), h.cos()));
        let mut reach = 120.0f32;
        if let Some(mut a) = self.assault.take() {
            let i = commander_soldier(&mut a, at);
            a.battle.player_set(i, at, ((-h.sin() * 1000.0) as i64, (h.cos() * 1000.0) as i64), stance, pace);
            if let Some((tx, ty)) = toward {
                if (self.gun.trigger || self.gun.pull) && (ready || self.gun.pull) && !self.gun.slung && pose != Pose::Sprint {
                    match a.battle.player_fire(i, ((tx * 1000.0) as i64, (ty * 1000.0) as i64), (self.gun_spread() * 1000.0) as i64) {
                        scalia_ai::battle::PlayerShot::Fired { .. } => {
                            self.gun.shots += 1;
                            self.gun.pull = false;
                        }
                        scalia_ai::battle::PlayerShot::Cycling => {}
                        // Out of rounds: the click of an empty rifle (once per pull).
                        scalia_ai::battle::PlayerShot::Empty => {
                            if self.frame_ms - self.gun.dry_at > 350.0 {
                                self.gun.dry_at = self.frame_ms;
                                a.cue(i, crate::assault::Cue::Dry);
                            }
                            self.gun.pull = false;
                        }
                        _ => self.gun.pull = false,
                    }
                }
                let l = (tx * tx + ty * ty).sqrt().max(1e-3);
                let eye = stance.eye();
                let mut d = 0.5;
                while d < reach && !a.battle.sight.solid_at(at.0 + (tx / l * d * 1000.0) as i64, at.1 + (ty / l * d * 1000.0) as i64, eye) {
                    d += 0.5;
                }
                reach = d;
            }
            self.assault = Some(a);
        }
        self.gun.line = match toward.filter(|_| self.gun.aim && ready) {
            Some((tx, ty)) => {
                let l = (tx * tx + ty * ty).sqrt().max(1e-3);
                let hgt = match self.gun.stance { 0 => 1.45, 1 => 0.95, _ => 0.3 };
                let a0 = (self.cmd.0 + (tx / l * 0.7) as f64, self.cmd.1 + (ty / l * 0.7) as f64);
                let out = reach.min(mouse_m.max(2.0));
                let a1 = (self.cmd.0 + (tx / l * out) as f64, self.cmd.1 + (ty / l * out) as f64);
                // From the end of his barrel (as drawn last frame) when known.
                let barrel = self.assault.as_ref().and_then(|a| a.battle.soldiers.iter().position(|s| s.player).and_then(|i| a.muzzle_of(i)));
                let p0 = match barrel {
                    Some((mx, my, mz)) => self.screen_of((mx as f64 / 1000.0, my as f64 / 1000.0), mz as f32 / 1000.0),
                    None => self.screen_of(a0, hgt),
                };
                let p1 = self.screen_of(a1, hgt);
                Some([p0.0, p0.1, p1.0, p1.1])
            }
            None => None,
        };
    }

    /// The packed bank (pose.rs `pack_bank`) for figure `key` facing `heading` at `speed` m/s: how hard he is
    /// turning is followed from frame to frame (smoothed, so one jerk of the heading does not tip him).
    fn bank_for(&mut self, key: u64, heading: f32, speed: f32) -> f32 {
        let now = self.frame_ms;
        let e = self.turns.entry(key).or_insert((heading, 0.0, now, speed, 0.0));
        let dt = ((now - e.2) / 1000.0) as f32;
        if dt > 0.0005 {
            let mut d = heading - e.0;
            while d > std::f32::consts::PI { d -= std::f32::consts::TAU; }
            while d < -std::f32::consts::PI { d += std::f32::consts::TAU; }
            let fresh = dt < 0.25;
            let rate = if fresh { d / dt } else { 0.0 };
            let accel = if fresh { (speed - e.3) / dt } else { 0.0 };
            e.1 += (rate - e.1) * (dt * 6.0).min(1.0);
            e.4 += (accel - e.4) * (dt * 8.0).min(1.0);
            *e = (heading, e.1, now, speed, e.4);
        }
        scalia_render::pose::pack_tilt(scalia_render::pose::bank(speed, e.1), scalia_render::pose::accel_pitch(e.4))
    }

    /// Where a point (absolute metres, `up` metres over the ground) is on screen (fractions from the top left).
    fn screen_of(&mut self, at: (f64, f64), up: f32) -> (f32, f32) {
        let z = self.height_at(at.0 as f32, at.1 as f32) + up;
        let (target, yaw, view_h) = self.last_cam;
        let (r, u, _) = scalia_render::camera::basis(yaw);
        let d = Vec3::new((at.0 - self.origin.0) as f32, (at.1 - self.origin.1) as f32, z) - target;
        let aspect = self.size.0 as f32 / self.size.1.max(1) as f32;
        (0.5 + d.dot(r) / (view_h * aspect), 0.5 - d.dot(u) / view_h)
    }

    /// How far the commander has dissolved for the tablet close-up (0 solid, 1 gone).
    fn cine_fade(&self, now_ms: f64) -> f32 {
        if let Some((_, _, fade)) = self.intro {
            return fade;
        }
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
                // Behind the tablet once it is up: the ground under him (blurred by the page), not the sky the
                // face-on framing would see.
                let ground = (base, h, 9.0, 1.2);
                mix(close, ground, ease((e - CINE_OPEN_MS - 450.0) / 700.0))
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
        let glide_ms = if self.inspect_glide { 220.0 } else { GLIDE_MS };
        let k = 1.0 - (-dt / glide_ms).exp();
        let (a, b) = (self.zoom_m.ln(), goal.ln());
        let mut z = (a + (b - a) * k as f32).exp();
        if (z / goal - 1.0).abs() < 1e-3 {
            z = goal;
            self.inspect_glide = false;
        }
        // The menu's intro flight sets the zoom outright.
        if let Some((view_m, _, _)) = self.intro {
            z = view_m;
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

/// A prop's bounding box (absolute metres), standing on the base pad at height `h`.
fn prop_box(p: &Prop, h: f32) -> (Vec3, Vec3) {
    let (x, y) = (p.x_mm as f32 / 1000.0, p.y_mm as f32 / 1000.0);
    match p.kind {
        PropKind::Rock { radius_mm } => {
            let r = radius_mm as f32 / 1000.0;
            (Vec3::new(x - r, y - r, h - 0.2), Vec3::new(x + r, y + r, h + r * 0.9))
        }
        PropKind::Wall { length_mm, thickness_mm } => {
            let (l, t) = (length_mm as f32 / 1000.0, thickness_mm as f32 / 1000.0);
            let (hx, hy) = if p.quarter_turns % 2 == 0 { (l / 2.0, t / 2.0) } else { (t / 2.0, l / 2.0) };
            let top = if t <= 0.65 { 2.75 } else if t <= 0.8 { 1.0 } else if t <= 3.0 { 2.6 } else { 3.9 };
            (Vec3::new(x - hx, y - hy, h), Vec3::new(x + hx, y + hy, h + top))
        }
        PropKind::Panel { length_mm, .. } => {
            let (l, t) = (length_mm as f32 / 1000.0, PropKind::PANEL_MM as f32 / 1000.0);
            let (hx, hy) = if p.quarter_turns % 2 == 0 { (l / 2.0, t / 2.0) } else { (t / 2.0, l / 2.0) };
            (Vec3::new(x - hx, y - hy, h), Vec3::new(x + hx, y + hy, h + scalia_core::killhouse::WALL_TOP_M))
        }
        PropKind::Crate { size_mm } => {
            let s = size_mm as f32 / 2000.0;
            let top = if size_mm >= 2_500 { 6.6 } else { s * 2.0 };
            (Vec3::new(x - s, y - s, h), Vec3::new(x + s, y + s, h + top))
        }
        PropKind::Furniture { item, .. } => {
            let (hx, hy) = p.half_extent();
            let (hx, hy) = (hx as f32 / 1000.0, hy as f32 / 1000.0);
            (Vec3::new(x - hx, y - hy, h), Vec3::new(x + hx, y + hy, h + item.height_mm() as f32 / 1000.0))
        }
    }
}

/// What a prop is, by its shape (the same rules `base_mesh` draws it by).
fn prop_kind(p: &Prop) -> &'static str {
    match p.kind {
        PropKind::Rock { .. } => "rock",
        PropKind::Wall { thickness_mm, .. } if thickness_mm <= 650 => "wall",
        PropKind::Wall { thickness_mm, .. } if thickness_mm <= 800 => "sandbags",
        PropKind::Wall { thickness_mm, .. } if thickness_mm <= 3_000 => "container",
        PropKind::Wall { .. } => "barracks",
        PropKind::Crate { size_mm } if size_mm >= 2_500 => "tower",
        PropKind::Crate { .. } => "crate",
        PropKind::Panel { .. } => "killhouse",
        PropKind::Furniture { item, .. } => match item {
            Furnish::Bunk => "bunk",
            Furnish::Locker => "locker",
            Furnish::Table => "table",
            Furnish::Bench => "bench",
            Furnish::Rack => "weapons rack",
            Furnish::Jukebox => "jukebox",
        },
    }
}

/// A building's layout for the structure scan: outer faces, doors, windows and room partitions (absolute metres).
/// Barracks run along x (24 x 8 m): four rooms, a door at each end and two on the south side, windows between.
/// Guard towers: an open platform with a firing port on each side and a ladder hatch.
struct Structure {
    faces: Vec<Vec<Vec3>>,
    doors: Vec<Vec<Vec3>>,
    windows: Vec<Vec<Vec3>>,
    rooms: Vec<Vec<Vec3>>,
}

/// Door and window layout of a barracks block, along its length from its west end (metres): (x, width, sill, height).
const BARRACKS_SOUTH: [(f32, f32, f32, f32, bool); 8] = [
    (2.5, 1.2, 1.0, 1.0, false), (6.0, 1.2, 0.0, 2.1, true), (9.0, 1.2, 1.0, 1.0, false), (11.5, 1.2, 1.0, 1.0, false),
    (14.5, 1.2, 1.0, 1.0, false), (18.0, 1.2, 0.0, 2.1, true), (21.5, 1.2, 1.0, 1.0, false), (0.0, 0.0, 0.0, 0.0, false),
];
const BARRACKS_NORTH: [f32; 8] = [2.0, 5.0, 8.0, 11.0, 13.0, 16.0, 19.0, 22.0];
const BARRACKS_ROOMS: [f32; 3] = [6.0, 12.0, 18.0];

fn structure_of(p: &Prop, h: f32) -> Structure {
    let (lo, hi) = prop_box(p, h);
    let box_faces = |lo: Vec3, hi: Vec3| {
        let c = |x: f32, y: f32, z: f32| Vec3::new(x, y, z);
        vec![
            vec![c(lo.x, lo.y, hi.z), c(hi.x, lo.y, hi.z), c(hi.x, hi.y, hi.z), c(lo.x, hi.y, hi.z)],
            vec![c(lo.x, lo.y, lo.z), c(hi.x, lo.y, lo.z), c(hi.x, lo.y, hi.z), c(lo.x, lo.y, hi.z)],
            vec![c(lo.x, hi.y, lo.z), c(hi.x, hi.y, lo.z), c(hi.x, hi.y, hi.z), c(lo.x, hi.y, hi.z)],
            vec![c(lo.x, lo.y, lo.z), c(lo.x, hi.y, lo.z), c(lo.x, hi.y, hi.z), c(lo.x, lo.y, hi.z)],
            vec![c(hi.x, lo.y, lo.z), c(hi.x, hi.y, lo.z), c(hi.x, hi.y, hi.z), c(hi.x, lo.y, hi.z)],
        ]
    };
    let mut s = Structure { faces: box_faces(lo, hi), doors: vec![], windows: vec![], rooms: vec![] };
    let quad_y = |y: f32, x0: f32, x1: f32, z0: f32, z1: f32| vec![Vec3::new(x0, y, z0), Vec3::new(x1, y, z0), Vec3::new(x1, y, z1), Vec3::new(x0, y, z1)];
    let quad_x = |x: f32, y0: f32, y1: f32, z0: f32, z1: f32| vec![Vec3::new(x, y0, z0), Vec3::new(x, y1, z0), Vec3::new(x, y1, z1), Vec3::new(x, y0, z1)];
    match prop_kind(p) {
        "barracks" => {
            let my = (lo.y + hi.y) / 2.0;
            for &(x, w, sill, ht, door) in BARRACKS_SOUTH.iter().filter(|q| q.1 > 0.0) {
                let q = quad_y(lo.y - 0.02, lo.x + x - w / 2.0, lo.x + x + w / 2.0, h + sill, h + sill + ht);
                if door { s.doors.push(q) } else { s.windows.push(q) }
            }
            for &x in &BARRACKS_NORTH {
                s.windows.push(quad_y(hi.y + 0.02, lo.x + x - 0.6, lo.x + x + 0.6, h + 1.0, h + 2.0));
            }
            s.doors.push(quad_x(lo.x - 0.02, my - 0.6, my + 0.6, h, h + 2.1));
            s.doors.push(quad_x(hi.x + 0.02, my - 0.6, my + 0.6, h, h + 2.1));
            for &x in &BARRACKS_ROOMS {
                s.rooms.push(vec![Vec3::new(lo.x + x, lo.y, hi.z), Vec3::new(lo.x + x, hi.y, hi.z)]);
            }
            s.rooms.push(vec![Vec3::new(lo.x, my + 1.0, hi.z), Vec3::new(hi.x, my + 1.0, hi.z)]); // the corridor
        }
        "killhouse" => {
            use scalia_core::killhouse::{SIZE_M, WALL_TOP_M, WINDOW_SILL_M, WINDOW_TOP_M};
            let k = killhouse();
            let (ox, oy) = (k.origin.0 as f32 / 1000.0, k.origin.1 as f32 / 1000.0);
            let (lo, hi) = (Vec3::new(ox, oy, h), Vec3::new(ox + SIZE_M.0 as f32, oy + SIZE_M.1 as f32, h + WALL_TOP_M));
            s.faces = box_faces(lo, hi).into_iter().skip(1).collect(); // no roof
            let quad = |x: i64, y: i64, along_x: bool, w: f32, z0: f32, z1: f32| {
                let (x, y) = (x as f32 / 1000.0, y as f32 / 1000.0);
                if along_x { quad_y(y, x - w / 2.0, x + w / 2.0, z0, z1) } else { quad_x(x, y - w / 2.0, y + w / 2.0, z0, z1) }
            };
            for d in &k.doors {
                s.doors.push(quad(d.x_mm, d.y_mm, d.along_x, d.width_mm as f32 / 1000.0, h, h + 2.1));
            }
            for w in &k.windows {
                s.windows.push(quad(w.x_mm, w.y_mm, w.along_x, 1.5, h + WINDOW_SILL_M, h + WINDOW_TOP_M));
            }
            for r in &k.rooms {
                let (a, b) = (Vec3::new(r.min.0 as f32 / 1000.0, r.min.1 as f32 / 1000.0, hi.z), Vec3::new(r.max.0 as f32 / 1000.0, r.max.1 as f32 / 1000.0, hi.z));
                s.rooms.push(vec![a, Vec3::new(b.x, a.y, a.z), b, Vec3::new(a.x, b.y, a.z), a]);
            }
        }
        "tower" => {
            let (z0, z1) = (h + 4.8, h + 5.7);
            let (mx, my, a) = ((lo.x + hi.x) / 2.0, (lo.y + hi.y) / 2.0, 0.5);
            s.windows.push(quad_y(lo.y - 0.02, mx - a, mx + a, z0 + 0.3, z1));
            s.windows.push(quad_y(hi.y + 0.02, mx - a, mx + a, z0 + 0.3, z1));
            s.windows.push(quad_x(lo.x - 0.02, my - a, my + a, z0 + 0.3, z1));
            s.windows.push(quad_x(hi.x + 0.02, my - a, my + a, z0 + 0.3, z1));
            s.doors.push(vec![Vec3::new(mx - 0.4, my - 0.4, h + 4.82), Vec3::new(mx + 0.4, my - 0.4, h + 4.82), Vec3::new(mx + 0.4, my + 0.4, h + 4.82), Vec3::new(mx - 0.4, my + 0.4, h + 4.82)]);
        }
        _ => {}
    }
    s
}

/// A piece of barracks furniture: steel bunks with olive mattresses, grey lockers, wooden tables and benches,
/// a weapons rack with rifles standing in it.
fn furniture_mesh(m: &mut MeshBuilder, p: &Prop, item: Furnish, h: f32) {
    use scalia_render::mesh::kit;
    let c = |q: u8| scalia_render::mesh::code(q, scalia_render::mesh::Part::Body);
    let (x, y) = (p.x_mm as f32 / 1000.0, p.y_mm as f32 / 1000.0);
    let (hx, hy) = p.half_extent();
    let (hx, hy) = (hx as f32 / 1000.0, hy as f32 / 1000.0);
    let top = item.height_mm() as f32 / 1000.0;
    let legs = |m: &mut MeshBuilder, up: f32, w: f32, col: u8| {
        for (sx, sy) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            let (lx, ly) = (x + sx * (hx - w), y + sy * (hy - w));
            m.cuboid([lx - w, ly - w, h], [lx + w, ly + w, h + up], c(col));
        }
    };
    match item {
        Furnish::Bunk => {
            legs(m, top, 0.03, kit::METAL);
            for z in [0.38f32, 1.38] {
                m.cuboid([x - hx, y - hy, h + z], [x + hx, y + hy, h + z + 0.06], c(kit::METAL));
                m.cuboid([x - hx + 0.04, y - hy + 0.04, h + z + 0.06], [x + hx - 0.04, y + hy - 0.04, h + z + 0.2], c(kit::UNIFORM_DARK));
            }
            // Pillows at the head (the end against the wall: whichever end is nearer the barracks' long walls).
            let head = if (y - barracks_mm().1 as f32 / 1000.0).abs() < (y - barracks_mm().3 as f32 / 1000.0).abs() { -1.0 } else { 1.0 };
            for z in [0.58f32, 1.58] {
                let py = y + head * (hy - 0.25);
                m.cuboid([x - hx + 0.1, py - 0.15, h + z], [x + hx - 0.1, py + 0.15, h + z + 0.1], c(15));
            }
        }
        Furnish::Locker => {
            m.cuboid([x - hx, y - hy, h], [x + hx, y + hy, h + top], c(kit::STRAP));
            m.cuboid([x - 0.01, y - hy - 0.01, h + 0.1], [x + 0.01, y + hy + 0.01, h + top - 0.1], c(kit::METAL));
        }
        Furnish::Jukebox => {
            // A small square table, and the jukebox on it: a wooden cabinet with a lit front and a speaker grille.
            let t = 0.78;
            legs(m, t - 0.04, 0.035, kit::COYOTE_DARK);
            m.cuboid([x - hx, y - hy, h + t - 0.04], [x + hx, y + hy, h + t], c(13));
            let (bx, by) = (0.26f32, 0.18f32);
            m.cuboid([x - bx, y - by, h + t], [x + bx, y + by, h + top], c(kit::COYOTE_DARK));
            // The front faces the room (south, -y): a lit panel above the grille.
            m.cuboid([x - bx + 0.04, y - by - 0.012, h + t + 0.32], [x + bx - 0.04, y - by, h + top - 0.05], c(kit::LENS));
            m.cuboid([x - bx + 0.06, y - by - 0.012, h + t + 0.06], [x + bx - 0.06, y - by, h + t + 0.26], c(kit::METAL));
        }
        Furnish::Table => {
            legs(m, top - 0.04, 0.04, kit::COYOTE_DARK);
            m.cuboid([x - hx, y - hy, h + top - 0.04], [x + hx, y + hy, h + top], c(13));
        }
        Furnish::Bench => {
            legs(m, top - 0.05, 0.04, kit::COYOTE_DARK);
            m.cuboid([x - hx, y - hy, h + top - 0.05], [x + hx, y + hy, h + top], c(13));
        }
        Furnish::Rack => {
            m.cuboid([x - hx, y - hy, h], [x + hx, y + hy, h + 0.12], c(kit::METAL));
            m.cuboid([x - hx, y - 0.03, h + top - 0.08], [x + hx, y + 0.03, h + top], c(kit::METAL));
            let n = ((hx * 2.0) / 0.25) as i32;
            for k in 0..n {
                let rx = x - hx + 0.125 + k as f32 * 0.25;
                m.cuboid([rx - 0.025, y - 0.06, h + 0.12], [rx + 0.025, y + 0.06, h + top - 0.05], c(1));
            }
        }
    }
}

/// A box standing on four ground corners (counter-clockwise from above), from z0 to z1.
fn slab(m: &mut MeshBuilder, c: [[f32; 2]; 4], z0: f32, z1: f32, colour: f32) {
    let p = |k: usize, z: f32| [c[k][0], c[k][1], z];
    let (a, b, cc, d) = (p(0, z0), p(1, z0), p(2, z0), p(3, z0));
    let (e, f, g, h) = (p(0, z1), p(1, z1), p(2, z1), p(3, z1));
    m.quad(a, d, cc, b, colour);
    m.quad(e, f, g, h, colour);
    m.quad(a, b, f, e, colour);
    m.quad(b, cc, g, f, colour);
    m.quad(cc, d, h, g, colour);
    m.quad(d, a, e, h, colour);
}

/// The doors as they hang now: a leaf on its hinge across the doorway when shut, swung back into the room when
/// open, knocked flat when breached; it swings over half a second from its last change (`since(ms)`: how long
/// ago, in real ms, battle time ms happened). Steel doors are darker, with a heavier frame.
fn door_mesh(doors: &[scalia_ai::doors::Door], steel: &[bool], h: f32, since: impl Fn(usize) -> f64) -> MeshBuilder {
    use scalia_ai::doors::DoorState;
    use scalia_render::mesh::{code, kit, Part};
    let mut m = MeshBuilder::default();
    for (i, d) in doors.iter().enumerate() {
        let is_steel = steel.get(i).copied().unwrap_or(false);
        let (cx, cy) = (d.at.0 as f32 / 1000.0, d.at.1 as f32 / 1000.0);
        let w = d.width_mm as f32 / 1000.0;
        let top = d.height_mm as f32 / 1000.0;
        let (u, n) = if d.along_x { ([1.0f32, 0.0f32], [0.0f32, 1.0f32]) } else { ([0.0, 1.0], [-1.0, 0.0]) };
        let hinge = [cx - u[0] * w / 2.0, cy - u[1] * w / 2.0];
        let leaf = w - 0.04;
        let t = if is_steel { 0.07 } else { 0.05 };
        // Where it is swinging to, and from.
        let (to, from) = match d.state {
            DoorState::Open => (88.0f32, 0.0f32),
            DoorState::Closed | DoorState::Locked => (0.0, 88.0),
            DoorState::Breached => (100.0, 0.0),
        };
        let k = ((since(i) / 450.0).clamp(0.0, 1.0) as f32).powf(0.6);
        let k = if d.changed_ms == 0 { 1.0 } else { k };
        let ang = (from + (to - from) * k).to_radians();
        let v = [u[0] * ang.cos() + n[0] * ang.sin(), u[1] * ang.cos() + n[1] * ang.sin()];
        let p = [-v[1], v[0]];
        let colour = if is_steel { code(1, Part::Body) } else { code(13, Part::Body) };
        let corners = |len: f32| [
            [hinge[0] - p[0] * t / 2.0, hinge[1] - p[1] * t / 2.0],
            [hinge[0] + v[0] * len - p[0] * t / 2.0, hinge[1] + v[1] * len - p[1] * t / 2.0],
            [hinge[0] + v[0] * len + p[0] * t / 2.0, hinge[1] + v[1] * len + p[1] * t / 2.0],
            [hinge[0] + p[0] * t / 2.0, hinge[1] + p[1] * t / 2.0],
        ];
        if d.state == DoorState::Breached && k >= 1.0 {
            // Off its hinges, lying in the doorway.
            let c = corners(leaf);
            let flat = [c[0], c[1], [c[1][0] + p[0] * top * 0.45, c[1][1] + p[1] * top * 0.45], [c[0][0] + p[0] * top * 0.45, c[0][1] + p[1] * top * 0.45]];
            slab(&mut m, flat, h + 0.01, h + 0.06, colour);
        } else {
            slab(&mut m, corners(leaf), h + 0.02, h + top, colour);
            // The handle, on the free edge, both faces.
            let e = [hinge[0] + v[0] * (leaf - 0.1), hinge[1] + v[1] * (leaf - 0.1)];
            let r = t / 2.0 + 0.04;
            m.cuboid([e[0] - 0.04 - p[0].abs() * r, e[1] - 0.04 - p[1].abs() * r, h + 1.0], [e[0] + 0.04 + p[0].abs() * r, e[1] + 0.04 + p[1].abs() * r, h + 1.08], code(kit::METAL, Part::Body));
        }
        // The frame's header over the doorway (the posts are the walls' ends).
        let f = if is_steel { 0.12 } else { 0.08 };
        let (a, b) = ([cx - u[0] * w / 2.0 - n[0] * f, cy - u[1] * w / 2.0 - n[1] * f], [cx + u[0] * w / 2.0 + n[0] * f, cy + u[1] * w / 2.0 + n[1] * f]);
        m.cuboid([a[0].min(b[0]), a[1].min(b[1]), h + top], [a[0].max(b[0]), a[1].max(b[1]), h + top + 0.12], colour);
    }
    m
}

/// The commander's body radius for walking into things (mm).
const CMD_RADIUS_MM: i64 = 280;

/// Whether a man of radius `r` standing at (x, y) (mm) touches no prop.
fn body_clear(props: &[Prop], x: i64, y: i64, r: i64) -> bool {
    props.iter().all(|p| {
        let (dx, dy) = (x - p.x_mm, y - p.y_mm);
        match p.kind {
            PropKind::Rock { radius_mm } => dx * dx + dy * dy >= (radius_mm + r) * (radius_mm + r),
            _ => {
                let (hx, hy) = p.half_extent();
                let (ox, oy) = ((dx.abs() - hx).max(0), (dy.abs() - hy).max(0));
                ox * ox + oy * oy >= r * r
            }
        }
    })
}

/// Low-poly mesh for the base's props, standing on the levelled pad at height `h` (same style as the soldiers).
fn base_mesh(props: &[Prop], h: f32) -> MeshBuilder {
    use scalia_render::mesh::kit;
    let mut m = MeshBuilder::default();
    let c = |p: u8| scalia_render::mesh::code(p, scalia_render::mesh::Part::Body);
    // The range's concrete floor.
    let (x0, y0, x1, y1) = (range_mm().0 as f32 / 1000.0, range_mm().1 as f32 / 1000.0, range_mm().2 as f32 / 1000.0, range_mm().3 as f32 / 1000.0);
    m.cuboid([x0, y0, h - 0.05], [x1, y1, h + 0.03], c(2));
    // The barracks' concrete floor.
    let (x0, y0, x1, y1) = (barracks_mm().0 as f32 / 1000.0, barracks_mm().1 as f32 / 1000.0, barracks_mm().2 as f32 / 1000.0, barracks_mm().3 as f32 / 1000.0);
    m.cuboid([x0, y0, h - 0.05], [x1, y1, h + 0.03], c(19));
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
                    // Barracks block with a flat roof, its doors and windows (the layout the structure scan reports).
                    m.cuboid([x - hx, y - hy, h], [x + hx, y + hy, h + 3.6], c(kit::UNIFORM));
                    m.cuboid([x - hx - 0.3, y - hy - 0.3, h + 3.6], [x + hx + 0.3, y + hy + 0.3, h + 3.9], c(kit::UNIFORM_DARK));
                    let st = structure_of(p, h);
                    for q in st.doors.iter().chain(st.windows.iter()) {
                        let (a, b) = (q[0], q[2]);
                        let n = Vec3::new(if (a.x - b.x).abs() < 1e-3 { 0.04 } else { 0.0 }, if (a.y - b.y).abs() < 1e-3 { 0.04 } else { 0.0 }, 0.0);
                        let (lo, hi) = (a.min(b) - n, a.max(b) + n);
                        m.cuboid([lo.x, lo.y, lo.z], [hi.x, hi.y, hi.z], c(kit::GLOVE));
                    }
                }
            }
            PropKind::Furniture { item, .. } => furniture_mesh(&mut m, p, item, h),
            PropKind::Panel { length_mm, window } => {
                // Plywood on a dark frame: posts at the ends, a window as a sill wall and a header.
                use scalia_core::killhouse::{WALL_TOP_M, WINDOW_SILL_M, WINDOW_TOP_M};
                let (l, t) = (length_mm as f32 / 1000.0, PropKind::PANEL_MM as f32 / 1000.0);
                let along_x = p.quarter_turns % 2 == 0;
                let (hx, hy) = if along_x { (l / 2.0, t / 2.0) } else { (t / 2.0, l / 2.0) };
                let (lo, hi) = ([x - hx, y - hy], [x + hx, y + hy]);
                if window {
                    m.cuboid([lo[0], lo[1], h], [hi[0], hi[1], h + WINDOW_SILL_M], c(kit::COYOTE_DARK));
                    m.cuboid([lo[0], lo[1], h + WINDOW_TOP_M], [hi[0], hi[1], h + WALL_TOP_M], c(kit::COYOTE_DARK));
                    // Sill and frame.
                    m.cuboid([lo[0] - 0.03, lo[1] - 0.03, h + WINDOW_SILL_M], [hi[0] + 0.03, hi[1] + 0.03, h + WINDOW_SILL_M + 0.05], c(13));
                } else {
                    m.cuboid([lo[0], lo[1], h], [hi[0], hi[1], h + WALL_TOP_M], c(kit::COYOTE_DARK));
                }
                for e in [-1.0f32, 1.0] {
                    let (px, py) = if along_x { (x + e * (hx - 0.05), y) } else { (x, y + e * (hy - 0.05)) };
                    m.cuboid([px - 0.06, py - 0.12, h], [px + 0.06, py + 0.12, h + WALL_TOP_M + 0.05], c(13));
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

/// The enemy's outfit: the same soldier with a red helmet and red plate carrier, readable at any distance.
fn enemy_kit(mut f: MeshBuilder) -> MeshBuilder {
    use scalia_render::mesh::{code, kit, palette_of, part_of};
    for v in &mut f.vertices {
        if v.color >= 0.0 && matches!(palette_of(v.color) as u8, kit::HELMET | kit::VEST) {
            v.color = code(ENEMY_RED, part_of(v.color));
        }
    }
    f
}
/// Palette red for the enemy's kit and armbands.
const ENEMY_RED: u8 = 9;

/// The enemy squad stationed in the training house (absolute metres from its south-west corner): position, the
/// point it watches, pose. Covering the doors and windows from inside, two to a room.
const ENEMY_POSTS: [((f32, f32), (f32, f32), Pose); 8] = [
    ((12.5, 3.5), (12.7, 0.0), Pose::CoverAim),
    ((16.0, 2.0), (20.0, 3.7), Pose::StandAim),
    ((5.0, 2.0), (3.7, 0.0), Pose::StandAim),
    ((2.5, 5.0), (8.0, 3.6), Pose::CoverAim),
    ((3.0, 10.6), (0.0, 10.7), Pose::CoverAim),
    ((6.5, 12.0), (5.2, 14.0), Pose::StandAim),
    ((16.5, 10.2), (20.0, 10.2), Pose::CoverAim),
    ((13.0, 12.5), (10.6, 10.6), Pose::Stand),
];

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

/// The commander's weapon handling on foot: trigger and aim held, where the mouse points (screen px), his stance
/// (0 standing, 1 crouched, 2 prone), the rifle slung, rounds fired (the page shakes the view on each), and the
/// aim line's ends this frame (screen fractions) while aiming.
#[derive(Default)]
struct Gun {
    trigger: bool,
    /// The trigger was pressed since the last round (fires once even if released before the next frame).
    pull: bool,
    aim: bool,
    at_px: Option<(f64, f64)>,
    stance: u8,
    slung: bool,
    shots: u32,
    line: Option<[f32; 4]>,
    /// The way the rifle points (absolute metres, from him), once the mouse has been seen.
    toward: Option<(f32, f32)>,
    /// When the empty rifle last clicked (ms): once per squeeze, not every frame.
    dry_at: f64,
}

/// The battle's man standing for the commander (joined on first use).
fn commander_soldier(a: &mut crate::assault::Assault, at_mm: (i64, i64)) -> usize {
    match a.battle.soldiers.iter().position(|s| s.player) {
        Some(i) => i,
        None => {
            let i = a.battle.join_player(0, COMMANDER_SQUAD, at_mm);
            a.battle.soldiers[i].mags = COMMANDER_MAGS;
            i
        }
    }
}


#[cfg(test)]
mod audio_spatial_tests {
    use super::*;
    use scalia_core::nav::{Prop, PropKind};

    #[test]
    fn tablet_listener_height_is_used_for_occlusion() {
        let wall = Prop {
            kind: PropKind::Wall { length_mm: 20_000, thickness_mm: 300 },
            x_mm: 50_000,
            y_mm: 50_000,
            quarter_turns: 1,
        };
        let sight = scalia_ai::sight::SightGrid::build(0, 0, 400, 400, &[wall]);
        let ear = (40.0, 50.0);
        let source = (60.0, 50.0);
        let field = scalia_audio::Field::build(&sight, ear, 32.0);

        let on_foot = sound_occlusion(&sight, Some(&field), ear, source, 19.2, false);
        let tablet = sound_occlusion(&sight, Some(&field), ear, source, 30.0, true);

        assert!(on_foot.gain() < 0.8 && on_foot.lowpass_hz() < 18_000.0, "{on_foot:?}");
        assert_eq!(tablet, scalia_audio::Occlusion::NONE, "{tablet:?}");
    }

    #[test]
    fn tablet_listener_still_hears_a_wall_that_intersects_the_elevated_ray() {
        let wall = Prop {
            kind: PropKind::Wall { length_mm: 20_000, thickness_mm: 300 },
            x_mm: 50_000,
            y_mm: 50_000,
            quarter_turns: 1,
        };
        let sight = scalia_ai::sight::SightGrid::build(0, 0, 400, 400, &[wall]);
        let ear = (40.0, 50.0);
        let source = (60.0, 50.0);
        let tablet = sound_occlusion(&sight, None, ear, source, 4.0, true);
        assert!(tablet.wall_db >= 40.0, "{tablet:?}");
    }
}
