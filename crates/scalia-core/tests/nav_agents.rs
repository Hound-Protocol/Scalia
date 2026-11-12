use scalia_core::nav::{FlowField, NavGrid, Prop, PropKind};
use scalia_core::nav_agents::AgentGroup;

const SOLDIER: i64 = 300;

fn walled() -> NavGrid {
    // 60 m x 60 m; a wall across y = 30 m from x = 0 to 45 m, gap from 45 m to 60 m.
    let mut g = NavGrid::new(0, 0, 120, 120);
    g.rebuild(&[Prop { kind: PropKind::Wall { length_mm: 45_000, thickness_mm: 600 }, x_mm: 22_500, y_mm: 30_000, quarter_turns: 0 }]);
    g
}

#[test]
fn flow_field_counts_down_to_the_goal_and_routes_round_walls() {
    let g = walled();
    let f = FlowField::new(&g, (10_000, 50_000), SOLDIER).expect("goal is open");
    assert_eq!(f.cost_at(10_000, 50_000), Some(0));
    assert_eq!(f.cost_at(20_000, 30_000), None, "the wall is unreachable");
    // From under the wall's middle, the way leads east towards the gap, not straight north into the wall.
    let (dx, _) = f.direction_at(20_000, 27_000).expect("a direction");
    assert!(dx > 0, "heads for the gap");
}

#[test]
fn a_squad_walks_round_the_wall_to_its_slots_without_overlapping_or_clipping() {
    let g = walled();
    let starts: Vec<(i64, i64)> = (0..36).map(|k| (8_000 + (k % 6) * 1_000, 8_000 + (k / 6) * 1_000)).collect();
    let mut squad = AgentGroup::new(&g, starts, (12_000, 48_000), 1_500, SOLDIER).expect("an order");
    for _ in 0..20 * 120 {
        squad.step(&g, 50);
        for &(x, y) in squad.positions() {
            assert!(g.walkable_at(x, y, SOLDIER - 100), "a soldier clipped into the wall at ({x}, {y})");
        }
        if squad.all_arrived() {
            break;
        }
    }
    assert!(squad.all_arrived(), "everyone reaches a slot in two minutes");
    let p = squad.positions();
    for i in 0..p.len() {
        for j in i + 1..p.len() {
            let d = (((p[i].0 - p[j].0) as f64).powi(2) + ((p[i].1 - p[j].1) as f64).powi(2)).sqrt();
            assert!(d >= 450.0, "soldiers {i} and {j} overlap ({d:.0} mm)");
        }
    }
}

#[test]
fn agents_are_deterministic() {
    let g = walled();
    let starts: Vec<(i64, i64)> = (0..20).map(|k| (5_000 + k * 900, 10_000)).collect();
    let run = || {
        let mut s = AgentGroup::new(&g, starts.clone(), (40_000, 50_000), 1_500, SOLDIER).unwrap();
        for _ in 0..400 {
            s.step(&g, 50);
        }
        s.positions().to_vec()
    };
    assert_eq!(run(), run());
}

#[test]
fn an_order_onto_an_obstacle_goes_to_the_nearest_open_ground() {
    let mut g = NavGrid::new(0, 0, 80, 80);
    g.rebuild(&[Prop { kind: PropKind::Crate { size_mm: 3_000 }, x_mm: 20_000, y_mm: 20_000, quarter_turns: 0 }]);
    let starts: Vec<(i64, i64)> = (0..9).map(|k| (4_000 + (k % 3) * 1_000, 4_000 + (k / 3) * 1_000)).collect();
    let mut squad = AgentGroup::new(&g, starts, (20_000, 20_000), 1_500, SOLDIER).expect("an order even onto a crate");
    for _ in 0..20 * 60 {
        squad.step(&g, 50);
        if squad.all_arrived() {
            break;
        }
    }
    assert!(squad.all_arrived());
    for &(x, y) in squad.positions() {
        assert!(g.walkable_at(x, y, SOLDIER), "nobody stands in the crate: ({x}, {y})");
        let d = (((x - 20_000) as f64).powi(2) + ((y - 20_000) as f64).powi(2)).sqrt();
        assert!(d < 6_000.0, "they gather round it ({d:.0} mm away)");
    }
}
