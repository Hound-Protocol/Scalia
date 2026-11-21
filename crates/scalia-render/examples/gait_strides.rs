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
            // Boot centre and lowest point per phase, for each leg.
            let track = |pts: &Vec<Vec3>, part: Part| -> Vec<(Vec3, f32)> {
                (0..N).map(|k| {
                    let p = params_dir(pose, k as f32 / N as f32, dir);
                    let q: Vec<Vec3> = pts.iter().map(|v| pose_vertex(*v, part, &p)).collect();
                    (q.iter().copied().sum::<Vec3>() / q.len() as f32, q.iter().map(|v| v.z).fold(f32::MAX, f32::min))
                }).collect()
            };
            let feet = [track(&left, Part::LeftFoot), track(&right, Part::RightFoot)];
            // The planted boot is the lower one, when it is on the ground (within 1.5 cm) at both ends of a step. Its
            // speed back along the travel, as a share of the cycle, is the stride; the median keeps the moment a
            // landing boot touches down while still moving forward (crouched, both boots are nearly as low) out.
            let mut speeds = Vec::new();
            for k in 0..N {
                let f = if feet[0][k].1 <= feet[1][k].1 { &feet[0] } else { &feet[1] };
                let (a, b) = (f[k], f[(k + 1) % N]);
                if a.1 < 0.015 && b.1 < 0.015 { speeds.push(-(b.0 - a.0).dot(travel) * N as f32); }
            }
            speeds.sort_by(f32::total_cmp);
            row.push(if speeds.is_empty() { 0.0 } else { speeds[speeds.len() / 2] });
        }
        println!("    [{}], // {pose:?}", row.iter().map(|v| format!("{v:.2}")).collect::<Vec<_>>().join(", "));
    }
    println!("];");
}
