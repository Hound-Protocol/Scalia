use scalia_core::killhouse::{Killhouse, Opening};
use scalia_core::nav::{NavGrid, PropKind};

fn house() -> Killhouse {
    Killhouse::new(80_000, 62_000)
}

#[test]
fn four_rooms_all_reachable_from_outside_through_doors() {
    let k = house();
    assert_eq!(k.rooms.len(), 4);
    // Breadth-first over doors from "outside" (None) reaches every room.
    let mut seen = vec![false; k.rooms.len()];
    let mut frontier: Vec<Option<usize>> = vec![None];
    while let Some(r) = frontier.pop() {
        for d in &k.doors {
            for (a, b) in [(d.rooms.0, d.rooms.1), (d.rooms.1, d.rooms.0)] {
                if a == r {
                    if let Some(i) = b {
                        if !seen[i] { seen[i] = true; frontier.push(Some(i)); }
                    }
                }
            }
        }
    }
    assert!(seen.iter().all(|&s| s), "every room reachable: {seen:?}");
    assert!(k.doors.iter().filter(|d| d.rooms.0.is_none() || d.rooms.1.is_none()).count() >= 2, "at least two ways in");
}

#[test]
fn walls_block_walking_and_doorways_are_open() {
    let k = house();
    let mut g = NavGrid::new(70_000, 52_000, 80, 68);
    g.rebuild(&k.props());
    for d in &k.doors {
        assert!(g.walkable_at(d.x_mm, d.y_mm, 0), "doorway at ({}, {}) open", d.x_mm, d.y_mm);
    }
    // The middle of the south wall between its openings is solid.
    assert!(!g.walkable_at(80_000 + 8_000, 62_000, 0));
    // Windows are panels with an opening: they block walking.
    let w = k.windows[0];
    assert!(!g.walkable_at(w.x_mm, w.y_mm, 0));
    assert!(k.props().iter().any(|p| matches!(p.kind, PropKind::Panel { window: true, .. })));
}

#[test]
fn every_room_centre_is_inside_its_rectangle_and_openings_are_on_walls() {
    let k = house();
    for r in &k.rooms {
        let (cx, cy) = r.centre();
        assert!(r.contains(cx, cy));
    }
    for o in k.doors.iter().map(|d| Opening { x_mm: d.x_mm, y_mm: d.y_mm, along_x: d.along_x }).chain(k.windows.iter().copied()) {
        assert!(k.on_wall(o.x_mm, o.y_mm), "opening ({}, {}) sits on a wall line", o.x_mm, o.y_mm);
    }
}
