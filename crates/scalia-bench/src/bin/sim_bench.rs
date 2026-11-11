//! Budget checks: one tick of a million soldiers and deriving 20,000 individuals.

use scalia_core::derive::{derive_visible, Derived, ViewBox};
use scalia_core::position::RegionId;
use scalia_core::scenario::build_scenario;
use scalia_core::time::WorldMs;
use std::time::Instant;

const TICK_BUDGET_MS: f64 = 5.0;
const DERIVE_BUDGET_MS: f64 = 2.0;

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

    // Printing real state keeps the work observable and sanity-checks that ticks drained needs.
    let food: f64 = w.groups.needs(scalia_core::groups::GroupId(5)).food.to_f32() as f64;
    println!("sanity_group5_food={food:.4} after {} ticks", t / 1000);
    let verdict = |ms: f64, budget: f64| if ms <= budget { "PASS" } else { "FAIL" };
    let (v1, v2) = (verdict(tick_ms, TICK_BUDGET_MS), verdict(derive_ms, DERIVE_BUDGET_MS));
    println!("tick_1m_soldiers_ms={tick_ms:.3} budget=5 {v1}");
    println!("derive_20k_ms={derive_ms:.3} budget=2 {v2}");
    if !report_only && (v1 == "FAIL" || v2 == "FAIL") {
        std::process::exit(1);
    }
}
