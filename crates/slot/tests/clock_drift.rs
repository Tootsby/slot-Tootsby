mod common;

use common::{app_booting_at, tmp_root_with_carts};
use slot::app::{App, Phase};
use slot_input::{Action, Btn};
use slot_store::clock_drift::{read_clock, write_clock, ClockRecord};
use slot_store::{write_slot_state, SlotState};
use slot_ui::{Field, QuickRow};

const OFF_AT: i64 = 1_786_568_000;
const TEN_HOURS: i64 = 10 * 3600;

fn card(clock_set: bool, record: ClockRecord) -> tempfile::TempDir {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    write_slot_state(
        d.path(),
        &SlotState {
            clock_set,
            ..SlotState::default()
        },
    )
    .unwrap();
    write_clock(d.path(), &record).unwrap();
    d
}

fn open_clock(a: &mut App) {
    a.apply(Action::QuickMenu);
    for _ in 0..QuickRow::General.position(&[]) {
        a.apply(Action::GbaDown(Btn::Down));
    }
    a.apply(Action::GbaDown(Btn::A));
    for _ in 0..QuickRow::DateTime.position(&a.hidden_rows()) {
        a.apply(Action::GbaDown(Btn::Down));
    }
    a.apply(Action::GbaDown(Btn::A));
    assert!(
        matches!(a.phase(), Phase::SetClock { .. }),
        "{:?}",
        a.phase()
    );
}

fn minute_up(a: &mut App) {
    while a.picker().expect("on the clock").cursor() != Field::Minute {
        a.apply(Action::GbaDown(Btn::Right));
    }
    a.apply(Action::GbaDown(Btn::Up));
    a.apply(Action::GbaDown(Btn::A));
}

#[test]
fn a_boot_after_a_power_off_scales_what_the_rtc_counted() {
    let d = card(
        true,
        ClockRecord {
            off_at: Some(OFF_AT),
            rate: 1.1,
        },
    );
    let (_a, clock) = app_booting_at(d.path(), OFF_AT + TEN_HOURS);
    assert_eq!(
        clock.get(),
        OFF_AT + 11 * 3600,
        "the clock was not corrected"
    );
    let record = read_clock(d.path());
    assert_eq!(
        record.off_at, None,
        "the power off was left to be used twice"
    );
    assert_eq!(record.rate, 1.1, "the rate was lost");
}

#[test]
fn a_boot_with_no_clean_power_off_on_record_leaves_the_clock() {
    let d = card(
        true,
        ClockRecord {
            off_at: None,
            rate: 1.1,
        },
    );
    let (_a, clock) = app_booting_at(d.path(), OFF_AT + TEN_HOURS);
    assert_eq!(clock.get(), OFF_AT + TEN_HOURS);
}

#[test]
fn a_clock_never_set_by_hand_is_not_corrected() {
    let d = card(
        false,
        ClockRecord {
            off_at: Some(OFF_AT),
            rate: 1.1,
        },
    );
    let (_a, clock) = app_booting_at(d.path(), OFF_AT + TEN_HOURS);
    assert_eq!(clock.get(), OFF_AT + TEN_HOURS);
}

#[test]
fn setting_the_clock_after_a_long_power_off_teaches_the_rate() {
    let d = card(
        true,
        ClockRecord {
            off_at: Some(OFF_AT),
            rate: 1.0,
        },
    );
    let (mut a, _clock) = app_booting_at(d.path(), OFF_AT + TEN_HOURS);
    open_clock(&mut a);
    minute_up(&mut a);
    let rate = read_clock(d.path()).rate;
    let want = 1.0 + 60.0 / TEN_HOURS as f64;
    assert!((rate - want).abs() < 1e-5, "learned {rate}, wanted {want}");
}

#[test]
fn only_the_first_correction_after_a_boot_teaches_the_rate() {
    let d = card(
        true,
        ClockRecord {
            off_at: Some(OFF_AT),
            rate: 1.0,
        },
    );
    let (mut a, _clock) = app_booting_at(d.path(), OFF_AT + TEN_HOURS);
    open_clock(&mut a);
    minute_up(&mut a);
    let first = read_clock(d.path()).rate;
    a.apply(Action::GbaDown(Btn::A));
    assert!(
        matches!(a.phase(), Phase::SetClock { .. }),
        "{:?}",
        a.phase()
    );
    minute_up(&mut a);
    assert_eq!(read_clock(d.path()).rate, first);
}

#[test]
fn a_power_off_records_when_it_happened() {
    let d = card(true, ClockRecord::default());
    let (mut a, clock) = app_booting_at(d.path(), OFF_AT);
    clock.advance(500);
    a.note_power_off();
    assert_eq!(read_clock(d.path()).off_at, Some(OFF_AT + 500));
}

#[test]
fn a_power_off_before_the_clock_was_ever_set_records_nothing() {
    let d = card(false, ClockRecord::default());
    let (mut a, _clock) = app_booting_at(d.path(), OFF_AT);
    a.note_power_off();
    assert_eq!(read_clock(d.path()).off_at, None);
}
