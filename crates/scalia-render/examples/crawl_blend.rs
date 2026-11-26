//! How deep anything dips during a crawl <-> prone blend, by the crawl phase the blend leaves or meets
//! (`cargo run -q --release -p scalia-render --example crawl_blend`): per phase, the most any group goes below the
//! lower of its two ends (m), and the fastest joint (rad/s over BLEND_S).
use scalia_render::contact::Body;
use scalia_render::pose::{blend, grip, grip_weight, params, params_dir, Pose, PoseParams, BLEND_S};
fn angles(p: &PoseParams) -> Vec<f32> {
    let mut v = vec![p.lean, p.head, p.rifle, p.rifle_yaw, p.twist, p.look, p.tilt, p.yaw, p.raise];
    for pair in [p.hip, p.knee, p.side, p.shoulder, p.elbow, p.reach, p.abduct, p.roll] { v.extend(pair); }
    v
}
fn main() {
    let body = Body::new();
    let prone = params(Pose::Prone, 0.0);
    println!("phase  dip(knee elbow boot rifle)  fastest rad/s");
    for k in 0..32 {
        let ph = k as f32 / 32.0;
        let crawl = params_dir(Pose::Crawl, ph, 0.0);
        let (a, b) = (body.stats(&grip(&prone, 1.0)), body.stats(&crawl));
        let mut dip = [0.0f32; 4];
        let mut fast = 0.0f32;
        let mut prev = angles(&grip(&prone, 1.0));
        for i in 1..=40 {
            let w = i as f32 / 40.0;
            // As drawn: the support hand's grip blended in with the pose (common.wgsl).
            let p = grip(&blend(&prone, &crawl, w), grip_weight(Pose::Prone, Pose::Crawl, w));
            let s = body.stats(&p);
            for (j, (now, e)) in [(s.knee, a.knee.min(b.knee)), (s.elbow, a.elbow.min(b.elbow)), (s.boot, a.boot.min(b.boot)), (s.rifle, a.rifle.min(b.rifle))].into_iter().enumerate() {
                dip[j] = dip[j].max(e - now);
            }
            let cur = angles(&p);
            fast = fast.max(cur.iter().zip(&prev).map(|(x, y)| (x - y).abs()).fold(0.0, f32::max) / (BLEND_S / 40.0));
            prev = cur;
        }
        println!("{ph:.3}  {:.3} {:.3} {:.3} {:.3}  {fast:.1}", dip[0], dip[1], dip[2], dip[3]);
    }
}
