use scalia_blueprint::{mesh::{mesh, mesh_hash}, parse, triangle_count, types::*};

const BASE: &str = r#"(
    id: "mesh_test", version: 1, name: "Mesh Test", class: Tank,
    hull: (length_mm: 6000, width_mm: 3000, height_mm: 2400, mass_kg: 40000),
    stats: (speed_mm_s: 9000, crew: 3, passengers: 0, armour: 8),
    interior: (ramps: 0, doors: 0, corridor_width_cm: 0, seat_cols: 0, seats: 0, rooms: []),
    model: [Box(x: -1000, y: -2000, z: 0, w: 2000, d: 4000, h: 1000, color: 1)],
)"#;

fn with(model: Vec<Prim>) -> Blueprint {
    let mut b = parse(BASE).unwrap();
    b.model = model;
    b
}

fn bx() -> Prim {
    Prim::Box { x: -1000, y: -2000, z: 0, w: 2000, d: 4000, h: 1000, color: 1 }
}
fn wedge() -> Prim {
    Prim::Wedge { x: -1000, y: -2000, z: 1000, w: 2000, d: 1500, h: 600, color: 2 }
}
fn cyl() -> Prim {
    Prim::Cylinder { x: 0, y: 500, z: 1000, radius: 400, h: 1200, color: 3 }
}

#[test]
fn box_has_12_triangles_and_8_vertices() {
    let m = mesh(&with(vec![bx()]));
    assert_eq!(m.indices.len(), 36);
    assert_eq!(m.vertices.len(), 8);
    assert_eq!(m.colors.len(), 12);
}

#[test]
fn wedge_has_8_triangles() {
    let m = mesh(&with(vec![wedge()]));
    assert_eq!(m.indices.len(), 24);
    assert_eq!(m.colors.len(), 8);
}

#[test]
fn cylinder_has_28_triangles() {
    let m = mesh(&with(vec![cyl()]));
    assert_eq!(m.indices.len(), 84);
    assert_eq!(m.colors.len(), 28);
}

#[test]
fn triangle_count_matches_validate_count() {
    let b = with(vec![bx(), wedge(), cyl()]);
    let m = mesh(&b);
    assert_eq!(m.indices.len() / 3, triangle_count(&b) as usize);
    assert!(m.indices.iter().all(|&i| (i as usize) < m.vertices.len()));
}

#[test]
fn mesh_is_deterministic() {
    let b = with(vec![bx(), wedge(), cyl()]);
    assert_eq!(mesh(&b), mesh(&b));
    assert_eq!(mesh_hash(&mesh(&b)), mesh_hash(&mesh(&b)));
}

#[test]
fn different_models_hash_differently() {
    let a = mesh_hash(&mesh(&with(vec![bx()])));
    let c = mesh_hash(&mesh(&with(vec![bx(), cyl()])));
    assert_ne!(a, c);
}

#[test]
fn all_vertices_inside_hull() {
    let b = with(vec![bx(), wedge(), cyl()]);
    let (hw, hl, hh) = (b.hull.width_mm as i64, b.hull.length_mm as i64, b.hull.height_mm as i64);
    for v in mesh(&b).vertices {
        let (x, y, z) = (v[0] as i64, v[1] as i64, v[2] as i64);
        assert!(2 * x >= -hw && 2 * x <= hw, "x {x}");
        assert!(2 * y >= -hl && 2 * y <= hl, "y {y}");
        assert!(z >= 0 && z <= hh, "z {z}");
    }
}
