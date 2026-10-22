//! Software standby: while the device dozes (lid shut, or a tap of POWER), the main loop
//! stops drawing and stops clocking the emulator, and only wakes a few times a second to
//! read input, poll the lid and run the app's timers (the doze timeout among them).
//!
//! This is not a hardware suspend: the CPU, the memory and the kernel keep running. It only
//! takes the GPU, the swap chain and the per-frame work out of a screen nobody can see.
//!
//! The decision lives here, apart from the device loop, so it can be tested on any machine.

use std::time::Duration;

/// How long the loop rests between input polls while standing by. The lid is read every
/// 150 ms (`LID_POLL_MS`), so a 50 ms rest still sees every lid change on its next poll and
/// adds at most 50 ms before a press or an opened lid is noticed.
pub const STANDBY_REST: Duration = Duration::from_millis(50);

/// Black frames presented after a doze begins before drawing stops, one per buffer of the
/// swap chain, so the panel holds black and not the last game frame behind the backlight.
pub const DARK_FRAMES: u8 = 2;

/// What the loop does this time round.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Step {
    /// A normal frame: step the emulator, render, swap.
    Draw,
    /// Standing by: rest, read input and run timers, but draw nothing.
    Rest,
}

#[derive(Clone, Debug, Default)]
pub struct Standby {
    dark: u8,
}

impl Standby {
    pub fn new() -> Self {
        Self::default()
    }

    /// Decides this pass from whether the app is dozing now. Waking draws on the very same
    /// pass, so the first thing seen after the lid opens is a fresh frame.
    pub fn step(&mut self, dozing: bool) -> Step {
        if !dozing {
            self.dark = 0;
            return Step::Draw;
        }
        if self.dark < DARK_FRAMES {
            self.dark += 1;
            return Step::Draw;
        }
        Step::Rest
    }

    pub fn resting(&self) -> bool {
        self.dark >= DARK_FRAMES
    }
}
