//! Dumps the skinned soldier for scripts/stance_sheet.py (AN-03): `skin_dump <out> <clip>@<seconds>...`
//! (`DUMP=skin_dump scripts/stance_sheet.py out.png walk_f@0 walk_f@0.25`). Same layout as stance_dump: per
//! vertex x, y, z, colour code (f32).
use scalia_render::skin::{skin_vertex, soldier, Anims, ANIM_BYTES};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let anims = Anims::load(ANIM_BYTES);
    let mesh = soldier(&anims);
    let mut out = Vec::new();
    for spec in &a[2..] {
        let (name, t) = spec
            .split_once('@')
            .map_or((spec.as_str(), 0.0), |(n, t)| (n, t.parse().unwrap()));
        let clip = anims.clip(name).unwrap_or_else(|| panic!("no clip {name}"));
        let skin = anims.skinning(&anims.world(&anims.sample(clip, t)));
        for v in &mesh {
            let q = skin_vertex(v, &skin);
            for f in [q.x, q.y, q.z, v.color] {
                out.extend_from_slice(&f.to_le_bytes());
            }
        }
    }
    std::fs::write(&a[1], out).unwrap();
}
