//! Prints the killhouse battle hash after `ms` (default 60 s), for the native-versus-WebAssembly check.
fn main() {
    let mut a = std::env::args().skip(1);
    let seed: u64 = a.next().and_then(|s| s.parse().ok()).unwrap_or(42);
    let ms: i64 = a.next().and_then(|s| s.parse().ok()).unwrap_or(60_000);
    println!("{}", scalia_ai::scenario::killhouse_hash(seed, ms));
}
