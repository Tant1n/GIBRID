use std::time::Duration;

/// Publicly documented vanilla tick rate. Confirm with fixtures before using for parity.
pub const TICKS_PER_SECOND: u32 = 20;
pub const TICK_DURATION_MS: u64 = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TickClock {
    tick: u64,
    pending: Duration,
    max_catch_up: u32,
}

impl TickClock {
    pub fn new(max_catch_up: u32) -> Self {
        Self {
            tick: 0,
            pending: Duration::ZERO,
            max_catch_up,
        }
    }

    pub fn tick(&self) -> u64 {
        self.tick
    }

    pub fn step_duration() -> Duration {
        Duration::from_millis(TICK_DURATION_MS)
    }

    /// Advance wall time and return how many simulation ticks should run (capped).
    pub fn accumulate(&mut self, delta: Duration) -> u32 {
        self.pending = self.pending.saturating_add(delta);
        let step = Self::step_duration();
        let mut produced = 0;
        while self.pending >= step && produced < self.max_catch_up {
            self.pending -= step;
            self.tick += 1;
            produced += 1;
        }
        if produced == self.max_catch_up && self.pending >= step {
            self.pending = Duration::ZERO;
        }
        produced
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn produces_one_tick_per_50ms() {
        let mut clock = TickClock::new(8);
        assert_eq!(clock.accumulate(Duration::from_millis(50)), 1);
        assert_eq!(clock.tick(), 1);
    }

    #[test]
    fn caps_catch_up_after_stall() {
        let mut clock = TickClock::new(3);
        let produced = clock.accumulate(Duration::from_secs(5));
        assert_eq!(produced, 3);
        assert_eq!(clock.tick(), 3);
        assert_eq!(clock.pending, Duration::ZERO);
    }
}
