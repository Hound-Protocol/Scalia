//! AN-04: the prone poses lie on the ground as drawn (the two-handed grip on top): boots, forearms and belly on it,
//! nothing pushed through it, the support elbow down under the rifle when aiming.
use glam::Vec3;
use scalia_render::mesh::{figure, part_of, Part};
use scalia_render::pose::{grip, params, pose_mesh_vertex, two_handed, Pose};

/// Lowest skin point per group: boots (L, R), arms (L, R), belly and thighs; and the rifle.
fn lows(pose: Pose) -> ([f32; 5], f32) {
    let p = grip(&params(pose, 0.0), two_handed(pose));
    let mut l = [f32::MAX; 5];
    let mut rifle = f32::MAX;
    for v in &figure().vertices {
        let z = pose_mesh_vertex(Vec3::from(v.pos), v.color, &p).z;
        let k = match part_of(v.color) {
            Part::LeftFoot | Part::LeftShin => 0,
            Part::RightFoot | Part::RightShin => 1,
            Part::LeftArm | Part::LeftForearm => 2,
            Part::RightArm | Part::RightForearm => 3,
            Part::Torso | Part::Body | Part::LeftThigh | Part::RightThigh => 4,
            Part::Rifle => {
                rifle = rifle.min(z);
                continue;
            }
            _ => continue,
        };
        l[k] = l[k].min(z);
    }
    (l, rifle)
}

/// Lying at rest and lying aiming, the arms rest on the ground (from 2.5 cm into soft ground to 3.5 cm above it), the
/// belly lies on it (within 1 cm), the boots press no more than 1.5 cm in, and the rifle is clear of it. Before AN-04
/// the right forearm sank 3.5 cm and the boots 2.5 cm at rest and, aiming, the support elbow hung 7 cm in the air.
#[test]
fn prone_lies_on_the_ground() {
    for pose in [Pose::Prone, Pose::ProneAim] {
        let (l, rifle) = lows(pose);
        for (k, what) in ["left boot", "right boot"].iter().enumerate() {
            assert!(l[k] > -0.015, "{pose:?}: the {what} sinks {:.3} m", -l[k]);
        }
        for (k, what) in [(2, "left arm"), (3, "right arm")] {
            assert!((-0.025..0.035).contains(&l[k]), "{pose:?}: the {what}'s lowest point is {:.3} m", l[k]);
        }
        assert!((-0.01..0.01).contains(&l[4]), "{pose:?}: the belly's lowest point is {:.3} m", l[4]);
        assert!(rifle > 0.01, "{pose:?}: the rifle is {rifle:.3} m off the ground");
    }
}
