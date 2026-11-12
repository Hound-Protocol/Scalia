use scalia_render::mesh::{figure, part_of, MeshBuilder, Part};

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
    assert!(f.vertices.len() / 3 <= 260, "{} triangles", f.vertices.len() / 3);
    assert!(f.vertices.iter().any(|v| v.color < 0.0));
    let top = f.vertices.iter().map(|v| v.pos[2]).fold(0.0, f32::max);
    assert!((1.6..=1.9).contains(&top), "height {top}");
    assert!(scalia_render::mesh::figure_lo().vertices.len() / 3 <= 120);
}

#[test]
fn figure_has_two_legs_and_two_arms_that_can_swing() {
    let f = figure();
    for part in [Part::LeftThigh, Part::RightThigh, Part::LeftShin, Part::RightShin, Part::LeftArm, Part::RightArm, Part::Rifle] {
        let n = f.vertices.iter().filter(|v| part_of(v.color) == part).count();
        assert!(n >= 36, "{part:?} has {n} vertices");
    }
    // Legs hang from the hips, arms from the shoulders.
    let top_of = |p: Part| f.vertices.iter().filter(|v| part_of(v.color) == p).map(|v| v.pos[2]).fold(0.0, f32::max);
    assert!((0.8..=0.95).contains(&top_of(Part::LeftThigh)));
    assert!((0.4..=0.5).contains(&top_of(Part::LeftShin)));
    assert!((1.35..=1.5).contains(&top_of(Part::RightArm)));
}

#[test]
fn part_encoding_round_trips() {
    assert_eq!(part_of(-1.0), Part::Torso);
    assert_eq!(part_of(3.0), Part::Body);
    assert_eq!(part_of(3.0 + 16.0 * 2.0), Part::RightThigh);
    assert_eq!(part_of(16.0 * 9.0), Part::Rifle);
    assert_eq!(part_of(5.0 + 16.0 * 4.0), Part::RightArm);
}

#[test]
fn grass_tuft_is_tiny_and_sways() {
    let g = scalia_render::mesh::grass_tuft();
    assert!(g.vertices.len() / 3 <= 6);
    assert!(g.vertices.iter().all(|v| part_of(v.color) == Part::Sway));
    let top = g.vertices.iter().map(|v| v.pos[2]).fold(0.0, f32::max);
    assert!((0.3..=0.6).contains(&top));
}
