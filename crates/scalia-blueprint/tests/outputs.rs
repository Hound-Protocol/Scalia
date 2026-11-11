use scalia_blueprint::{draw::svg, layout::ship_layout, parse};
use scalia_core::boarding::ShipLayout;

const TRANSPORT: &str = r#"(
    id: "ox_test", version: 1, name: "Ox Test", class: Transport,
    hull: (length_mm: 20000, width_mm: 8000, height_mm: 5000, mass_kg: 90000),
    stats: (speed_mm_s: 12000, crew: 6, passengers: 500, armour: 3),
    interior: (ramps: 2, doors: 4, corridor_width_cm: 120, seat_cols: 20, seats: 600, rooms: [
        (name: "bridge", purpose: Bridge, x_mm: -1500, y_mm: 7000, w_mm: 3000, d_mm: 2500),
        (name: "troops", purpose: Troops, x_mm: -3000, y_mm: -6000, w_mm: 6000, d_mm: 12000),
    ]),
    model: [Box(x: -3500, y: -9000, z: 0, w: 7000, d: 18000, h: 4000, color: 1)],
)"#;

const WALKER: &str = r#"(
    id: "walker_test", version: 1, name: "Walker", class: Walker,
    hull: (length_mm: 600, width_mm: 500, height_mm: 1800, mass_kg: 90),
    stats: (speed_mm_s: 1500, crew: 1, passengers: 0, armour: 1),
    interior: (ramps: 0, doors: 0, corridor_width_cm: 0, seat_cols: 0, seats: 0, rooms: []),
    model: [Box(x: -200, y: -150, z: 0, w: 400, d: 300, h: 1700, color: 1)],
)"#;

#[test]
fn transport_yields_layout_matching_interior() {
    let l = ship_layout(&parse(TRANSPORT).unwrap()).expect("transport has a layout");
    assert_eq!(l, ShipLayout { ramps: 2, doors: 4, corridor_width_cm: 120, seat_cols: 20 });
}

#[test]
fn walker_yields_no_layout() {
    assert!(ship_layout(&parse(WALKER).unwrap()).is_none());
}

#[test]
fn layout_throughput_is_nonzero_for_a_transport() {
    let l = ship_layout(&parse(TRANSPORT).unwrap()).unwrap();
    assert!(l.throughput_milli_per_s().unwrap() > 0);
}

#[test]
fn svg_contains_every_room() {
    let b = parse(TRANSPORT).unwrap();
    let s = svg(&b);
    assert!(s.starts_with("<svg"));
    assert_eq!(s.matches("data-room=").count(), b.interior.rooms.len());
    assert!(s.contains("data-room=\"bridge\"") && s.contains("data-room=\"troops\""));
}

#[test]
fn svg_is_deterministic() {
    let b = parse(TRANSPORT).unwrap();
    assert_eq!(svg(&b), svg(&b));
}

#[test]
fn svg_escapes_names() {
    let mut b = parse(TRANSPORT).unwrap();
    b.interior.rooms[0].name = "a<b&\"c\"".to_string();
    let s = svg(&b);
    assert!(s.contains("a&lt;b&amp;&quot;c&quot;"), "{s}");
    assert!(!s.contains("a<b"));
}
