//! Hierarchical positions: a region id plus a local position in whole millimetres.
//! Arithmetic saturates so debug and release builds agree.

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash, Default)]
pub struct RegionId(pub u32);

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Pos {
    pub region: RegionId,
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

impl Pos {
    pub fn offset(self, dx: i64, dy: i64, dz: i64) -> Pos {
        Pos {
            region: self.region,
            x: self.x.saturating_add(dx),
            y: self.y.saturating_add(dy),
            z: self.z.saturating_add(dz),
        }
    }

    /// Blend towards `to` by `num/den` (clamped to 1), flooring each axis.
    /// `None` if the regions differ or `den` is zero.
    pub fn lerp(self, to: Pos, num: u64, den: u64) -> Option<Pos> {
        if self.region != to.region || den == 0 {
            return None;
        }
        let num = num.min(den) as i128;
        let den = den as i128;
        let axis = |a: i64, b: i64| -> i64 {
            let delta = b as i128 - a as i128;
            let step = (delta * num).div_euclid(den);
            (a as i128 + step) as i64
        };
        Some(Pos {
            region: self.region,
            x: axis(self.x, to.x),
            y: axis(self.y, to.y),
            z: axis(self.z, to.z),
        })
    }
}

pub fn isqrt(n: u128) -> u128 {
    if n == 0 {
        return 0;
    }
    let bits = 128 - n.leading_zeros();
    let mut x = 1u128 << ((bits + 1) / 2);
    loop {
        let y = (x + n / x) / 2;
        if y >= x {
            return x;
        }
        x = y;
    }
}

pub fn dist_mm(a: Pos, b: Pos) -> u64 {
    let dx = (a.x as i128 - b.x as i128).unsigned_abs();
    let dy = (a.y as i128 - b.y as i128).unsigned_abs();
    let dz = (a.z as i128 - b.z as i128).unsigned_abs();
    let sum = dx
        .saturating_mul(dx)
        .saturating_add(dy.saturating_mul(dy))
        .saturating_add(dz.saturating_mul(dz));
    isqrt(sum).min(u64::MAX as u128) as u64
}
