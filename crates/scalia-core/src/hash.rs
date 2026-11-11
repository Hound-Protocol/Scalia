//! Counter-based hashing: any soldier at any time can be computed alone, in any order.
//! No generator state is stored anywhere.

use crate::fixed::Fix;

pub const STREAM_FORMATION: u64 = 1;
pub const STREAM_WALK: u64 = 2;
pub const STREAM_SEAT: u64 = 3;
pub const STREAM_FORMATION_Y: u64 = 4;
pub const STREAM_ACCURACY: u64 = 5;
pub const STREAM_SPEED: u64 = 6;
pub const STREAM_COMPOSURE: u64 = 7;
pub const STREAM_NAME: u64 = 8;
pub const STREAM_TERRAIN: u64 = 9;

/// SplitMix64 finaliser applied to `x + golden gamma`.
pub fn mix64(x: u64) -> u64 {
    let mut z = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

pub fn hash4(seed: u64, a: u64, b: u64, c: u64) -> u64 {
    let mut h = mix64(seed);
    h = mix64(h ^ a);
    h = mix64(h ^ b);
    mix64(h ^ c)
}

/// Value in [0, 1) from the top 32 bits.
pub fn unit(h: u64) -> Fix {
    Fix((h >> 32) as i64)
}

/// `lo` inclusive, `hi` exclusive; returns `lo` if `hi <= lo`.
pub fn range_i64(h: u64, lo: i64, hi: i64) -> i64 {
    if hi <= lo {
        return lo;
    }
    let span = (hi as i128 - lo as i128) as u128;
    let step = ((h as u128 * span) >> 64) as i128;
    (lo as i128 + step) as i64
}
