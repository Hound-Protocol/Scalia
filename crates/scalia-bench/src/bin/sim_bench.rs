//! Budget checks: one tick of a million soldiers and deriving 20,000 individuals.

use scalia_core::derive::{derive_visible, Derived, ViewBox};
use scalia_core::position::RegionId;
use scalia_core::scenario::build_scenario;
use scalia_core::time::WorldMs;
use std::time::Instant;

const TICK_BUDGET_MS: f64 = 5.0;
const DERIVE_BUDGET_MS: f64 = 2.0;
const CHUNK_BUDGET_MS: f64 = 1.0;
/// One squad-level GOAP plan (about 14 actions, 12 facts).
const PLAN_BUDGET_MS: f64 = 0.05;

fn median_ms(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn main() {
    let report_only = std::env::args().any(|a| a == "--report-only");
    let mut w = build_scenario(42, 6_667, 150);
    println!("soldiers={} groups={}", w.roster.len(), w.groups.len());

    let mut t = 0u64;
    for _ in 0..3 {
        t += 1000;
        w.tick(WorldMs(t)).unwrap();
    }
    let mut ticks = Vec::new();
    for _ in 0..30 {
        t += 1000;
        let s = Instant::now();
        w.tick(WorldMs(t)).unwrap();
        ticks.push(s.elapsed().as_secs_f64() * 1000.0);
    }
    let tick_ms = median_ms(ticks);

    let view = ViewBox {
        region: RegionId(1),
        min_x: -1_000_000,
        max_x: 40_000_000,
        min_y: -1_000_000,
        max_y: 40_000_000,
    };
    let mut out: Vec<Derived> = Vec::with_capacity(20_000);
    let now = WorldMs(t);
    for _ in 0..3 {
        derive_visible(&w, now, &view, 20_000, &mut out);
    }
    let mut derives = Vec::new();
    for _ in 0..30 {
        let s = Instant::now();
        derive_visible(&w, now, &view, 20_000, &mut out);
        derives.push(s.elapsed().as_secs_f64() * 1000.0);
    }
    assert_eq!(out.len(), 20_000);
    let derive_ms = median_ms(derives);

    let mut chunks = Vec::new();
    let mut guard = 0u64;
    for k in 0..30i64 {
        let s = Instant::now();
        let c = scalia_core::planet::Planet::demo(42).chunk(k, 1000 - k, 64);
        chunks.push(s.elapsed().as_secs_f64() * 1000.0);
        guard ^= scalia_core::planet::chunk_hash(&c);
    }
    let chunk_ms = median_ms(chunks);

    // GOAP: 4,096 squad plans from every start state; median per plan.
    let squad = scalia_ai::squad::actions();
    let mut plans = Vec::new();
    let mut steps = 0usize;
    for start in 0..4096u64 {
        let s = Instant::now();
        let p = scalia_ai::goap::plan(scalia_ai::goap::State(start), scalia_ai::squad::take_objective(), &squad, Default::default());
        plans.push(s.elapsed().as_secs_f64() * 1000.0);
        steps += p.map_or(0, |p| p.actions.len());
    }
    let plan_worst_ms = plans.iter().cloned().fold(0.0, f64::max);
    let plan_ms = median_ms(plans);

    // Navigation: a 400 m testing ground (800 x 800 cells) with 300 props, rebuilt; the budget is 1 ms per
    // 32 m tile, so the whole ground (156 tiles) may take 156 ms, but we hold it to the per-tile rate.
    use scalia_core::nav::{NavGrid, Prop, PropKind};
    let mut props = Vec::new();
    for k in 0..300i64 {
        let h = scalia_core::hash::mix64(k as u64);
        let (x, y) = ((h % 390_000) as i64 + 5_000, ((h >> 20) % 390_000) as i64 + 5_000);
        let kind = match k % 3 {
            0 => PropKind::Rock { radius_mm: 600 + (h >> 40) as i64 % 2_000 },
            1 => PropKind::Wall { length_mm: 4_000 + (h >> 40) as i64 % 12_000, thickness_mm: 400 },
            _ => PropKind::Crate { size_mm: 1_200 },
        };
        props.push(Prop { kind, x_mm: x, y_mm: y, quarter_turns: (k % 4) as u8 });
    }
    let mut grid = NavGrid::new(0, 0, 800, 800);
    let mut navs = Vec::new();
    for _ in 0..5 {
        let s = Instant::now();
        grid.rebuild(&props);
        navs.push(s.elapsed().as_secs_f64() * 1000.0);
    }
    let nav_tile_ms = median_ms(navs) / (800.0 * 800.0 / (64.0 * 64.0));
    // Route-finding: corner to corner across the 400 m ground (one group's route; soldiers share it).
    let mut routes = Vec::new();
    let mut legs = 0usize;
    for k in 0..10i64 {
        let s = Instant::now();
        let r = scalia_core::nav::find_path(&grid, (2_000 + k * 500, 2_000), (397_000, 396_000 - k * 700), 300);
        routes.push(s.elapsed().as_secs_f64() * 1000.0);
        legs += r.map_or(0, |r| r.len());
    }
    let route_ms = median_ms(routes);

    // Printing real state keeps the work observable and sanity-checks that ticks drained needs.
    let food: f64 = w.groups.needs(scalia_core::groups::GroupId(5)).food.to_f32() as f64;
    println!("sanity_group5_food={food:.4} after {} ticks", t / 1000);
    let verdict = |ms: f64, budget: f64| if ms <= budget { "PASS" } else { "FAIL" };
    let (v1, v2) = (verdict(tick_ms, TICK_BUDGET_MS), verdict(derive_ms, DERIVE_BUDGET_MS));
    let v3 = verdict(chunk_ms, CHUNK_BUDGET_MS);
    let v4 = verdict(plan_ms, PLAN_BUDGET_MS);
    let v5 = verdict(nav_tile_ms, 1.0);
    let v6 = verdict(route_ms, 20.0);
    println!("tick_1m_soldiers_ms={tick_ms:.3} budget=5 {v1}");
    println!("derive_20k_ms={derive_ms:.3} budget=2 {v2}");
    println!("chunk_64_ms={chunk_ms:.3} budget=1 {v3} (check {guard})");
    println!("goap_squad_plan_ms={plan_ms:.4} worst={plan_worst_ms:.4} budget={PLAN_BUDGET_MS} {v4} ({steps} steps over 4096 plans)");
    println!("nav_grid_per_tile_ms={nav_tile_ms:.4} budget=1 {v5} (fingerprint {:x})", grid.fingerprint());
    println!("route_400m_ms={route_ms:.3} budget=20 {v6} ({legs} waypoints over 10 routes)");
    if !report_only && (v1 == "FAIL" || v2 == "FAIL" || v3 == "FAIL" || v4 == "FAIL" || v5 == "FAIL" || v6 == "FAIL") {
        std::process::exit(1);
    }
}
