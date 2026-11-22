use scalia_render::contact::Body;
use scalia_render::pose::{blend, params, params_dir, Pose};
fn main() {
    let body = Body::new();
    let (a, b) = (params(Pose::Prone, 0.0), params_dir(Pose::Crawl, 0.0, 0.0));
    for i in 0..=10 { let p = blend(&a, &b, i as f32 / 10.0); let s = body.stats(&p);
        println!("{i}/10 rifle {:.3} raise {:.3} rifle_pitch {:.2} ryaw {:.2} sh1 {:.2} el1 {:.2} ab1 {:.2} reach1 {:.2} twist {:.2}", s.rifle, p.raise, p.rifle, p.rifle_yaw, p.shoulder[1], p.elbow[1], p.abduct[1], p.reach[1], p.twist); }
}
