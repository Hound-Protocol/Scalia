//! The battle simulation is integer only: floats (and the platform maths behind sin, cos, …) can differ
//! between native and WebAssembly builds and would break determinism. A line that must touch a float says why
//! with a `float-ok:` comment.
#[test]
fn the_simulation_uses_no_floating_point() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
    let mut bad = Vec::new();
    for e in std::fs::read_dir(dir).unwrap() {
        let path = e.unwrap().path();
        let text = std::fs::read_to_string(&path).unwrap();
        for (n, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            if (code.contains("f64") || code.contains("f32")) && !line.contains("float-ok:") {
                bad.push(format!(
                    "{}:{}: {}",
                    path.file_name().unwrap().to_string_lossy(),
                    n + 1,
                    line.trim()
                ));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "floating point in the simulation:\n{}",
        bad.join("\n")
    );
}
