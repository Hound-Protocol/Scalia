//! Joint positions through the crawl cycle (figure frame: +y ahead, +x his right, z up; m), for checking the stroke
//! by numbers: `cargo run -q --release -p scalia-render --example crawl_joints [dir]`.
//! Per phase: each elbow and palm relative to its shoulder (ahead, out to the side, height), each knee's height and
//! sideways reach, the head and pelvis height, and the lowest point of each part group.
use glam::Vec3;
use scalia_render::contact::Body;
use scalia_render::mesh::Part;
use scalia_render::pose::{params_dir, pose_rigid, Pose, ELBOW_X, ELBOW_Z, HIP_X, HIP_Z, KNEE_Z, NECK_Z, PALM, SHOULDER_X, SHOULDER_Z};

fn main() {
    let dir: f32 = std::env::args().nth(1).map_or(0.0, |s| s.parse().unwrap());
    let body = Body::new();
    println!("phase | L elbow(ahead,out,z) L palm(ahead,out,z) | R elbow R palm | knees z/out L R | head z pelvis z | low: knee elbow rifle head");
    for k in 0..16 {
        let ph = k as f32 / 16.0;
        let p = params_dir(Pose::Crawl, ph, dir);
        let mut row = format!("{ph:.3} |");
        for (g, arm, fore) in [(-1.0, Part::LeftArm, Part::LeftForearm), (1.0, Part::RightArm, Part::RightForearm)] {
            let sh = pose_rigid(Vec3::new(g * SHOULDER_X, 0.0, SHOULDER_Z), arm, &p);
            let el = pose_rigid(Vec3::new(g * ELBOW_X, 0.0, ELBOW_Z), fore, &p);
            let pa = pose_rigid(Vec3::new(g * PALM.x, PALM.y, PALM.z), fore, &p);
            let rel = |v: Vec3| format!("{:+.2},{:+.2},{:.2}", v.y - sh.y, (v.x - sh.x) * g, v.z);
            row += &format!(" {} {} |", rel(el), rel(pa));
        }
        for (g, sh) in [(-1.0, Part::LeftShin), (1.0, Part::RightShin)] {
            let kn = pose_rigid(Vec3::new(g * HIP_X, 0.0, KNEE_Z), sh, &p);
            let hip = pose_rigid(Vec3::new(g * HIP_X, 0.0, HIP_Z), Part::Torso, &p);
            row += &format!(" {:.2}/{:+.2}", kn.z, (kn.x - hip.x) * g);
        }
        let head = pose_rigid(Vec3::new(0.0, 0.0, NECK_Z + 0.12), Part::Head, &p);
        let pel = pose_rigid(Vec3::new(0.0, 0.0, HIP_Z), Part::Torso, &p);
        let s = body.stats(&p);
        row += &format!(" | {:.2} {:.2} | {:+.3} {:+.3} {:+.3} {:+.3}", head.z, pel.z, s.knee, s.elbow, s.rifle, s.head);
        println!("{row}");
    }
}
