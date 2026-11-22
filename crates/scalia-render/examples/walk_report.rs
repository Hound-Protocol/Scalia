//! Gait quality numbers for one pose (debug aid for the walk polish):
//! `cargo run -q -p scalia-render --example walk_report -- <pose id> [dir]`.
//! Prints left/right symmetry (left leg against the right leg half a cycle later), stance share, knee range,
//! and the largest frame-to-frame jerk in the leg angles (a stutter shows as a spike).
use scalia_render::pose::{params_dir, Pose};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let pose = Pose::from_id(a[1].parse().unwrap());
    let dir: f32 = a.get(2).map_or(0.0, |d| d.parse().unwrap());
    let n = 96;
    let p: Vec<_> = (0..n).map(|k| params_dir(pose, k as f32 / n as f32, dir)).collect();
    let (mut asym_hip, mut asym_knee) = (0.0f32, 0.0f32);
    for k in 0..n {
        let o = &p[(k + n / 2) % n];
        asym_hip = asym_hip.max((p[k].hip[0] - o.hip[1]).abs());
        asym_knee = asym_knee.max((p[k].knee[0] - o.knee[1]).abs());
    }
    for (s, name) in [(0, "L"), (1, "R")] {
        let hip: Vec<f32> = p.iter().map(|q| q.hip[s]).collect();
        let knee: Vec<f32> = p.iter().map(|q| q.knee[s]).collect();
        let jerk = |v: &[f32]| (0..n).map(|k| (v[(k + 2) % n] - 2.0 * v[(k + 1) % n] + v[k]).abs()).fold(0.0f32, f32::max);
        let worst = (0..n).max_by(|&i, &j| {
            let f = |k: usize| (knee[(k + 2) % n] - 2.0 * knee[(k + 1) % n] + knee[k]).abs();
            f(i).total_cmp(&f(j))
        }).unwrap();
        let mx = |v: &[f32]| v.iter().cloned().fold(f32::MIN, f32::max);
        let mn = |v: &[f32]| v.iter().cloned().fold(f32::MAX, f32::min);
        println!("{name}: hip {:+.2}..{:+.2}  knee {:+.2}..{:+.2}  jerk hip {:.3} knee {:.3} (worst knee at {:.2})",
            mn(&hip), mx(&hip), mn(&knee), mx(&knee), jerk(&hip), jerk(&knee), worst as f32 / n as f32);
    }
    println!("asymmetry: hip {asym_hip:.3} knee {asym_knee:.3}");
}
