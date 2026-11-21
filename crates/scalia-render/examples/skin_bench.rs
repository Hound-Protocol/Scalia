//! CPU cost of the per-soldier muzzle lookup the game runs every frame (AN-03): `skin_bench [soldiers]`.
use scalia_render::skin::{anims, muzzle};
fn main() {
    let n: usize = std::env::args().nth(1).map_or(1000, |s| s.parse().unwrap());
    let an = anims();
    let recs: Vec<[f32; 12]> = (0..n)
        .map(|i| {
            [
                0.0,
                0.0,
                0.0,
                1.0,
                0.0,
                i as f32 * 0.013 % 1.0,
                1.4,
                [1.0, 2.0, 7.0, 6.0][i % 4],
                1.0,
                9.0,
                0.3,
                0.0,
            ]
        })
        .collect();
    let t0 = std::time::Instant::now();
    let mut acc = 0.0;
    for frame in 0..60 {
        for r in &recs {
            acc += muzzle(an, r, 10.0 + frame as f32 / 60.0).0.x;
        }
    }
    println!(
        "{n} soldiers: {:.2} ms per frame ({acc:.0})",
        t0.elapsed().as_secs_f64() * 1000.0 / 60.0
    );
}
