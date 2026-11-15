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
fn prone_aim_pose_is_mirrored_in_the_shader() {
    use scalia_render::pose::{params, wgsl_literal, Pose};
    let want = wgsl_literal(&params(Pose::ProneAim, 0.0));
    let body = &WGSL[WGSL.find("pose == 36u").unwrap()..];
    let body = &body[..body.find("\n  } else if").unwrap_or(body.len())];
    let expected = [
        "p.prone = 1.0;",
        "carry(&p, 5u);",
        "p.rifle = p.rifle + 1.5707963;",
        "p.twist = -p.ryaw;",
        "p.look = p.ryaw;",
        "p.head = 1.0;",
    ];
    for line in expected { assert!(body.contains(line), "shader prone aim missing: {line}"); }
    assert!(want.contains("1.570796"), "Rust reference keeps the rifle rotated onto the prone aiming line: {want}");
}

#[test]
fn crawl_directional_terms_match_in_rust_and_wgsl() {
    assert!(RS.contains("phase * std::f32::consts::TAU + dir"), "Rust crawl phase uses travel direction");
    assert!(WGSL.contains("phase * 6.2831853 + dir"), "WGSL crawl phase uses travel direction");
    for term in ["0.16 * lateral", "0.12 * lateral", "0.08 * lateral", "0.04 * lateral"] {
        let rust = RS.contains(term);
        let shader = WGSL.contains(term);
        assert_eq!(rust, shader, "crawl directional parameter parity for {term}");
    }
}


#[test]
fn prone_aim_is_two_handed_in_the_shader() {
    use scalia_render::pose::{two_handed, Pose};
    let body = &WGSL[WGSL.find("fn two_handed(").unwrap()..];
    let body = &body[..body.find("\n").unwrap()];
    assert!(body.contains("pose == 36u"), "shader grip correction must include ProneAim");
    assert_eq!(two_handed(Pose::ProneAim), 1.0);
}
