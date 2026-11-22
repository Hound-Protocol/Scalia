//! The vertex shader ports pose.rs by hand; these checks catch the two drifting apart in the numbers that
//! get tuned most (stride and carry tables).
use regex::Regex;

const RS: &str = include_str!("../src/pose.rs");
const WGSL: &str = include_str!("../src/wgsl/common.wgsl");

fn nums(s: &str) -> Vec<f32> {
    Regex::new(r"-?\d+\.\d+").unwrap().find_iter(s).map(|m| m.as_str().parse().unwrap()).collect()
}


#[test]
fn stride_calls_match() {
    let rs: Vec<Vec<f32>> = Regex::new(r"stride\(&mut p, phase, ([^)]*)\)").unwrap().captures_iter(RS).map(|c| nums(&c[1])).collect();
    let wg: Vec<Vec<f32>> = Regex::new(r"stride\(&p, phase, ([^)]*)\)").unwrap().captures_iter(WGSL).map(|c| nums(&c[1])).collect();
    assert!(!rs.is_empty());
    assert_eq!(rs, wg);
}

#[test]
fn carry_table_matches() {
    let rs: Vec<f32> = Regex::new(r"Carry::\w+ => \((.*)\),").unwrap().captures_iter(RS).flat_map(|c| nums(&c[1])).collect();
    let body = &WGSL[WGSL.find("fn carry(").unwrap()..];
    let body = &body[..body.find("\n}\n").unwrap()];
    // Per carry in the shader: sh, el, reach, abd (pairs), rifle, ryaw on one line, in carry-index order.
    let mut rows = Vec::new();
    for line in body.lines().filter(|l| l.contains("(*p).sh = vec2f(")) {
        rows.extend_from_slice(&nums(line)[..10]);
    }
    assert_eq!(rows.len(), 70, "seven carries in the shader");
    // pose.rs lists Rush after HighReady and before ReadyUp; the shader orders by index (ReadyUp 4, Rush 5).
    let mut rs_rows: Vec<&[f32]> = rs.chunks(10).collect();
    rs_rows.swap(4, 5);
    assert_eq!(rs_rows.concat(), rows);
}

#[test]
fn gait_calls_match() {
    let rs: Vec<Vec<f32>> = Regex::new(r"= gait_dir\(phase, ([^)]*), dir\)").unwrap().captures_iter(RS).map(|c| nums(&c[1])).collect();
    let wg: Vec<Vec<f32>> = Regex::new(r"[^_]gait\(&p, phase, ([^)]*), dir\)").unwrap().captures_iter(WGSL).map(|c| nums(&c[1])).collect();
    assert!(!rs.is_empty());
    assert_eq!(rs, wg);
}

#[test]
fn crouch_gait_calls_match() {
    let rs: Vec<Vec<f32>> = Regex::new(r"= crouch_gait\((?:phase|0\.0), ([^)]*), (?:dir|0\.0)\)").unwrap().captures_iter(RS).map(|c| nums(&c[1])).collect();
    let wg: Vec<Vec<f32>> = Regex::new(r"crouch_gait\((?:&p|p), (?:phase|0\.0), ([^)]*), (?:dir|0\.0)\)").unwrap().captures_iter(WGSL).map(|c| nums(&c[1])).collect();
    assert!(!rs.is_empty());
    // The two files list the poses in different orders: compare as sets.
    let key = |v: &Vec<Vec<f32>>| { let mut s: Vec<String> = v.iter().map(|x| format!("{x:?}")).collect(); s.sort(); s };
    assert_eq!(key(&rs), key(&wg));
}

#[test]
fn action_keyframes_match() {
    let sorted = |v: Vec<Vec<f32>>| { let mut s: Vec<String> = v.iter().map(|x| format!("{x:?}")).collect(); s.sort(); s };
    let rs = Regex::new(r"keyed\(phase, k, \[([^\]]*)\]\)").unwrap().captures_iter(RS).map(|c| nums(&c[1])).collect();
    let wg = Regex::new(r"keyed\(phase, k, array<f32, 5>\(([^)]*)\)\)").unwrap().captures_iter(WGSL).map(|c| nums(&c[1])).collect();
    let rk = Regex::new(r"let k = \[([^\]]*)\];").unwrap().captures_iter(RS).map(|c| nums(&c[1])).collect();
    let wk = Regex::new(r"let k = array<f32, 5>\(([^)]*)\);").unwrap().captures_iter(WGSL).map(|c| nums(&c[1])).collect();
    let (rs, wg) = (sorted(rs), sorted(wg));
    assert!(!rs.is_empty());
    assert_eq!(rs, wg);
    assert_eq!(sorted(rk), sorted(wk));
}

