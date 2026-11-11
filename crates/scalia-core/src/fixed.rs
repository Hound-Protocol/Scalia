//! Q32.32 fixed-point numbers. All rounding is floor (toward negative infinity).

const FRAC: u32 = 32;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct Fix(pub i64);

fn floor_div(n: i128, d: i128) -> i128 {
    let q = n / d;
    if n % d != 0 && ((n < 0) != (d < 0)) {
        q - 1
    } else {
        q
    }
}

fn saturate(v: i128) -> i64 {
    v.clamp(i64::MIN as i128, i64::MAX as i128) as i64
}

impl Fix {
    pub const ZERO: Fix = Fix(0);
    pub const ONE: Fix = Fix(1 << FRAC);

    pub fn from_int(n: i32) -> Fix {
        Fix((n as i64) << FRAC)
    }

    pub fn from_ratio(n: i64, d: i64) -> Option<Fix> {
        if d == 0 {
            return None;
        }
        Some(Fix(saturate(floor_div((n as i128) << FRAC, d as i128))))
    }

    pub fn mul(self, o: Fix) -> Fix {
        Fix(saturate((self.0 as i128 * o.0 as i128) >> FRAC))
    }

    pub fn div(self, o: Fix) -> Option<Fix> {
        if o.0 == 0 {
            return None;
        }
        Some(Fix(saturate(floor_div((self.0 as i128) << FRAC, o.0 as i128))))
    }

    pub fn floor_to_i64(self) -> i64 {
        self.0 >> FRAC
    }

    pub fn to_f32(self) -> f32 {
        self.0 as f32 / 4_294_967_296.0
    }
}
