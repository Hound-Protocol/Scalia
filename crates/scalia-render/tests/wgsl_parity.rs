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
    assert_eq!(rows.len(), 60, "six carries in the shader");
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
    let wg: Vec<Vec<f32>> = Regex::new(r"crouch_gait\(&p, (?:phase|0\.0), ([^)]*), (?:dir|0\.0)\)").unwrap().captures_iter(WGSL).map(|c| nums(&c[1])).collect();
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
