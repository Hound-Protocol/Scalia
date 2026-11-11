//! JavaScript bridge: the same deterministic core, compiled to WebAssembly.

use scalia_core::derive::{derive_visible, Derived, ViewBox};
use scalia_core::position::RegionId;
use scalia_core::scenario::build_scenario;
use scalia_core::state_hash::{derive_hash, state_hash};
use scalia_core::time::WorldMs;
use scalia_core::world::World;
use wasm_bindgen::prelude::*;

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
    pub fn new(seed: u64, groups: u32, per_group: u32) -> Sim {
        Sim {
            world: build_scenario(seed, groups, per_group),
            scratch: Vec::new(),
            xs: Vec::new(),
            ys: Vec::new(),
            zs: Vec::new(),
            phases: Vec::new(),
        }
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
