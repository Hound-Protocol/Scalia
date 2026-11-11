//! World time is real milliseconds. Game time runs at 24 times real time.

pub const GAME_SPEED: u64 = 24;
pub const TICK_MS: u64 = 1000;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct WorldMs(pub u64);

impl WorldMs {
    pub fn game_ms(self) -> u64 {
        self.0.saturating_mul(GAME_SPEED)
    }

    pub fn since(self, earlier: WorldMs) -> u64 {
        self.0.saturating_sub(earlier.0)
    }

    pub fn saturating_add(self, ms: u64) -> WorldMs {
        WorldMs(self.0.saturating_add(ms))
    }
}
