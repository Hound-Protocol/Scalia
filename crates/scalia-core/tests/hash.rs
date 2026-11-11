use scalia_core::fixed::Fix;
use scalia_core::hash::{hash4, mix64, range_i64, unit};

#[test]
fn mix64_golden() {
    assert_eq!(mix64(0), 0xe220a8397b1dcdaf);
    assert_eq!(mix64(1), 0x910a2dec89025cc1);
}

#[test]
fn hash4_golden() {
    assert_eq!(hash4(1, 2, 3, 4), 0xd55ccd4aeb3ccafb);
}

#[test]
fn unit_is_top_32_bits() {
    assert_eq!(unit(hash4(1, 2, 3, 4)), Fix(3_579_628_874));
    assert_eq!(unit(u64::MAX), Fix(0xFFFF_FFFF));
}

#[test]
fn range_golden_and_degenerate() {
    assert_eq!(range_i64(hash4(1, 2, 3, 4), -5, 5), 3);
    assert_eq!(range_i64(mix64(0), 0, 100), 88);
    assert_eq!(range_i64(7, 5, 5), 5);
}
