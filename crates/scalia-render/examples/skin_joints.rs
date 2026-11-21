//! Lists the skinned soldier's joints and where a clip puts the hands (AN-03 debug aid):
//! `cargo run -p scalia-render --example skin_joints -- [clip] [seconds]`.
use scalia_render::skin::{Anims, ANIM_BYTES};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let an = Anims::load(ANIM_BYTES);
    let clip = an.clip(a.get(1).map_or("idle_aim", |s| s)).unwrap();
    let t: f32 = a.get(2).map_or(0.0, |s| s.parse().unwrap());
    let w = an.world(&an.sample(clip, t));
    for (j, joint) in an.joints.iter().enumerate() {
        println!(
            "{j:2} {:20} parent {:3} at {:+.3} {:+.3} {:+.3}",
            joint.name, joint.parent, w[j].pos.x, w[j].pos.y, w[j].pos.z
        );
    }
}