/// The drop's middle keys are literals in the shader (`DROP_REACH_P`, `DROP_PLANT_P`): they must be exactly the
/// Rust poses. On a mismatch the message holds the literal to paste.
#[test]
fn drop_keys_match() {
    use scalia_render::pose::{params, wgsl_literal, Pose};
    for (name, pose) in [("DROP_REACH_P", Pose::DropReach), ("DROP_PLANT_P", Pose::DropPlant)] {
        let want = wgsl_literal(&params(pose, 0.0));
        let line = WGSL.lines().find(|l| l.starts_with(&format!("const {name}: PP = "))).unwrap_or("");
        let got = line.trim_start_matches(&format!("const {name}: PP = ")).trim_end_matches(';');
        let (a, b) = (nums(got), nums(&want));
        assert!(a.len() == b.len() && a.iter().zip(&b).all(|(x, y)| (x - y).abs() < 1e-5), "const {name}: PP = {want};");
    }
}

/// Shader compilers inline every call, and `pose_params` is the biggest function in the shader: each extra call in
/// `animate` multiplies what the GPU driver must compile at load (a long compile can stall the whole browser).
#[test]
fn animate_evaluates_poses_at_most_four_times() {
    let body = &WGSL[WGSL.find("fn animate(").unwrap()..];
    let body = &body[..body.find("\n}\n").unwrap()];
    assert!(body.matches("pose_params(").count() <= 4, "pose_params called {} times in animate", body.matches("pose_params(").count());
    assert_eq!(WGSL.matches("pose_params(").count() - 1, body.matches("pose_params(").count(), "and nowhere else");
}


#[test]
fn reload_manipulation_phase_and_key_times_match_the_wgsl_mirror() {
    assert!(RS.contains("pub fn reload_hand("), "Rust reference exposes the magazine reach");
    assert!(WGSL.contains("let reload_phase = clamp(-b.z - 1.0, 0.0, 1.0);"));
    assert!(WGSL.contains("let base_phase = select(phase, reload_phase, u32(b.w) == 18u);"));
    assert!(WGSL.contains("let upper_phase = select(phase, reload_phase, upper_id == 18u);"));
    assert!(WGSL.contains("reload_target(pp, reload_phase)"), "shader uses the independent reload phase");
    assert!(WGSL.contains("lb.x == 19.0"), "the previous upper reload keeps its hand path during blend-out");

    let rust = Regex::new(r"let kt = \[([^\]]*)\]").unwrap().captures(RS).unwrap();
    let mut rust_times = nums(&rust[1]);
    let body = &WGSL[WGSL.find("fn reload_target(").unwrap()..];
    let body = &body[..body.find("\n}\n").unwrap()];
    let mut shader_times: Vec<f32> = Regex::new(r"t >= ([0-9.]+)")
        .unwrap()
        .captures_iter(body)
        .map(|c| c[1].parse().unwrap())
        .collect();
    shader_times.push(0.0);
    rust_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    shader_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(rust_times, shader_times, "reload hand path key times match");
}
#[test]
fn idle_animation_constants_match_between_rust_and_wgsl() {
    let extract = |src: &str| {
        let re = Regex::new(r"const (IDLE_[A-Z0-9_]+): f32 = ([0-9.]+);").unwrap();
        let mut values: Vec<(String, f32)> = re.captures_iter(src)
            .map(|c| (c[1].to_string(), c[2].parse().unwrap()))
            .collect();
        values.sort_by(|a, b| a.0.cmp(&b.0));
        values
    };
    let rust = extract(RS);
    let shader = extract(WGSL);
    assert!(!rust.is_empty(), "idle constants are present in the Rust pose reference");
    assert_eq!(rust, shader, "idle animation tuning stays in sync with the shader");
}


#[test]
fn crawl_directional_terms_match_in_rust_and_wgsl() {
    // The high crawl reads HIGH_CRAWL in both; its direction terms are written by hand in each.
    for term in ["(-0.2 - fwd) * 5.0", "0.3 * lateral", "0.4 * lateral", "-phase"] {
        assert!(RS.contains(term) && WGSL.contains(term), "crawl direction term `{term}` must be in pose.rs and common.wgsl");
    }
}

