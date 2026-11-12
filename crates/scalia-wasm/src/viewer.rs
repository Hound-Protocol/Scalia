//! The game viewer in Rust: three scenes drawn by `scalia-render`, driven by the core directly.
//! JavaScript only forwards the animation frame and input.

use std::collections::HashMap;

use glam::Vec3;
use scalia_core::derive::{derive_visible, Derived, Phase, ViewBox};
use scalia_core::flight::{flight_pos, plan_flight, FlightPlan, DEMO_PEAK_SPEED_MM_S};
use scalia_core::planet::Planet;
use scalia_core::position::RegionId;
use scalia_core::scenario::build_scenario;
use scalia_core::system::{BodyId, System, PARK_OFFSET};
use scalia_core::time::WorldMs;
use scalia_core::world::World;
use scalia_render::gpu::{FrameDesc, Gpu};
use scalia_render::mesh::{figure, MeshBuilder};
use scalia_render::sky::{clock, sky};
use wasm_bindgen::prelude::*;

const SOLDIERS: u32 = 2000;
const CELLS: u32 = 64;
const KM: f64 = 1e6;
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
    ground_view: f32,
    focus: Vec3,
    system: System,
    sys_scale: f32,
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
        world: build_scenario(42, 1, SOLDIERS),
        derived: Vec::new(),
        chunks: HashMap::new(),
        ground_view: 60.0,
        focus: Vec3::ZERO,
        system: System::demo(),
        sys_scale: 260_000.0,
        docked: 0,
        flight: None,
        trail: Vec::new(),
        ground_cam: None,
        sys_cam: None,
        yaw: std::f32::consts::FRAC_PI_4,
        ship_yaw: None,
        pixel_scale: 2,
        day_start: 0.3,
        day_rate: 1.0 / 360_000.0,
        day_at: 0.0,
        last_view: (60.0, 1.0),
        fps: 0.0,
        frames: 0,
        fps_t: 0.0,
        size,
    };
    v.gpu.upload("figure", figure().bytes());
    Ok(v)
}

fn wgpu_target(canvas: web_sys::HtmlCanvasElement) -> scalia_render::SurfaceTarget<'static> {
    scalia_render::SurfaceTarget::Canvas(canvas)
}

fn bytes(v: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 4) }
}

