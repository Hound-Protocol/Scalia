//! The commander's tablet pose for the first-person render (scripts/first_person.py), optionally with the left
//! hand raised beside the tablet holding the walkie-talkie: `fp_pose_dump <out> [walkie]`.
//! Writes the posed soldier (f32 x, y, z, code per vertex); prints the left palm and forearm frame as JSON.
use glam::Vec3;
use scalia_render::mesh::{figure, part_of, Part};
use scalia_render::pose::{params, pose_vertex, Pose, PoseParams, ELBOW_X, ELBOW_Z};

fn palm(p: &PoseParams) -> Vec3 { pose_vertex(Vec3::new(-0.18, 0.0, 0.80), Part::LeftForearm, p) }
fn elbow(p: &PoseParams) -> Vec3 { pose_vertex(Vec3::new(-ELBOW_X, 0.0, ELBOW_Z), Part::LeftForearm, p) }

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let mut p = params(Pose::Tablet, 0.0);
    eprintln!("tablet pose: left palm {:?} elbow {:?}", palm(&p), elbow(&p));
    if a.get(2).map(|s| s.as_str()) == Some("walkie") {
        let want_palm = Vec3::new(a[3].parse().unwrap(), a[4].parse().unwrap(), a[5].parse().unwrap());
        let want_elbow = want_palm + Vec3::new(0.02, -0.12, -0.24);
        let base = [p.shoulder[0], p.elbow[0], p.reach[0], p.abduct[0]];
        let cost = |x: &[f32; 4]| {
            let mut q = p;
            (q.shoulder[0], q.elbow[0], q.reach[0], q.abduct[0]) = (x[0], x[1], x[2], x[3]);
            palm(&q).distance_squared(want_palm) + 0.3 * elbow(&q).distance_squared(want_elbow)
                + 0.002 * x.iter().zip(base).map(|(a, b)| (a - b) * (a - b)).sum::<f32>()
        };
        let mut x = base;
        let mut step = 0.2f32;
        while step > 1e-4 {
            let mut better = false;
            for k in 0..4 { for s in [-1.0f32, 1.0] {
                let mut y = x; y[k] += s * step;
                if cost(&y) < cost(&x) { x = y; better = true; }
            } }
            if !better { step *= 0.5; }
        }
        (p.shoulder[0], p.elbow[0], p.reach[0], p.abduct[0]) = (x[0], x[1], x[2], x[3]);
        eprintln!("walkie arm {x:?}: palm {:?} (want {want_palm:?}) elbow {:?}", palm(&p), elbow(&p));
    }
    // Forearm frame at the palm: along the forearm (elbow to palm), and the palm's outward normal.
    let (pa, el) = (palm(&p), elbow(&p));
    let side = pose_vertex(Vec3::new(-0.18 - 0.05, 0.0, 0.80), Part::LeftForearm, &p) - pa;
    let front = pose_vertex(Vec3::new(-0.18, 0.05, 0.80), Part::LeftForearm, &p) - pa;
    println!("{{\"palm\":[{},{},{}],\"along\":[{},{},{}],\"out\":[{},{},{}],\"front\":[{},{},{}]}}",
        pa.x, pa.y, pa.z, (pa - el).normalize().x, (pa - el).normalize().y, (pa - el).normalize().z,
        side.normalize().x, side.normalize().y, side.normalize().z, front.normalize().x, front.normalize().y, front.normalize().z);
    let mut out = Vec::new();
    for v in figure().vertices {
        let q = pose_vertex(Vec3::from(v.pos), part_of(v.color), &p);
        for f in [q.x, q.y, q.z, v.color] { out.extend_from_slice(&f.to_le_bytes()); }
    }
    std::fs::write(&a[1], out).unwrap();
}
