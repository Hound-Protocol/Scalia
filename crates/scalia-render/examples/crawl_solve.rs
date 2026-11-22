//! Solves the high crawl and prints it: `cargo run -q -p scalia-render --example crawl_solve [rows]`.
//! With `rows`, prints the table for pose.rs; without, a check of the contacts per phase.
use scalia_render::contact::Body;
use scalia_render::crawl::{apply, base, solve};
fn main() {
    let rows = solve();
    if std::env::args().nth(1).as_deref() == Some("rows") {
        for r in &rows {
            println!("    [{}],", r.iter().map(|v| format!("{v:.4}")).collect::<Vec<_>>().join(", "));
        }
        return;
    }
    let body = Body::new();
    println!("phase  boot   knee   elbow  belly  head   rifle  arm-in | hipL hipR kneeL kneeR sideL sideR shL shR elL elR lean raise");
    for (k, r) in rows.iter().enumerate() {
        let mut p = base();
        apply(&mut p, r);
        let s = body.stats(&p);
        println!("{:.3} {:6.3} {:6.3} {:6.3} {:6.3} {:6.3} {:6.3} {:6.3} | {}", k as f32 / rows.len() as f32, s.boot, s.knee, s.elbow, s.belly, s.head, s.rifle, s.arm_in,
            [r[0], r[1], r[2], r[3], r[4], r[5], r[6], r[7], r[8], r[9], r[14], r[15]].iter().map(|v| format!("{v:5.2}")).collect::<Vec<_>>().join(" "));
    }
}
