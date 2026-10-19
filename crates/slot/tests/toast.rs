mod common;

use common::{app_playing_in, tmp_root_with_carts};
use slot_input::Action;
use slot_ui::{Toast, HUD_MS};

#[test]
fn a_save_hotkey_says_state_saved() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.apply(Action::SaveState);
    assert_eq!(a.toast(), Some(Toast::StateSaved));
}

#[test]
fn a_load_says_state_loaded() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.apply(Action::SaveState);
    a.apply(Action::LoadState);
    assert_eq!(a.toast(), Some(Toast::StateLoaded));
}

#[test]
fn a_refused_load_says_nothing() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.apply(Action::LoadState);
    assert_eq!(a.toast(), None, "it announced a load with an empty ring");
    assert!(a.refusal_active(a.now()), "and it did not shake either");
}

#[test]
fn a_toast_goes_away_on_its_own() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.apply_at(Action::SaveState, 1_000);
    assert_eq!(a.toast(), Some(Toast::StateSaved));
    a.tick_ms(1_000 + HUD_MS);
    assert_eq!(a.toast(), None);
}

#[test]
fn a_screenshot_hotkey_writes_a_png_and_says_screenshot_saved() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.apply_at(Action::Screenshot, 1_000);
    assert_eq!(a.toast(), Some(Toast::ScreenshotSaved));
    let shots_dir = d.path().join("Screenshots/GBA");
    let files: Vec<_> = std::fs::read_dir(&shots_dir)
        .expect("Screenshots/GBA dir")
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(files.len(), 1, "expected 1 screenshot file, got {files:?}");
    let name = files[0].file_name().to_string_lossy().into_owned();
    assert!(
        name.starts_with("Emerald-") && name.ends_with(".png"),
        "unexpected screenshot filename {name}"
    );
}