#[wasm_bindgen]
impl Viewer {
    pub fn set_scene(&mut self, name: &str) {
        self.scene = match name {
            "ground" => Scene::Ground,
            "system" => Scene::System,
            _ => Scene::Ships,
        };
    }
    pub fn set_ship(&mut self, i: u32) {
        self.ship = (i as usize).min(self.ships.len() - 1);
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
                self.ground_cam = Some(base + delta);
            }
            Scene::System => self.sys_cam = Some(self.sys_cam.unwrap_or(Vec3::ZERO) + delta),
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
            Scene::System => self.sys_cam = None,
            Scene::Ships => self.ship_yaw = None,
        }
    }
    /// Cycles the speed of the day: 6-minute days, 12-second days, paused.
    pub fn cycle_day_speed(&mut self, now_ms: f64) -> String {
        let t = self.sim_ms(now_ms);
        self.day_start = self.time_of_day(t);
        self.day_at = t;
        self.day_rate = if self.day_rate == 1.0 / 360_000.0 {
            1.0 / 12_000.0
        } else if self.day_rate > 0.0 {
            0.0
        } else {
            1.0 / 360_000.0
        };
        match self.day_rate {
            r if r == 0.0 => "time paused",
            r if r > 1.0 / 100_000.0 => "fast day",
            _ => "normal day",
        }
        .into()
    }
    pub fn scene_name(&self) -> String {
        match self.scene {
            Scene::Ships => "ships",
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
    /// Mouse wheel: positive zooms out.
    pub fn wheel(&mut self, delta: f64) {
        let k = if delta > 0.0 { 1.15 } else { 1.0 / 1.15 };
        match self.scene {
            Scene::System => self.sys_scale = (self.sys_scale * k).clamp(30.0, 500_000.0),
            Scene::Ground => self.ground_view = (self.ground_view * k).clamp(8.0, 160.0),
            Scene::Ships => {}
        }
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
        let (mut meshes, mut instanced) = (Vec::new(), Vec::new());
        let (target, yaw, view_height, range, text) = match self.scene {
            Scene::Ships => self.ships_scene(t, &mut meshes),
            Scene::Ground => self.ground_scene(t, &mut meshes, &mut instanced),
            Scene::System => self.system_scene(t, &mut meshes),
        };
        self.last_view = (view_height, self.size.1 as f32);
        let tod = self.time_of_day(t) as f32;
        let (sky_now, atmosphere) = match self.scene {
            Scene::System => (sky(0.5), false),
            _ => (sky(tod), true),
        };
        let desc = FrameDesc { meshes: &meshes, instanced: &instanced, target, yaw, view_height, range, internal, outline: self.outline, sky: sky_now, atmosphere };
        if let Err(e) = self.gpu.frame(&desc) {
            return format!("Render error: {e}");
        }
        let gpu = if self.gpu.software { " | SOFTWARE RENDERING: the CPU is drawing" } else { "" };
        let time = if self.scene == Scene::System { String::new() } else { format!(" | {}", clock(tod)) };
        format!("{text}{time} | {}x{} | {:.0} fps{gpu}", internal.0, internal.1, self.fps)
    }
}

impl Viewer {
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

    fn ships_scene(&mut self, t: f64, meshes: &mut Vec<String>) -> (Vec3, f32, f32, f32, String) {
        let i = self.ship;
        let s = self.ship_info(i);
        let ext = s.hi - s.lo;
        let size = ext.max_element();
        let text = format!("{}: {} triangles, {:.1} m long", s.name, s.triangles, ext.y);
        let centre = (s.lo + s.hi) / 2.0;
        meshes.push(format!("ship{i}"));
        let yaw = self.ship_yaw.unwrap_or((t / 2500.0) as f32);
        (centre, yaw, size, size * 3.0, text)
    }

    fn chunk(&mut self, cx: i64, cy: i64) -> &Vec<f32> {
        if !self.chunks.contains_key(&(cx, cy)) {
            let c = Planet::demo(42).chunk(cx, cy + EQUATOR_CHUNK, CELLS);
            let n = CELLS as usize + 1;
            let hm: Vec<f32> = c.heights.iter().map(|&h| h as f32 / 1000.0).collect();
            let mut m = MeshBuilder::default();
            let base = (cx as f32 * CELLS as f32, cy as f32 * CELLS as f32);
            let p = |i: usize, j: usize| [base.0 + i as f32, base.1 + j as f32, hm[j * n + i]];
            for j in 0..CELLS as usize {
                for i in 0..CELLS as usize {
                    let (a, b, c2, d) = (p(i, j), p(i + 1, j), p(i + 1, j + 1), p(i, j + 1));
                    let col = 10.0 + ((a[2] * 8.0).floor().rem_euclid(4.0));
                    m.tri(a, b, c2, col);
                    m.tri(a, c2, d, col);
                }
            }
            self.gpu.upload(&format!("terrain{cx},{cy}"), m.bytes());
            self.chunks.insert((cx, cy), hm);
        }
        &self.chunks[&(cx, cy)]
    }

    fn height_at(&mut self, x: f32, y: f32) -> f32 {
        let (cx, cy) = ((x / CELLS as f32).floor() as i64, (y / CELLS as f32).floor() as i64);
        let n = CELLS as usize + 1;
        let i = (x - cx as f32 * CELLS as f32).round().clamp(0.0, CELLS as f32) as usize;
        let j = (y - cy as f32 * CELLS as f32).round().clamp(0.0, CELLS as f32) as usize;
        self.chunk(cx, cy)[j * n + i]
    }

    fn ground_scene(&mut self, t: f64, meshes: &mut Vec<String>, instanced: &mut Vec<(String, String)>) -> (Vec3, f32, f32, f32, String) {
        let now = WorldMs(t as u64);
        let tick = WorldMs(now.0.max(self.world.now.0));
        let _ = self.world.tick(tick);
        let view = ViewBox { region: RegionId(1), min_x: -1_000_000_000, max_x: 1_000_000_000, min_y: -1_000_000_000, max_y: 1_000_000_000 };
        let mut out = std::mem::take(&mut self.derived);
        derive_visible(&self.world, tick, &view, SOLDIERS as usize, &mut out);
        let n = out.len();
        let (mut cx, mut cy) = (0.0f64, 0.0f64);
        for d in &out {
            cx += d.pos.x as f64 / 1000.0;
            cy += d.pos.y as f64 / 1000.0;
        }
        if n > 0 {
            cx /= n as f64;
            cy /= n as f64;
        }
        let mut inst = Vec::with_capacity(n * 4);
        for d in &out {
            let (x, y) = (d.pos.x as f32 / 1000.0, d.pos.y as f32 / 1000.0);
            let z = self.height_at(x, y);
            inst.extend_from_slice(&[x, y, z, if d.phase == Phase::Marching { 7.0 } else { 6.0 }]);
        }
        self.derived = out;
        self.gpu.upload("soldiers", bytes(&inst));
        let look = self.ground_cam.map(|c| (c.x as f64, c.y as f64)).unwrap_or((cx, cy));
        let (gx, gy) = ((look.0 / CELLS as f64).floor() as i64, (look.1 / CELLS as f64).floor() as i64);
        // Enough chunks to cover a wide screen at any yaw; at most a few new ones per frame to avoid hitches.
        let rad = (self.ground_view * 1.3 / CELLS as f32).ceil() as i64 + 1;
        let mut budget = 6;
        for dy in -rad..=rad {
            for dx in -rad..=rad {
                let key = (gx + dx, gy + dy);
                if !self.chunks.contains_key(&key) {
                    if budget == 0 {
                        continue;
                    }
                    budget -= 1;
                    self.chunk(key.0, key.1);
                }
                meshes.push(format!("terrain{},{}", key.0, key.1));
            }
        }
        let keep = rad + 2;
        let far: Vec<_> = self.chunks.keys().copied().filter(|&(x, y)| (x - gx).abs() > keep || (y - gy).abs() > keep).collect();
        for k in far {
            self.chunks.remove(&k);
            self.gpu.drop_buffer(&format!("terrain{},{}", k.0, k.1));
        }
        instanced.push(("figure".into(), "soldiers".into()));
        self.focus = Vec3::new(cx as f32, cy as f32, 0.0);
        let mut target = self.ground_cam.unwrap_or(self.focus);
        target.z = self.height_at(target.x, target.y);
        let mode = if self.ground_cam.is_some() { "free camera (F follows)" } else { "following the group" };
        let text = format!("Ground: {n} soldiers, {mode}, {:.0} m view", self.ground_view);
        (target, self.yaw, self.ground_view, 600.0, text)
    }

    fn system_scene(&mut self, t: f64, meshes: &mut Vec<String>) -> (Vec3, f32, f32, f32, String) {
        let now = WorldMs(t as u64);
        let mut m = MeshBuilder::default();
        let pix = self.sys_scale / 270.0;
        let names: Vec<&str> = self.system.bodies.iter().map(|b| b.name).collect();
        for (i, b) in self.system.bodies.iter().enumerate() {
            let p = self.system.body_pos(BodyId(i as u8), now).expect("known body");
            m.ring([0.0, 0.0], (b.orbit_radius_mm as f64 / KM) as f32, (pix * 0.6).max(20.0), 128, 3.0);
            m.diamond([(p.x as f64 / KM) as f32, (p.y as f64 / KM) as f32, 0.0], (pix * 5.0).max(800.0), if i == 0 { 6.0 } else { 5.0 });
        }
        m.diamond([0.0, 0.0, 0.0], (pix * 9.0).max(2500.0), 14.0);
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
                text = format!("Docked at {}. Wheel zooms ({:.0} thousand km across).", names[self.docked as usize], self.sys_scale / 1000.0);
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
            m.diamond([x, y, 0.0], (pix * 1.2).max(120.0), 8.0);
        }
        m.cuboid([sx - pix * 2.5, sy - pix * 2.5, 0.0], [sx + pix * 2.5, sy + pix * 2.5, pix * 5.0], 15.0);
        self.gpu.upload("system", m.bytes());
        meshes.push("system".into());
        if self.flight.is_some() {
            text.push_str(" (in flight)");
        }
        let target = self.sys_cam.unwrap_or(Vec3::ZERO);
        (target, self.yaw, self.sys_scale, self.sys_scale * 4.0, text)
    }
}
