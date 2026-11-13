use scalia_ai::sight::SightGrid;
use scalia_ai::tactics::{Cover, CoverMap, Height};
use scalia_core::killhouse::Killhouse;
use scalia_core::nav::{NavGrid, Prop, PropKind};

fn bags() -> Prop {
    Prop { kind: PropKind::Wall { length_mm: 6_000, thickness_mm: 700 }, x_mm: 50_000, y_mm: 50_000, quarter_turns: 0 }
}
fn maps(props: &[Prop]) -> (NavGrid, SightGrid, CoverMap) {
    let mut nav = NavGrid::new(0, 0, 200, 200);
    nav.rebuild(props);
    let sight = SightGrid::build(0, 0, 400, 400, props);
    let cover = CoverMap::build(props, &nav);
    (nav, sight, cover)
}

#[test]
fn cover_lines_both_sides_of_a_sandbag_wall_on_walkable_ground() {
    let (nav, _, c) = maps(&[bags()]);
    let south = c.spots.iter().filter(|s| s.pos.1 < 50_000).count();
    let north = c.spots.iter().filter(|s| s.pos.1 > 50_000).count();
    assert!(south >= 4 && north >= 4, "{south} south, {north} north");
    assert!(c.spots.iter().all(|s| nav.walkable_at(s.pos.0, s.pos.1, 0)));
    assert!(c.spots.iter().all(|s| s.height == Height::Low), "sandbags are low cover");
}

#[test]
fn the_side_away_from_the_threat_protects_and_the_near_side_does_not() {
    let (_, sight, c) = maps(&[bags()]);
    let threat = (50_000, 80_000); // north
    let south = c.spots.iter().find(|s| s.pos.1 < 50_000 && (s.pos.0 - 50_000).abs() < 600).unwrap();
    let north = c.spots.iter().find(|s| s.pos.1 > 50_000 && (s.pos.0 - 50_000).abs() < 600).unwrap();
    assert!(c.protection(south, threat, &sight) > c.protection(north, threat, &sight));
    assert!(c.can_fire(south, threat, &sight), "can rise and shoot over the bags");
}

#[test]
fn best_cover_is_near_and_actually_protects() {
    let (_, sight, c) = maps(&[bags()]);
    let me = (49_000, 46_000); // south of the bags, in the open
    let i = c.best(me, &[(50_000, 80_000)], 15_000, &sight, &[]).expect("cover in reach");
    let s: &Cover = &c.spots[i];
    assert!(s.pos.1 < 50_000, "behind the bags from the threat: {:?}", s.pos);
    // A taken spot is not offered again.
    let j = c.best(me, &[(50_000, 80_000)], 15_000, &sight, &[i]).unwrap();
    assert_ne!(i, j);
}

#[test]
fn panel_ends_are_corners_to_lean_out_of() {
    let wall = Prop { kind: PropKind::Panel { length_mm: 4_000, window: false }, x_mm: 50_000, y_mm: 50_000, quarter_turns: 0 };
    let (_, _, c) = maps(&[wall]);
    assert!(c.spots.iter().all(|s| s.height == Height::High));
    assert!(c.spots.iter().any(|s| s.corner.is_some()), "spots at the ends lean out");
    assert!(c.spots.iter().any(|s| s.corner.is_none()), "spots in the middle do not");
}

#[test]
fn the_killhouse_gives_stack_points_and_a_clearing_order() {
    let k = Killhouse::new(80_000, 62_000);
    let mut nav = NavGrid::new(60_000, 40_000, 100, 100);
    nav.rebuild(&k.props());
    let plan = scalia_ai::tactics::Building::of(&k);
    for e in &plan.entries {
        assert_eq!(e.stack.len(), 4, "four men stack at a door");
        for &p in &e.stack {
            assert!(nav.walkable_at(p.0, p.1, 0), "stack point {p:?} walkable");
            assert!(k.room_at(p.0, p.1).is_none(), "stack point {p:?} outside");
        }
        let order = plan.clear_order(e.door);
        assert_eq!(order.len(), k.rooms.len(), "every room once");
        assert_eq!(Some(order[0]), k.doors[e.door].rooms.0.or(k.doors[e.door].rooms.1), "first the room behind the entry");
    }
    assert!(plan.entries.len() >= 2);
}
