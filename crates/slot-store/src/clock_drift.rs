//! Keeping the clock right across a power off on a device whose RTC runs off the H700's
//! internal oscillator, which ticks about 11% slow and can lose hours a day while the device is
//! off. While it is on, Linux keeps time from the main crystal and stays accurate.
//!
//! slot writes the system time into the RTC as it powers off and notes that moment here
//! (`off_at`). At the next boot, the RTC's count since then is scaled by `rate`, the real
//! seconds per RTC second, and the clock is moved to the result. `rate` starts at 1 (no
//! correction) and is learned from the next time the clock is set by hand after a power off.
//!
//! `Config/clock.txt`:
//!
//! ```text
//! off_at = 1792800000
//! rate = 1.12950
//! ```

use std::path::Path;

pub const CLOCK_FILE: &str = "Config/clock.txt";

/// A power off shorter than this is not used to learn the rate: the clock is set to the
/// minute, and over a shorter span that minute is too much of the answer.
pub const LEARN_AFTER_SECS: i64 = 3 * 60 * 60;

/// Rates outside this are a broken RTC or a mis-set clock, not oscillator drift.
const RATE_MIN: f64 = 0.5;
const RATE_MAX: f64 = 2.0;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ClockRecord {
    /// When slot last powered off with the RTC set to the system time, in UTC seconds.
    pub off_at: Option<i64>,
    /// Real seconds per RTC second while powered off.
    pub rate: f64,
}

impl Default for ClockRecord {
    fn default() -> Self {
        ClockRecord {
            off_at: None,
            rate: 1.0,
        }
    }
}

/// What a boot found: how long the RTC counted while off, at which rate it was corrected, and
/// the corrected time. Kept for the session so a later hand correction can teach the rate.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BootCorrection {
    pub rtc_off: i64,
    pub rate: f64,
    pub corrected: i64,
}

pub fn read_clock(root: &Path) -> ClockRecord {
    let map = crate::ini::read(root, CLOCK_FILE);
    ClockRecord {
        off_at: map.get("off_at").and_then(|v| v.parse().ok()),
        rate: map
            .get("rate")
            .and_then(|v| v.parse::<f64>().ok())
            .filter(|r| (RATE_MIN..=RATE_MAX).contains(r))
            .unwrap_or(1.0),
    }
}

pub fn write_clock(root: &Path, record: &ClockRecord) -> std::io::Result<()> {
    let mut text = String::from(
        "# Written by slot to keep the clock right while the device is off. Delete to reset.\n",
    );
    if let Some(off_at) = record.off_at {
        text.push_str(&format!("off_at = {off_at}\n"));
    }
    text.push_str(&format!("rate = {:.5}\n", record.rate));
    let path = root.join(CLOCK_FILE);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    crate::atomic::atomic_write(&path, text.as_bytes())
}

/// The corrected time for a boot whose clock reads `now`, after a power off at `off_at`.
/// None when there is nothing to correct from: the RTC was reset or went backwards.
pub fn correct_boot(off_at: i64, now: i64, rate: f64) -> Option<BootCorrection> {
    let rtc_off = now.checked_sub(off_at).filter(|&s| s >= 0)?;
    let corrected = off_at + (rtc_off as f64 * rate).round() as i64;
    Some(BootCorrection {
        rtc_off,
        rate,
        corrected,
    })
}

/// The rate a hand correction of `error` seconds (the set time minus the clock's) teaches,
/// given the boot it follows. None when the power off was too short to learn from, or the
/// answer is not believable.
pub fn learned_rate(boot: &BootCorrection, error: i64) -> Option<f64> {
    if boot.rtc_off < LEARN_AFTER_SECS {
        return None;
    }
    let rate = boot.rate + error as f64 / boot.rtc_off as f64;
    (RATE_MIN..=RATE_MAX).contains(&rate).then_some(rate)
}
