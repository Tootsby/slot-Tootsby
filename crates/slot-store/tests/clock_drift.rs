use slot_store::clock_drift::{
    correct_boot, learned_rate, read_clock, write_clock, ClockRecord, LEARN_AFTER_SECS,
};

#[test]
fn a_card_without_the_file_corrects_nothing() {
    let d = tempfile::tempdir().unwrap();
    assert_eq!(read_clock(d.path()), ClockRecord::default());
    assert_eq!(read_clock(d.path()).rate, 1.0);
}

#[test]
fn the_record_survives_a_round_trip() {
    let d = tempfile::tempdir().unwrap();
    let record = ClockRecord {
        off_at: Some(1_792_800_000),
        rate: 1.1295,
    };
    write_clock(d.path(), &record).unwrap();
    assert_eq!(read_clock(d.path()), record);
}

#[test]
fn an_unbelievable_rate_on_the_card_is_ignored() {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(d.path().join("Config")).unwrap();
    std::fs::write(d.path().join("Config/clock.txt"), "rate = 40\n").unwrap();
    assert_eq!(read_clock(d.path()).rate, 1.0);
}

#[test]
fn a_boot_scales_the_time_the_rtc_counted_while_off() {
    let boot = correct_boot(1_000_000, 1_000_000 + 10 * 3600, 1.1).unwrap();
    assert_eq!(boot.rtc_off, 10 * 3600);
    assert_eq!(boot.corrected, 1_000_000 + 11 * 3600);
}

#[test]
fn an_rtc_that_went_backwards_is_left_alone() {
    assert_eq!(correct_boot(1_000_000, 999_000, 1.1), None);
}

#[test]
fn a_correction_by_hand_teaches_the_rate_that_would_have_got_it_right() {
    let boot = correct_boot(0, 10 * 3600, 1.0).unwrap();
    let rate = learned_rate(&boot, 3600).unwrap();
    assert!((rate - 1.1).abs() < 1e-9, "{rate}");
    let next = correct_boot(0, 10 * 3600, rate).unwrap();
    assert_eq!(
        next.corrected,
        11 * 3600,
        "the learned rate does not undo the error"
    );
}

#[test]
fn a_short_power_off_teaches_nothing() {
    let boot = correct_boot(0, LEARN_AFTER_SECS - 1, 1.0).unwrap();
    assert_eq!(learned_rate(&boot, 60), None);
}

#[test]
fn a_correction_too_big_to_be_drift_teaches_nothing() {
    let boot = correct_boot(0, 10 * 3600, 1.0).unwrap();
    assert_eq!(learned_rate(&boot, 20 * 3600), None);
}
