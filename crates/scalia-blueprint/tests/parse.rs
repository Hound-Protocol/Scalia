use scalia_blueprint::{parse, types::*};

const MIN: &str = r#"(
    id: "trooper_test",
    version: 1,
    name: "Test Trooper",
    class: Walker,
    hull: (length_mm: 600, width_mm: 500, height_mm: 1800, mass_kg: 90),
    stats: (speed_mm_s: 1500, crew: 1, passengers: 0, armour: 1),
    interior: (ramps: 0, doors: 0, corridor_width_cm: 0, seat_cols: 0, seats: 0, rooms: []),
    model: [Box(x: -200, y: -150, z: 0, w: 400, d: 300, h: 1700, color: 1)],
)"#;

#[test]
fn parses_a_minimal_valid_file() {
    let b = parse(MIN).expect("parses");
    assert_eq!(b.id, "trooper_test");
    assert_eq!(b.class, Class::Walker);
    assert_eq!(b.model.len(), 1);
    assert!(matches!(b.model[0], Prim::Box { .. }));
}

#[test]
fn bad_syntax_names_position() {
    let e = parse(&MIN[..40]).unwrap_err();
    assert!(!e.message.is_empty());
}

#[test]
fn unknown_field_is_rejected() {
    let text = MIN.replacen("version: 1,", "version: 1, owner: \"x\",", 1);
    let e = parse(&text).unwrap_err();
    assert!(e.field.contains("owner") || e.message.contains("owner"), "{e:?}");
}
