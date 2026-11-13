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
    // Per carry in the shader: sh, el, reach, abd (pairs), rifle; the turn across the body is shared.
    let wg = nums(body);
    let ryaw = Regex::new(r"\(\*p\)\.ryaw = (-?\d+\.\d+)").unwrap().captures(body).unwrap()[1].parse::<f32>().unwrap();
    let mut rows = Vec::new();
    let mut i = 0;
    for line in body.lines().filter(|l| l.contains("(*p).sh = vec2f(")) {
        let n = nums(line);
        rows.extend_from_slice(&n[..9]);
        rows.push(ryaw);
        i += 1;
    }
    assert_eq!(i, 6, "six carries in the shader ({wg:?})");
    assert_eq!(rs, rows);
}

#[test]
fn gait_calls_match() {
    let rs: Vec<Vec<f32>> = Regex::new(r"= gait_dir\(phase, ([^)]*), dir\)").unwrap().captures_iter(RS).map(|c| nums(&c[1])).collect();
    let wg: Vec<Vec<f32>> = Regex::new(r"gait\(&p, phase, ([^)]*), dir\)").unwrap().captures_iter(WGSL).map(|c| nums(&c[1])).collect();
    assert!(!rs.is_empty());
    assert_eq!(rs, wg);
}
