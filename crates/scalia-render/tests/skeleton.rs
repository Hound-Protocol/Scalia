use glam::Vec3;
use scalia_render::mesh::{figure, part_of, Part};
use scalia_render::pose::{blend, idle, layer, params, pose_vertex, Pose};
use scalia_render::skeleton::{bone_of, gpu_rows, skin_matrices, BONES};

/// A vertex bound wholly to one bone lands exactly where the rigid rig puts it, for every part and pose.
#[test]
fn bones_reproduce_the_rigid_rig_exactly() {
    let verts: Vec<(Part, Vec3)> = figure().vertices.iter().map(|v| (part_of(v.color), Vec3::from(v.pos))).collect();
    let mut cases = Vec::new();
    for id in 0..=10u32 {
        for k in 0..4 {
            cases.push(params(Pose::from_id(id), k as f32 / 4.0));
        }
    }
    cases.push(idle(&params(Pose::Stand, 0.0), Pose::Stand, 7.3, 0.4));
    cases.push(blend(&params(Pose::Stand, 0.0), &params(Pose::Prone, 0.0), 0.5));
    cases.push(blend(&params(Pose::Stand, 0.0), &params(Pose::Tablet, 0.0), 0.4));
    cases.push(layer(&params(Pose::Walk, 0.3), &params(Pose::StandAim, 0.3)));
    for p in &cases {
        let m = skin_matrices(p);
        for (part, v) in &verts {
            let want = pose_vertex(*v, *part, p);
            let got = m[bone_of(*part) as usize].transform_point3(*v);
            assert!((want - got).length() < 1e-4, "{part:?}: rig {want:?}, bone {got:?} in {p:?}");
        }
    }
}

#[test]
fn the_gpu_gets_three_rows_per_bone() {
    let mut out = Vec::new();
    gpu_rows(&skin_matrices(&params(Pose::Walk, 0.2)), &mut out);
    assert_eq!(out.len(), BONES * 12);
    // Rest pose: the identity (apart from the stance's height shift), row by row.
    let mut rest = Vec::new();
    gpu_rows(&skin_matrices(&Default::default()), &mut rest);
    assert_eq!(&rest[12..24], &[1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
}
