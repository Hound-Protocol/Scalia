//! AN-03: the baked soldier skin and clips load, hang together and stand on the ground.
use glam::Vec3;
use scalia_render::skin::{load_mesh, skin_vertex, Anims, ANIM_BYTES, SKIN_BYTES};

fn posed(a: &Anims, clip: &str, t: f32) -> Vec<Vec3> {
    let c = a.clip(clip).unwrap_or_else(|| panic!("no clip {clip}"));
    let skin = a.skinning(&a.world(&a.sample(c, t)));
    load_mesh(SKIN_BYTES)
        .iter()
        .map(|v| skin_vertex(v, &skin))
        .collect()
}

#[test]
fn mesh_and_skeleton_hang_together() {
    let (a, m) = (Anims::load(ANIM_BYTES), load_mesh(SKIN_BYTES));
    assert!(
        m.len() % 3 == 0 && m.len() > 3000,
        "triangle list of {} vertices",
        m.len()
    );
    for (j, joint) in a.joints.iter().enumerate() {
        assert!(
            joint.parent < j as i32,
            "{}: parent must come first",
            joint.name
        );
    }
    for v in &m {
        assert_eq!(v.weights.iter().map(|&w| w as u32).sum::<u32>(), 255);
        assert!(v.joints.iter().all(|&j| (j as usize) < a.joints.len()));
    }
}

#[test]
fn bind_pose_skins_to_itself() {
    // Skinning with the joints where the bind pose has them leaves every vertex where it is.
    let (a, m) = (Anims::load(ANIM_BYTES), load_mesh(SKIN_BYTES));
    let bind: Vec<_> = a
        .joints
        .iter()
        .map(|j| scalia_render::skin::Xf {
            rot: j.bind_rot,
            pos: j.bind_pos,
        })
        .collect();
    let skin = a.skinning(&bind);
    let worst = m
        .iter()
        .map(|v| skin_vertex(v, &skin).distance(Vec3::from(v.pos)))
        .fold(0.0, f32::max);
    assert!(worst < 1e-4, "bind pose moves a vertex {worst} m");
}

#[test]
fn soldiers_stand_and_lie_on_the_ground() {
    let a = Anims::load(ANIM_BYTES);
    for (clip, lo, hi) in [
        ("idle", 1.70, 1.85),
        ("walk_f", 1.5, 1.85),
        ("prone", 0.2, 0.55),
        ("crawl_f", 0.2, 0.6),
    ] {
        let c = a.clip(clip).unwrap();
        for k in 0..8 {
            let v = posed(&a, clip, a.clips[c].seconds * k as f32 / 8.0);
            let (bottom, top) = v
                .iter()
                .fold((f32::MAX, f32::MIN), |(b, t), p| (b.min(p.z), t.max(p.z)));
            assert!(
                bottom.abs() < 0.08,
                "{clip} at {k}/8: lowest point {bottom:.3} m"
            );
            assert!((lo..hi).contains(&top), "{clip} at {k}/8: top {top:.2} m");
        }
    }
}

#[test]
fn loops_close_and_travel_is_taken_out() {
    // A loop's last frame leads into its first: the joints and the root barely move across the seam.
    let a = Anims::load(ANIM_BYTES);
    for c in a.clips.iter().enumerate().filter(|(_, c)| c.looped) {
        let (i, info) = c;
        let s0 = a.sample(i, 0.0);
        let s1 = a.sample(i, info.seconds - 0.5 / info.fps);
        let root = s0[0].pos.distance(s1[0].pos);
        assert!(
            root < 0.12,
            "{}: root jumps {root:.3} m over the seam",
            info.name
        );
    }
    let walk = &a.clips[a.clip("walk_f").unwrap()];
    assert!(
        walk.travel.y > 1.5 && walk.travel.x.abs() < 0.05,
        "walk_f travels {:?}",
        walk.travel
    );
}

#[test]
fn one_joint_chain_matches_the_whole_pose_and_the_muzzle_points_ahead() {
    use scalia_render::skin::{joint_at, muzzle, pose_instance};
    let a = Anims::load(ANIM_BYTES);
    let rh = a.joints.iter().position(|j| j.name == "RightHand").unwrap();
    // Aiming, standing still: ib = (heading, phase, cadence, pose), ic = (previous pose, changed, layer, back).
    let rec = [
        0.0, 0.0, 0.0, 1.0, 0.0, 0.3, 0.0, 6.0, 6.0, -1000.0, 0.0, 0.0,
    ];
    for time in [0.0, 1.3, 7.7] {
        let skin = pose_instance(&a, &rec, time);
        let bind = scalia_render::skin::Xf {
            rot: a.joints[rh].bind_rot,
            pos: a.joints[rh].bind_pos,
        };
        let full = skin[rh].mul(bind);
        let one = joint_at(&a, &rec, time, rh);
        assert!(
            full.pos.distance(one.pos) < 1e-4,
            "chain {:?} vs pose {:?}",
            one.pos,
            full.pos
        );
        let (tip, bore) = muzzle(&a, &rec, time);
        assert!(
            tip.y > 0.4 && (1.0..1.6).contains(&tip.z),
            "aiming muzzle at {tip:?}"
        );
        assert!(bore.y > 0.85, "aiming bore {bore:?}");
    }
}
