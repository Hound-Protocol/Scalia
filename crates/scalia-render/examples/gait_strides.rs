//! Measures how far each baked gait's legs carry the body per cycle, per direction (AN-02-S13), so the game sets
//! the cadence that keeps the planted boot still: `cargo run -p scalia-render --example gait_strides`. While a
//! boot is the lowest point and on the ground, it should not move over the ground; its sweep against the body
//! over that time, divided by the share of the cycle it lasts, is the stride. Prints `GAIT_STRIDE` for pose.rs.
use glam::Vec3;
use scalia_render::mesh::{figure, part_of, Part};
use scalia_render::pose::{params_dir, pose_vertex, Pose};

fn main() {
    let fig = figure();
    let boot = |part: Part| -> Vec<Vec3> { fig.vertices.iter().filter(|v| part_of(v.color) == part).map(|v| Vec3::from(v.pos)).collect() };
    let (left, right) = (boot(Part::LeftFoot), boot(Part::RightFoot));
    const N: usize = 96;
    println!("pub const GAIT_STRIDE: [[f32; 8]; 4] = [");
    for pose in [Pose::Walk, Pose::Run, Pose::Sprint, Pose::CrouchWalk] {
        let mut row = Vec::new();
        for d in 0..8 {
            let dir = d as f32 * std::f32::consts::TAU / 8.0;
            let travel = Vec3::new(-dir.sin(), dir.cos(), 0.0);
            // Each boot's heel and toe (as examples/anim_audit samples them) per phase: a boot stands on the lower one,
            // and a rolling boot pivots on it, so that is the point that must hold still.
            let near = |pts: &Vec<Vec3>, at: Vec3| *pts.iter().min_by(|a, b| a.distance(at).total_cmp(&b.distance(at))).unwrap();
            let track = |pts: &Vec<Vec3>, part: Part, x: f32| -> Vec<[Vec3; 2]> {
                let ends = [near(pts, Vec3::new(x, -0.09, 0.03)), near(pts, Vec3::new(x, 0.14, 0.03))];
                (0..N).map(|k| { let p = params_dir(pose, k as f32 / N as f32, dir); ends.map(|e| pose_vertex(e, part, &p)) }).collect()
            };
            let feet = [track(&left, Part::LeftFoot, -0.1), track(&right, Part::RightFoot, 0.1)];
            let floor: [f32; 2] = std::array::from_fn(|i| feet[i].iter().map(|e| e[0].z.min(e[1].z)).fold(f32::MAX, f32::min));
            // The planted boot is the lower one, when it is down (within 1.5 cm of its lowest) at both ends of a step:
            // the speed back along the travel of the point it stands on, as a share of the cycle, is the stride; the
            // median keeps the moment a landing boot touches down while still moving forward out.
            let mut speeds = Vec::new();
            for k in 0..N {
                let low = |i: usize, k: usize| feet[i][k % N][0].z.min(feet[i][k % N][1].z) - floor[i];
                let i = if low(0, k) <= low(1, k) { 0 } else { 1 };
                if low(i, k) < 0.015 && low(i, k + 1) < 0.015 {
                    let j = if feet[i][k][0].z <= feet[i][k][1].z { 0 } else { 1 };
                    speeds.push(-(feet[i][(k + 1) % N][j] - feet[i][k][j]).dot(travel) * N as f32);
                }
            }
            speeds.sort_by(f32::total_cmp);
            row.push(if speeds.is_empty() { 0.0 } else { speeds[speeds.len() / 2] });
        }
        println!("    [{}], // {pose:?}", row.iter().map(|v| format!("{v:.2}")).collect::<Vec<_>>().join(", "));
    }
    println!("];");
}
