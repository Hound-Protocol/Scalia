use scalia_core::position::{Pos, RegionId};
use scalia_core::process::{march_pos, Process, ProcessError};
use scalia_core::time::WorldMs;

fn p(x: i64, y: i64, z: i64) -> Pos {
    Pos {
        region: RegionId(0),
        x,
        y,
        z,
    }
}

#[test]
fn march_duration() {
    let a = Process::march(p(0, 0, 0), p(3000, 4000, 0), 1000, WorldMs(0)).unwrap();
    assert_eq!(a.duration_ms, 5000);
    let b = Process::march(p(0, 0, 0), p(3000, 4000, 0), 3000, WorldMs(0)).unwrap();
    assert_eq!(b.duration_ms, 1667);
}

#[test]
fn march_zero_speed_errors() {
    let r = Process::march(p(0, 0, 0), p(10, 0, 0), 0, WorldMs(0));
    assert_eq!(r.unwrap_err(), ProcessError::ZeroSpeed);
}

#[test]
fn march_pos_clamps() {
    let from = p(1000, 1000, 0);
    let to = p(11000, 1000, 0);
    let pr = Process::march(from, to, 1000, WorldMs(5000)).unwrap();
    assert_eq!(march_pos(&pr, WorldMs(0)), Some(from));
    let mid = march_pos(&pr, WorldMs(5000 + pr.duration_ms / 2)).unwrap();
    assert!((mid.x - 6000).abs() <= 1 && mid.y == 1000);
    assert_eq!(march_pos(&pr, WorldMs(5000 + pr.duration_ms)), Some(to));
    assert_eq!(march_pos(&pr, WorldMs(u64::MAX)), Some(to));
}

#[test]
fn zero_length_march_is_instant() {
    let pr = Process::march(p(5, 5, 5), p(5, 5, 5), 1000, WorldMs(0)).unwrap();
    assert_eq!(pr.duration_ms, 0);
    assert_eq!(march_pos(&pr, WorldMs(0)), Some(p(5, 5, 5)));
}

#[test]
fn idle_has_no_march_position() {
    assert_eq!(march_pos(&Process::idle(), WorldMs(10)), None);
}

#[test]
fn march_across_regions_errors() {
    let from = p(0, 0, 0);
    let to = Pos {
        region: RegionId(1),
        x: 10,
        y: 0,
        z: 0,
    };
    assert_eq!(
        Process::march(from, to, 1000, WorldMs(0)).unwrap_err(),
        ProcessError::RegionMismatch
    );
}