#[test]
fn baked_clip_registry_matches() {
    // AN-02-S12: the baked clip tables live only in pose.rs and reach the shader through the clip buffer; the
    // offsets, field counts and row counts the shader indexes with must be the ones pose.rs lays out.
    use scalia_render::pose::{clip_buffer, CLIP_FLOATS};
    let table = Regex::new(r"pub const (\w+): \[\[f32; (\d+)\]; (\d+)\] = \[").unwrap();
    let mut at = 0u32;
    let mut seen = 0;
    for c in table.captures_iter(RS) {
        let (name, f, n): (&str, u32, u32) = (&c[1], c[2].parse().unwrap(), c[3].parse().unwrap());
        if !RS.contains(&format!("// CLIP {name} BEGIN")) { continue; }
        for (k, v) in [("AT", at), ("F", f), ("N", n)] {
            let want = format!("const {name}_{k}: u32 = {v}u;");
            assert!(WGSL.contains(&want), "common.wgsl lacks `{want}` (run scripts/clip_table.py --registry)");
        }
        assert!(RS.contains(&format!("pub const {name}_AT: u32 = {at};")), "{name}_AT out of date");
        at += f * n;
        seen += 1;
    }
    assert!(seen >= 1, "no baked clip tables found");
    // After the tables: the gaits' boot roll, built from the polished loops (pose.rs `clip_buffer`).
    assert!(WGSL.contains(&format!("const GAIT_TOE_AT: u32 = {at}u;")), "GAIT_TOE_AT out of date");
    assert_eq!(at, scalia_render::pose::GAIT_TOE_AT);
    assert_eq!(at + scalia_render::pose::GAIT_TOE_FLOATS, CLIP_FLOATS);
    assert_eq!(clip_buffer().len() as u32, CLIP_FLOATS);
}

/// The drop's hand plant constants (pose.rs DROP_HAND*) are written into the shader by hand.
#[test]
fn drop_hand_constants_match() {
    use scalia_render::pose::{DROP_HAND, DROP_HAND_REACH, DROP_HAND_SLIDE};
    let get = |name: &str| nums(WGSL.lines().find(|l| l.starts_with(&format!("const {name}:"))).unwrap_or("").split_once('(').map_or("", |x| x.1));
    assert_eq!(get("DROP_HAND"), DROP_HAND.to_vec());
    assert_eq!(get("DROP_HAND_REACH"), DROP_HAND_REACH.to_vec());
    assert_eq!(get("DROP_HAND_SLIDE"), DROP_HAND_SLIDE.to_vec());
}

/// Seams: the shader reads the blend as `bake_seams` packed it and turns about the same joint pivots
/// (pose.rs `SEAM_SCALE`, `seam_joint`).
#[test]
fn seam_constants_match() {
    use scalia_render::mesh::part_of;
    use scalia_render::pose::{seam_joint, SEAM_SCALE};
    let get = |name: &str| nums(WGSL.lines().find(|l| l.starts_with(&format!("const {name}:"))).unwrap_or("").split_once('=').map_or("", |x| x.1));
    assert_eq!(get("SEAM_SCALE"), vec![SEAM_SCALE]);
    let body = &WGSL[WGSL.find("fn seam_joint(").unwrap()..WGSL.find("fn quat_of(").unwrap()];
    let re = Regex::new(r"a == (\d+)u && b == (\d+)u\) \{ return vec4f\(([^)]*)\)").unwrap();
    let mut n = 0;
    for c in re.captures_iter(body) {
        let part = |i: usize| part_of(c[i].parse::<f32>().unwrap() * 32.0);
        let (j, _) = seam_joint(part(1), part(2)).expect("a joint the Rust side knows");
        assert_eq!(nums(&c[3])[..3], j.to_array(), "pivot between {:?} and {:?}", part(1), part(2));
        n += 1;
    }
    assert_eq!(n, 11, "every joint");
}

/// Pose changes: the shader's blend times are the Rust ones (pose.rs `BLEND_S`, `KNEEL_BLEND_S`).
#[test]
fn blend_times_match() {
    use scalia_render::pose::{BLEND_S, KNEEL_BLEND_S};
    let get = |name: &str| nums(WGSL.lines().find(|l| l.starts_with(&format!("const {name}:"))).unwrap_or("").split_once('=').map_or("", |x| x.1));
    assert_eq!(get("BLEND_S"), vec![BLEND_S]);
    assert_eq!(get("KNEEL_BLEND_S"), vec![KNEEL_BLEND_S]);
}
