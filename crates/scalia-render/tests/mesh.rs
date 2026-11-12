use scalia_render::mesh::{figure, MeshBuilder};

#[test]
fn box_has_twelve_triangles() {
    let mut m = MeshBuilder::default();
    m.cuboid([0.0; 3], [1.0, 2.0, 3.0], 4.0);
    assert_eq!(m.vertices.len(), 36);
    assert!(m.vertices.iter().all(|v| v.color == 4.0));
}

#[test]
fn diamond_and_ring_counts() {
    let mut m = MeshBuilder::default();
    m.diamond([0.0; 3], 1.0, 2.0);
    assert_eq!(m.vertices.len(), 24);
    let mut r = MeshBuilder::default();
    r.ring([0.0, 0.0], 10.0, 1.0, 16, 3.0);
    assert_eq!(r.vertices.len(), 16 * 6);
}

#[test]
fn figure_is_small_and_uses_instance_colour_for_the_torso() {
    let f = figure();
    assert!(f.vertices.len() / 3 <= 64);
    assert!(f.vertices.iter().any(|v| v.color < 0.0));
    let top = f.vertices.iter().map(|v| v.pos[2]).fold(0.0, f32::max);
    assert!((1.6..=1.9).contains(&top), "height {top}");
}
