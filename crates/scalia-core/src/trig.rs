//! Integer sine and cosine from a quarter-wave table with linear interpolation.
//! One turn is 2*pi. Error is under 1e-6.

use crate::fixed::Fix;
use crate::trig_table::QUARTER_SIN;

const QUARTER: i64 = 1 << 30;

/// sin of an angle `x` in [0, 2^30] where 2^30 is a quarter turn; result is a raw Q32.32 value.
fn sinq(x: i64) -> i64 {
    let idx = (x >> 20) as usize;
    if idx >= 1024 {
        return QUARTER_SIN[1024];
    }
    let rem = x & 0xF_FFFF;
    let a = QUARTER_SIN[idx];
    let b = QUARTER_SIN[idx + 1];
    a + (((b - a) * rem) >> 20)
}

/// `(sin, cos)` of `turns` full turns; any value wraps by floor modulo one turn.
pub fn sin_cos_turns(turns: Fix) -> (Fix, Fix) {
    let frac = (turns.0 as u64 & 0xFFFF_FFFF) as i64;
    let q = frac >> 30;
    let x = frac & (QUARTER - 1);
    let (a, b) = (sinq(x), sinq(QUARTER - x));
    let (s, c) = match q {
        0 => (a, b),
        1 => (b, -a),
        2 => (-a, -b),
        _ => (-b, a),
    };
    (Fix(s), Fix(c))
}
