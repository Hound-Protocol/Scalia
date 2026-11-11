//! JavaScript bridge: the same deterministic core, compiled to WebAssembly.

use scalia_core::derive::{derive_visible, Derived, ViewBox};
use scalia_core::flight::{flight_pos, plan_flight, FlightPlan};
use scalia_core::position::RegionId;
use scalia_core::system::{BodyId, System, PARK_OFFSET};
use scalia_core::scenario::build_scenario;
use scalia_core::state_hash::{derive_hash, state_hash};
use scalia_core::time::WorldMs;
use scalia_core::world::World;
use wasm_bindgen::prelude::*;

fn json_str(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            c if (c as u32) < 0x20 => o.push(' '),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// Parse and validate blueprint text, then draw its top-down SVG.
#[wasm_bindgen]
pub fn blueprint_svg(text: &str) -> Result<String, JsError> {
    let b = scalia_blueprint::parse(text).map_err(|e| JsError::new(&e.to_string()))?;
    Ok(scalia_blueprint::draw::svg(&b))
}

/// Compact JSON summary: id, name, class, triangles and the mesh hash as a decimal string.
#[wasm_bindgen]
pub fn blueprint_info(text: &str) -> Result<String, JsError> {
    let b = scalia_blueprint::parse(text).map_err(|e| JsError::new(&e.to_string()))?;
    let m = scalia_blueprint::mesh::mesh(&b);
    Ok(format!(
        "{{\"id\":{},\"name\":{},\"class\":\"{:?}\",\"triangles\":{},\"mesh_hash\":\"{}\"}}",
        json_str(&b.id),
        json_str(&b.name),
        b.class,
        m.indices.len() / 3,
        scalia_blueprint::mesh::mesh_hash(&m)
    ))
}

/// File text of shipped blueprint `index`, or undefined when there is no such blueprint.
#[wasm_bindgen]
pub fn shipped_ron(index: u32) -> Option<String> {
    scalia_blueprint::shipped::SHIPPED_RON.get(index as usize).map(|s| s.to_string())
}

/// Heights of one planet chunk, `(cells + 1)^2` values in millimetres.
#[wasm_bindgen]
pub fn planet_chunk(seed: u64, face: u8, cx: i32, cy: i32, cells: u32) -> Vec<i32> {
    scalia_core::planet::chunk(seed, face, cx as i64, cy as i64, cells.min(1024)).heights
}

#[wasm_bindgen]
pub fn planet_chunk_hash(seed: u64, face: u8, cx: i32, cy: i32, cells: u32) -> u64 {
    scalia_core::planet::chunk_hash(&scalia_core::planet::chunk(seed, face, cx as i64, cy as i64, cells.min(1024)))
}

/// Most soldiers a scenario built from JavaScript may hold (about 160 MB of roster).
const MAX_SCENARIO_PEOPLE: u64 = 20_000_000;

fn to_ms(v: f64) -> u64 {
    if v.is_finite() && v > 0.0 { v as u64 } else { 0 }
}

fn to_mm(v: f64) -> i64 {
    if v.is_finite() { v as i64 } else { 0 }
}

/// Same computation as the native `hash_scenario` binary.
#[wasm_bindgen]
pub fn scenario_hash(seed: u64) -> u64 {
    let mut w = build_scenario(seed, 200, 150);
    for s in 1..=10u64 {
        w.tick(WorldMs(s * 1000)).expect("time moves forward");
    }
    let view = ViewBox {
        region: RegionId(1),
        min_x: -1_000_000,
        max_x: 6_000_000,
        min_y: -1_000_000,
        max_y: 6_000_000,
    };
    state_hash(&w) ^ derive_hash(&w, WorldMs(10_000), &view, 5000)
}

#[wasm_bindgen]
pub struct Sim {
    world: World,
    scratch: Vec<Derived>,
    xs: Vec<f32>,
    ys: Vec<f32>,
    zs: Vec<f32>,
    phases: Vec<u8>,
}

#[wasm_bindgen]
impl Sim {
    #[wasm_bindgen(constructor)]
    pub fn new(seed: u64, groups: u32, per_group: u32) -> Result<Sim, JsError> {
        if per_group > scalia_core::groups::MAX_GROUP_PEOPLE || groups as u64 * per_group as u64 > MAX_SCENARIO_PEOPLE {
            return Err(JsError::new("scenario too large: reduce groups or people per group"));
        }
        Ok(Sim {
            world: build_scenario(seed, groups, per_group),
            scratch: Vec::new(),
            xs: Vec::new(),
            ys: Vec::new(),
            zs: Vec::new(),
            phases: Vec::new(),
        })
    }

    pub fn tick(&mut self, now_ms: f64) -> Result<(), JsError> {
        self.world
            .tick(WorldMs(to_ms(now_ms)))
            .map_err(|_| JsError::new("time went backwards"))
    }

    /// Fills the output buffers and returns how many individuals were derived.
    /// Positions are `f32` metres relative to the view centre.
    pub fn derive(
        &mut self,
        t_ms: f64,
        region: u32,
        min_x: f64,
        max_x: f64,
        min_y: f64,
        max_y: f64,
        cap: u32,
    ) -> u32 {
        let view = ViewBox {
            region: RegionId(region),
            min_x: to_mm(min_x),
            max_x: to_mm(max_x),
            min_y: to_mm(min_y),
            max_y: to_mm(max_y),
        };
        derive_visible(&self.world, WorldMs(to_ms(t_ms)), &view, cap as usize, &mut self.scratch);
        let cx = (view.min_x as i128 + view.max_x as i128) / 2;
        let cy = (view.min_y as i128 + view.max_y as i128) / 2;
        self.xs.clear();
        self.ys.clear();
        self.zs.clear();
        self.phases.clear();
        for d in &self.scratch {
            self.xs.push(((d.pos.x as i128 - cx) as f64 / 1000.0) as f32);
            self.ys.push(((d.pos.y as i128 - cy) as f64 / 1000.0) as f32);
            self.zs.push((d.pos.z as f64 / 1000.0) as f32);
            self.phases.push(d.phase as u8);
        }
        self.scratch.len() as u32
    }

    pub fn xs(&self) -> Vec<f32> {
        self.xs.clone()
    }
    pub fn ys(&self) -> Vec<f32> {
        self.ys.clone()
    }
    pub fn zs(&self) -> Vec<f32> {
        self.zs.clone()
    }
    pub fn phases(&self) -> Vec<u8> {
        self.phases.clone()
    }
    pub fn xs_ptr(&self) -> *const f32 {
        self.xs.as_ptr()
    }
    pub fn ys_ptr(&self) -> *const f32 {
        self.ys.as_ptr()
    }
    pub fn zs_ptr(&self) -> *const f32 {
        self.zs.as_ptr()
    }
    pub fn phases_ptr(&self) -> *const u8 {
        self.phases.as_ptr()
    }
}

// ---- orbits and flight ----

#[wasm_bindgen]
pub fn system_body_count() -> u32 {
    System::demo().bodies.len() as u32
}

#[wasm_bindgen]
pub fn system_body_name(i: u32) -> Option<String> {
    System::demo().bodies.get(i as usize).map(|b| b.name.to_string())
}

/// `[x_mm, y_mm]` of a body in the system region; empty for an unknown body.
#[wasm_bindgen]
pub fn system_body_pos(i: u32, t_ms: f64) -> Vec<f64> {
    match System::demo().body_pos(BodyId(i.min(255) as u8), WorldMs(to_ms(t_ms))) {
        Some(p) => vec![p.x as f64, p.y as f64],
        None => Vec::new(),
    }
}

/// A flight from the parking spot at one body to the parking spot at another.
#[wasm_bindgen]
pub struct Flight {
    plan: FlightPlan,
}

#[wasm_bindgen]
impl Flight {
    #[wasm_bindgen(constructor)]
    pub fn new(from_body: u32, to_body: u32, depart_ms: f64, peak_mm_s: f64) -> Result<Flight, JsError> {
        let sys = System::demo();
        let start = WorldMs(to_ms(depart_ms));
        let from = sys
            .body_pos(BodyId(from_body.min(255) as u8), start)
            .ok_or_else(|| JsError::new("unknown origin body"))?
            .offset(PARK_OFFSET.0, PARK_OFFSET.1, PARK_OFFSET.2);
        let speed = if peak_mm_s.is_finite() && peak_mm_s > 0.0 { peak_mm_s.min(u32::MAX as f64) as u32 } else { 0 };
        let plan = plan_flight(&sys, from, BodyId(to_body.min(255) as u8), PARK_OFFSET, speed, start)
            .map_err(|e| JsError::new(&format!("cannot plan flight: {e:?}")))?;
        Ok(Flight { plan })
    }

    pub fn duration_ms(&self) -> f64 {
        self.plan.duration_ms as f64
    }

    /// `[x_mm, y_mm]` at time `t_ms`.
    pub fn pos(&self, t_ms: f64) -> Vec<f64> {
        let p = flight_pos(&self.plan, WorldMs(to_ms(t_ms)));
        vec![p.x as f64, p.y as f64]
    }

    pub fn from(&self) -> Vec<f64> {
        vec![self.plan.from.x as f64, self.plan.from.y as f64]
    }

    pub fn to(&self) -> Vec<f64> {
        vec![self.plan.to.x as f64, self.plan.to.y as f64]
    }
}
