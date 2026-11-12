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
    assert!(f.vertices.len() / 3 <= 2000, "{} triangles", f.vertices.len() / 3);
    assert!(f.vertices.iter().any(|v| scalia_render::mesh::palette_of(v.color) == scalia_render::mesh::TEAM as u32), "team colour somewhere");
    let top = f.vertices.iter().map(|v| v.pos[2]).fold(0.0, f32::max);
    assert!((1.6..=1.9).contains(&top), "height {top}");
    assert!(scalia_render::mesh::figure_lo().vertices.len() / 3 <= 160);
}

#[test]
fn figure_has_two_legs_and_two_arms_that_can_swing() {
    let f = figure();
    for part in [Part::LeftThigh, Part::RightThigh, Part::LeftShin, Part::RightShin, Part::LeftArm, Part::RightArm, Part::LeftForearm, Part::RightForearm, Part::Head, Part::Rifle] {
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
    assert_eq!(part_of(3.0 + 32.0 * 2.0), Part::RightThigh);
    assert_eq!(part_of(32.0 * 9.0), Part::Rifle);
    assert_eq!(part_of(5.0 + 32.0 * 4.0), Part::RightArm);
    assert_eq!(part_of(32.0 * 10.0), Part::Head);
    assert_eq!(part_of(2.0 + 32.0 * 11.0), Part::LeftForearm);
    assert_eq!(part_of(32.0 * 12.0), Part::RightForearm);
    // Palette indices above 15 (the gear colours) survive the round trip too.
    assert_eq!(part_of(scalia_render::mesh::code(27, Part::Torso)), Part::Torso);
    assert_eq!(scalia_render::mesh::palette_of(scalia_render::mesh::code(27, Part::Torso)), 27);
}

#[test]
fn grass_tuft_is_tiny_and_sways() {
    let g = scalia_render::mesh::grass_tuft();
    assert!(g.vertices.len() / 3 <= 6);
    assert!(g.vertices.iter().all(|v| part_of(v.color) == Part::Sway));
    let top = g.vertices.iter().map(|v| v.pos[2]).fold(0.0, f32::max);
    assert!((0.3..=0.6).contains(&top));
}

#[test]
fn figure_carries_gear_and_forearms_hang_from_the_elbows() {
    let f = figure();
    assert!(f.vertices.len() / 3 >= 700, "only {} triangles", f.vertices.len() / 3);
    let span = |p: Part| {
        let z = f.vertices.iter().filter(|v| part_of(v.color) == p).map(|v| v.pos[2]);
        (z.clone().fold(f32::MAX, f32::min), z.fold(f32::MIN, f32::max))
    };
    let (lo, hi) = span(Part::LeftForearm);
    assert!(lo < 0.92 && (1.1..=1.2).contains(&hi), "forearm {lo}..{hi}");
    let (lo, _) = span(Part::LeftArm);
    assert!((1.08..=1.16).contains(&lo), "upper arm starts at {lo}");
    let (lo, hi) = span(Part::Head);
    assert!(lo > 1.38 && hi > 1.7, "head {lo}..{hi}");
}

#[test]
fn far_figure_is_a_handful_of_triangles_with_the_same_height() {
    let f = scalia_render::mesh::figure_far();
    assert!(f.vertices.len() / 3 <= 24, "{} triangles", f.vertices.len() / 3);
    let top = f.vertices.iter().map(|v| v.pos[2]).fold(0.0, f32::max);
    assert!((1.6..=1.8).contains(&top), "height {top}");
    assert!(f.vertices.iter().any(|v| scalia_render::mesh::palette_of(v.color) == scalia_render::mesh::TEAM as u32), "helmet takes the team colour");
}
