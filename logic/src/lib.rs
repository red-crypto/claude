//! Hardware-independent logic.
//!
//! Everything here is `no_std` and has no HAL dependency, so it can be
//! unit-tested on the host with a plain `cargo test` (fast inner loop).
//! The firmware crate wires it to real peripherals.
#![cfg_attr(not(test), no_std)]

/// Blink pattern selected by the user button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum BlinkMode {
    Slow,
    Fast,
    Off,
}

impl BlinkMode {
    /// Next mode on each button press: Slow -> Fast -> Off -> Slow.
    pub const fn next(self) -> Self {
        match self {
            BlinkMode::Slow => BlinkMode::Fast,
            BlinkMode::Fast => BlinkMode::Off,
            BlinkMode::Off => BlinkMode::Slow,
        }
    }

    /// Half period in milliseconds, `None` when the LED stays off.
    pub const fn half_period_ms(self) -> Option<u64> {
        match self {
            BlinkMode::Slow => Some(500),
            BlinkMode::Fast => Some(100),
            BlinkMode::Off => None,
        }
    }
}

/// Simple integrator-style debouncer for a sampled digital input.
pub struct Debouncer {
    count: u8,
    threshold: u8,
    state: bool,
}

impl Debouncer {
    pub const fn new(threshold: u8) -> Self {
        Self {
            count: 0,
            threshold,
            state: false,
        }
    }

    /// Feed one raw sample. Returns `Some(new_state)` on a debounced edge.
    pub fn update(&mut self, raw: bool) -> Option<bool> {
        if raw == self.state {
            self.count = 0;
            return None;
        }
        self.count += 1;
        if self.count >= self.threshold {
            self.count = 0;
            self.state = raw;
            Some(raw)
        } else {
            None
        }
    }

    pub const fn state(&self) -> bool {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blink_mode_cycles() {
        let m = BlinkMode::Slow;
        assert_eq!(m.next(), BlinkMode::Fast);
        assert_eq!(m.next().next(), BlinkMode::Off);
        assert_eq!(m.next().next().next(), BlinkMode::Slow);
        assert_eq!(BlinkMode::Off.half_period_ms(), None);
    }

    #[test]
    fn debouncer_rejects_glitch() {
        let mut d = Debouncer::new(3);
        assert_eq!(d.update(true), None);
        assert_eq!(d.update(false), None); // glitch resets the counter
        assert_eq!(d.update(true), None);
        assert_eq!(d.update(true), None);
        assert_eq!(d.update(true), Some(true));
        assert!(d.state());
    }
}
