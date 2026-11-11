use scalia_blueprint::{parse, triangle_budget, triangle_count, types::*, validate};

const OK: &str = r#"(
    id: "trooper_test",
    version: 1,
    name: "Test Trooper",
    class: Walker,
    hull: (length_mm: 600, width_mm: 500, height_mm: 1800, mass_kg: 90),
    stats: (speed_mm_s: 1500, crew: 1, passengers: 0, armour: 1),
    interior: (ramps: 0, doors: 0, corridor_width_cm: 0, seat_cols: 0, seats: 0, rooms: []),
    model: [Box(x: -200, y: -150, z: 0, w: 400, d: 300, h: 1700, color: 1)],
)"#;

fn ok() -> Blueprint {
    parse(OK).expect("base blueprint is valid")
}

fn field(b: &Blueprint) -> String {
    validate(b).expect_err("should be rejected").field
}

#[test]
fn valid_file_passes() {
    assert!(validate(&ok()).is_ok());
}

#[test]
fn zero_hull_length_rejected() {
    let mut b = ok();
    b.hull.length_mm = 0;
    assert_eq!(field(&b), "hull.length_mm");
}

#[test]
fn zero_hull_height_rejected() {
    let mut b = ok();
    b.hull.height_mm = 0;
    assert_eq!(field(&b), "hull.height_mm");
}

#[test]
fn empty_id_rejected() {
    let mut b = ok();
    b.id = String::new();
    assert_eq!(field(&b), "id");
}

#[test]
fn bad_id_characters_rejected() {
    let mut b = ok();
    b.id = "Tank 1".to_string();
    assert_eq!(field(&b), "id");
}

#[test]
fn version_zero_rejected() {
    let mut b = ok();
    b.version = 0;
    assert_eq!(field(&b), "version");
}

#[test]
fn room_outside_hull_rejected() {
    let mut b = ok();
    b.interior.rooms.push(Room {
        name: "bridge".into(),
        purpose: Purpose::Bridge,
        x_mm: 200,
        y_mm: 0,
        w_mm: 300,
        d_mm: 100,
    });
    assert_eq!(field(&b), "interior.rooms[0]");
}

#[test]
fn passengers_above_seats_rejected() {
    let mut b = ok();
    b.stats.passengers = 10;
    b.interior.seats = 9;
    assert_eq!(field(&b), "stats.passengers");
}

#[test]
fn over_budget_rejected() {
    let mut b = ok();
    let one = b.model[0];
    b.model = vec![one; 40]; // 40 * 12 = 480 > 400
    assert_eq!(triangle_count(&b), 480);
    assert_eq!(triangle_budget(Class::Walker), 400);
    assert_eq!(field(&b), "model");
}

#[test]
fn primitive_size_overflow_rejected() {
    let mut b = ok();
    b.model = vec![Prim::Box { x: i32::MAX, y: 0, z: 0, w: u32::MAX, d: 1, h: 1, color: 0 }];
    assert_eq!(field(&b), "model[0]");
}

#[test]
fn primitive_outside_hull_rejected() {
    let mut b = ok();
    b.model = vec![Prim::Box { x: 0, y: 0, z: 0, w: 400, d: 300, h: 1900, color: 0 }];
    assert_eq!(field(&b), "model[0]");
}

#[test]
fn no_primitives_rejected() {
    let mut b = ok();
    b.model.clear();
    assert_eq!(field(&b), "model");
}

#[test]
fn parse_runs_validation() {
    let bad = OK.replace("length_mm: 600", "length_mm: 0");
    assert_eq!(parse(&bad).unwrap_err().field, "hull.length_mm");
}

fn transport() -> Blueprint {
    let mut b = ok();
    b.class = Class::Transport;
    b.interior.seat_cols = 4;
    b.interior.ramps = 1;
    b.interior.doors = 1;
    b.interior.corridor_width_cm = 100;
    b
}

#[test]
fn transport_with_all_boarding_parts_passes() {
    assert!(validate(&transport()).is_ok());
}

#[test]
fn transport_without_ramps_rejected() {
    let mut b = transport();
    b.interior.ramps = 0;
    assert_eq!(field(&b), "interior.ramps");
}

#[test]
fn transport_without_doors_rejected() {
    let mut b = transport();
    b.interior.doors = 0;
    assert_eq!(field(&b), "interior.doors");
}

#[test]
fn ship_without_corridor_rejected() {
    let mut b = transport();
    b.class = Class::Ship;
    b.interior.corridor_width_cm = 0;
    assert_eq!(field(&b), "interior.corridor_width_cm");
}
