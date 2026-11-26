//! The crawl in the world, for judging whether it reads as a body moving or a figure sliding:
//! `cargo run -q --release -p scalia-render --example crawl_motion [rows]`.
//! The root advances CRAWL_STRIDE per cycle at a constant speed (as the game moves him). Per phase: pelvis, chest and
//! head forward speed (as a share of the root's), their heights (cm above their lowest), each side's hip and shoulder
//! height (roll: which side is up), and each planted boot and hand's slip along the ground (cm since it landed).
//! Ends with the spans: speed range, height range, the worst slip. With TRACK set, prints each hand's world track.
use glam::Vec3;
use scalia_render::crawl;
use scalia_render::mesh::Part;
use scalia_render::pose::{params_dir, pose_rigid, Pose, CRAWL_STRIDE, PALM, HIP_X, HIP_Z, ANKLE_Z, NECK_Z, SHOULDER_X, SHOULDER_Z};

const N: usize = 64;

fn main() {
    let rows = std::env::args().nth(1).is_some_and(|a| a == "rows");
    let world = |k: usize, v: Vec3| v + Vec3::new(0.0, CRAWL_STRIDE * k as f32 / N as f32, 0.0);
    let mut pel = vec![];
    let mut chest = vec![];
    let mut head = vec![];
    let mut roll = vec![];
    let mut knees = vec![];
    let mut hands = vec![];
    for k in 0..=N {
        let ph = k as f32 / N as f32;
        let p = params_dir(Pose::Crawl, ph, 0.0);
        let mid = |x: f32, z: f32, part: Part| (pose_rigid(Vec3::new(-x, 0.0, z), part, &p) + pose_rigid(Vec3::new(x, 0.0, z), part, &p)) / 2.0;
        pel.push(world(k, mid(HIP_X, HIP_Z, Part::Torso)));
        chest.push(world(k, mid(SHOULDER_X, SHOULDER_Z, Part::Torso)));
        head.push(world(k, pose_rigid(Vec3::new(0.0, 0.0, NECK_Z + 0.12), Part::Head, &p)));
        let hz = |g: f32| pose_rigid(Vec3::new(g * HIP_X, 0.0, HIP_Z), Part::Torso, &p).z;
        let sz = |g: f32| pose_rigid(Vec3::new(g * SHOULDER_X, 0.0, SHOULDER_Z), Part::Torso, &p).z;
        roll.push((hz(-1.0) - hz(1.0), sz(-1.0) - sz(1.0)));
        knees.push([
            world(k, pose_rigid(Vec3::new(-HIP_X, 0.0, ANKLE_Z), Part::LeftShin, &p)),
            world(k, pose_rigid(Vec3::new(HIP_X, 0.0, ANKLE_Z), Part::RightShin, &p)),
        ]);
        hands.push([
            world(k, pose_rigid(Vec3::new(-PALM.x, PALM.y, PALM.z), Part::LeftForearm, &p)),
            world(k, pose_rigid(Vec3::new(PALM.x, PALM.y, PALM.z), Part::RightForearm, &p)),
        ]);
    }
    let v_root = CRAWL_STRIDE / N as f32;
    let speed = |s: &[Vec3], k: usize| (s[(k + 1).min(N)].y - s[k.saturating_sub(1)].y) / (((k + 1).min(N) - k.saturating_sub(1)) as f32 * v_root);
    let low = |s: &[Vec3]| s.iter().map(|v| v.z).fold(f32::MAX, f32::min);
    let (lp, lc, lh) = (low(&pel), low(&chest), low(&head));
    // Slip: while a contact is planted, how far it has moved along the ground since it landed.
    let mut slip = [[0.0f32; 2]; 2];
    let mut land: [[Option<Vec3>; 2]; 2] = [[None; 2]; 2];
    let mut slips = vec![[[0.0f32; 2]; 2]; N + 1];
    for k in 0..=N {
        let ph = k as f32 / N as f32;
        for i in 0..2 {
            let kp = crawl::planted(ph + if i == 0 { 0.0 } else { 0.5 });
            let ep = crawl::arm_planted(crawl::arm_u(ph, i));
            for (c, (on, at)) in [(kp, knees[k][i]), (ep, hands[k][i])].into_iter().enumerate() {
                if on {
                    let l = *land[c][i].get_or_insert(at);
                    let d = (at.truncate() - l.truncate()).length();
                    slips[k][c][i] = d;
                    slip[c][i] = slip[c][i].max(d);
                } else {
                    land[c][i] = None;
                }
            }
        }
    }
    if std::env::var("TRACK").is_ok() {
        for k in 0..=N {
            let e = hands[k];
            println!("{:.3} L {:+.3} {:+.3} {:.3}  R {:+.3} {:+.3} {:.3}", k as f32 / N as f32, e[0].x, e[0].y, e[0].z, e[1].x, e[1].y, e[1].z);
        }
    }
    if rows {
        println!("phase | speed pel chest head | height cm pel chest head | roll cm hips(L-R) sh(L-R) | slip cm bootL bootR handL handR");
        for k in (0..N).step_by(2) {
            let s = slips[k];
            println!(
                "{:.3} | {:.2} {:.2} {:.2} | {:4.1} {:4.1} {:4.1} | {:+5.1} {:+5.1} | {:3.1} {:3.1} {:3.1} {:3.1}",
                k as f32 / N as f32,
                speed(&pel, k),
                speed(&chest, k),
                speed(&head, k),
                (pel[k].z - lp) * 100.0,
                (chest[k].z - lc) * 100.0,
                (head[k].z - lh) * 100.0,
                roll[k].0 * 100.0,
                roll[k].1 * 100.0,
                s[0][0] * 100.0,
                s[0][1] * 100.0,
                s[1][0] * 100.0,
                s[1][1] * 100.0
            );
        }
    }
    let span = |f: &dyn Fn(usize) -> f32| {
        let v: Vec<f32> = (0..N).map(f).collect();
        (v.iter().copied().fold(f32::MAX, f32::min), v.iter().copied().fold(f32::MIN, f32::max))
    };
    for (what, s) in [("pelvis", &pel), ("chest", &chest), ("head", &head)] {
        let (a, b) = span(&|k| speed(s, k));
        let (c, d) = span(&|k| s[k].z);
        println!("{what:6} speed {a:.2}..{b:.2} x root, height span {:.1} cm", (d - c) * 100.0);
    }
    println!("slip max cm: boot L {:.1} R {:.1}, hand L {:.1} R {:.1}", slip[0][0] * 100.0, slip[0][1] * 100.0, slip[1][0] * 100.0, slip[1][1] * 100.0);
}
