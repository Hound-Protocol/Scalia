//! Per phase of a gait: the body height shift and how low each foot gets (debug aid for baked gaits):
//! `cargo run -p scalia-render --example gait_trace -- <pose id> [dir]`.
use glam::Vec3;
use scalia_render::mesh::Part;
use scalia_render::pose::{ground_drop_side, params_dir, pose_vertex, Pose, ANKLE_Z, HIP_X, HIP_Z};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let pose = Pose::from_id(a[1].parse().unwrap());
    let dir: f32 = a.get(2).map_or(0.0, |d| d.parse().unwrap());
    let n = 48;
    let ank_at = |k: usize, px: f32, part| pose_vertex(Vec3::new(px, 0.0, ANKLE_Z), part, &params_dir(pose, (k % n) as f32 / n as f32, dir));
    for k in 0..n {
        let p = params_dir(pose, k as f32 / n as f32, dir);
        let vl = (ank_at(k + 1, -HIP_X, Part::LeftShin) - ank_at(k, -HIP_X, Part::LeftShin)) * n as f32;
        let vr = (ank_at(k + 1, HIP_X, Part::RightShin) - ank_at(k, HIP_X, Part::RightShin)) * n as f32;
        print!("vL {:+.2} {:+.2} vR {:+.2} {:+.2} ", vl.x, vl.y, vr.x, vr.y);
        let l = -ground_drop_side([p.hip[0]; 2], [p.knee[0]; 2], [p.side[0]; 2], p.flat);
        let r = -ground_drop_side([p.hip[1]; 2], [p.knee[1]; 2], [p.side[1]; 2], p.flat);
        let ank = |px: f32, part| pose_vertex(Vec3::new(px, 0.0, ANKLE_Z), part, &p).z;
        let hip = pose_vertex(Vec3::new(0.0, 0.0, HIP_Z), Part::Torso, &p).z;
        print!("hip over lower ankle {:.3} ", hip - ank(-HIP_X, Part::LeftShin).min(ank(HIP_X, Part::RightShin)));
        print!("boot {:+.3} ", p.drop + l.min(r));
        println!("{k:2} drop {:+.3}  left foot low {:+.3}  right foot low {:+.3}  hip {:+.2} {:+.2} knee {:+.2} {:+.2}", p.drop, l, r, p.hip[0], p.hip[1], p.knee[0], p.knee[1]);
    }
}
