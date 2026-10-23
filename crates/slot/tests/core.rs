mod common;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use common::core_lock;
use slot::app::Phase;
use slot::audio::{AudioSink, StubSink};
use slot::core::{apply_core_options, open_core_for};
use slot::emu::{CoreState, EmuHandle};
use slot::session::Session;
use slot_input::{Btn, RawEvent};
use slot_retro::{ButtonMask, LibretroCore, RetroCore};
use slot_store::{ColourCorrection, Core, Platform};

fn rom(name: &str) -> PathBuf {
    let p = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::write(&p, common::gba_rom()).expect("write rom");
    p
}

fn is_mgba(core: &mut dyn RetroCore, rom: &Path) -> bool {
    core.load(rom).expect("core refused the test rom");
    core.run_frame(ButtonMask::default());
    core.serialize().expect("core gave up no state").len() > 100_000
}

#[test]
fn the_vendored_core_is_preferred_over_the_mock() {
    let _g = core_lock();
    let Some(dylib) = common::vendored_core() else {
        return;
    };
    let d = common::tmp_root_with_real_carts(&[]);
    let mut core = open_core_for(
        d.path(),
        Core::Mgba,
        "auto",
        ColourCorrection::Off,
        &[dylib],
    );
    assert!(
        is_mgba(core.as_mut(), &rom("preferred.gba")),
        "the mock ran with a vendored core sitting right there"
    );
}

#[test]
fn a_missing_core_falls_back_to_the_mock_rather_than_failing() {
    let d = common::tmp_root_with_real_carts(&[]);
    let mut core = open_core_for(
        d.path(),
        Core::Mgba,
        "auto",
        ColourCorrection::Off,
        &[PathBuf::from("no/such/core.dylib")],
    );
    assert!(!is_mgba(core.as_mut(), &rom("missing.gba")));
}

#[test]
fn a_rom_the_real_core_refuses_reports_failed() {
    let _g = core_lock();
    let Some(dylib) = common::vendored_core() else {
        return;
    };
    let d = common::tmp_root_with_carts(&["Broken"]);
    let emu = EmuHandle::spawn(
        open_core_for(
            d.path(),
            Core::Mgba,
            "auto",
            ColourCorrection::Off,
            &[dylib],
        ),
        d.path().join("Games/GBA/Broken.gba"),
        StubSink::new().ring(),
        None,
        None,
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    while emu.state() == CoreState::Loading {
        assert!(Instant::now() < deadline, "the core never settled");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(emu.state(), CoreState::Failed);
}

#[test]
fn a_cart_the_real_core_refuses_comes_back_out_of_the_slot() {
    let _g = core_lock();
    let Some(dylib) = common::vendored_core() else {
        return;
    };
    std::env::set_var("SLOT_CORE", dylib);
    let d = common::tmp_root_with_carts(&["Broken"]);
    common::clocked(d.path());
    let mut s = Session::boot(d.path().to_path_buf());
    s.feed([RawEvent::Down(Btn::A)], 16);
    s.update(1.0 / 60.0);
    assert!(matches!(s.app().phase(), Phase::Inserting { .. }));

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut now = 16;
    while !matches!(s.app().phase(), Phase::Shelf) {
        assert!(
            Instant::now() < deadline,
            "the refused cart never came back out"
        );
        now += 16;
        s.feed([], now);
        s.update(1.0 / 60.0);
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn formatting_presses(frame: u32) -> ButtonMask {
    ButtonMask(match (frame / 15) % 4 {
        0 if frame > 120 => ButtonMask::START,
        2 if frame > 120 => ButtonMask::A,
        _ => 0,
    })
}

#[test]
fn on_mgba_loading_a_state_while_the_game_writes_its_second_flash_bank_keeps_the_save_whole() {
    let _g = core_lock();
    let rom = common::repo_root().join(
        "sdcard/Games/GBA/Super Mario Advance 4 - Super Mario Bros. 3 (USA, Australia) (Rev 1).gba",
    );
    let Some(dylib) = common::vendored_core().filter(|_| rom.exists()) else {
        eprintln!("no vendored mGBA core or no Super Mario Advance 4 under sdcard/Games, skipping");
        return;
    };
    let save_after_formatting = |reload: bool| {
        let mut core =
            LibretroCore::open(&dylib).expect("vendored core is present but would not open");
        apply_core_options(
            &mut core,
            Core::Mgba,
            Platform::Gba,
            "auto",
            false,
            ColourCorrection::Off,
            slot_store::Blending::Off,
            None,
        );
        core.load(&rom).expect("mGBA refused the rom");
        for frame in 0..900 {
            core.run_frame(formatting_presses(frame));
            let _ = core.take_audio();
            if reload {
                let state = core.serialize().expect("mGBA would not save");
                core.unserialize(&state)
                    .expect("mGBA would not load its own state");
            }
        }
        core.save_ram().expect("mGBA gave no save")
    };
    let played = save_after_formatting(false);
    assert!(
        played[..8] == *b"SMA4MWE0",
        "the game never formatted its save, so this test proves nothing"
    );
    let reloaded = save_after_formatting(true);
    let wrong = reloaded.iter().zip(&played).filter(|(a, b)| a != b).count();
    assert_eq!(
        wrong, 0,
        "loading a state on every frame of the format left {wrong} bytes of the save different \
         from playing straight through"
    );
}

fn rom_running(name: &str, code: &[u32]) -> PathBuf {
    let mut rom = common::gba_rom();
    for (i, w) in code.iter().enumerate() {
        rom[0xc0 + i * 4..0xc4 + i * 4].copy_from_slice(&w.to_le_bytes());
    }
    let p = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::write(&p, rom).expect("write rom");
    p
}

fn sram_rom() -> PathBuf {
    const CODE: [u32; 28] = [
        0xe3a00301, 0xe3a0540e, 0xe3a0105a, 0xe5c51001, 0xe3a01b01, 0xe3811003, 0xe5801000,
        0xe3a02406, 0xe3a03000, 0xe3e0c902, 0xe2807e13, 0xe3a06000, 0xe3a0a0a5, 0xe1d040b6,
        0xe35400a0, 0x1afffffc, 0xe1d740b0, 0xe2044001, 0xe2244001, 0xe0149006, 0x15c5a000,
        0x11c2c0b0, 0x01c230b0, 0xe1a06004, 0xe1d040b6, 0xe35400a0, 0x0afffffc, 0xeafffff0,
    ];
    rom_running("sram.gba", &CODE)
}

fn growing_eeprom_rom() -> PathBuf {
    const CODE: [u32; 54] = [
        0xe3a00301, 0xe2807e13, 0xe3a06000, 0xe3a09001, 0xe3a0c011, 0xeb000010, 0xe1d040b6,
        0xe35400a0, 0x1afffffc, 0xe1d740b0, 0xe2044001, 0xe2244001, 0xe1c49006, 0xe3590001,
        0x1a000002, 0xe3a09064, 0xe3a0c022, 0xeb000004, 0xe1a06004, 0xe1d040b6, 0xe35400a0,
        0x0afffffc, 0xeaffffee, 0xe3a01402, 0xe3a03001, 0xe0c130b2, 0xe3a03000, 0xe0c130b2,
        0xe3a0a00d, 0xe1a03a39, 0xe2033001, 0xe0c130b2, 0xe25aa001, 0x5afffffa, 0xe3a0b008,
        0xe3a0a007, 0xe1a03a3c, 0xe2033001, 0xe0c130b2, 0xe25aa001, 0x5afffffa, 0xe25bb001,
        0x1afffff7, 0xe3a03000, 0xe0c130b2, 0xe280c0d4, 0xe3a01402, 0xe58c1000, 0xe3a0140d,
        0xe58c1004, 0xe3a01051, 0xe3811102, 0xe58c1008, 0xe12fff1e,
    ];
    rom_running("eeprom.gba", &CODE)
}

fn mgba_with(dylib: &Path, rom: &Path) -> LibretroCore {
    let mut core = LibretroCore::open(dylib).expect("vendored core is present but would not open");
    apply_core_options(
        &mut core,
        Core::Mgba,
        Platform::Gba,
        "auto",
        false,
        ColourCorrection::Off,
        slot_store::Blending::Off,
        None,
    );
    core.load(rom).expect("mGBA refused the rom");
    core
}

fn play(core: &mut LibretroCore, frames: usize, input: u16) -> Vec<u8> {
    for _ in 0..frames {
        core.run_frame(ButtonMask(input));
        let _ = core.take_audio();
    }
    core.video_xrgb8888().to_vec()
}

#[test]
fn on_mgba_a_state_saved_once_the_game_settles_on_its_save_chip_is_that_chips_size() {
    let _g = core_lock();
    let Some(dylib) = common::vendored_core() else {
        eprintln!("no vendored mGBA core on this host, skipping");
        return;
    };
    let rom = sram_rom();
    let mut early = mgba_with(&dylib, &rom);
    let undetected = early.serialize().expect("mGBA would not save").len();
    play(&mut early, 30, 0);
    let after = early.serialize().expect("mGBA would not save").len();
    drop(early);

    let mut late = mgba_with(&dylib, &rom);
    play(&mut late, 30, 0);
    let exact = late.serialize().expect("mGBA would not save").len();
    assert!(
        exact < undetected,
        "mGBA's state did not shrink once the chip was found, so this proves nothing"
    );
    assert_eq!(
        after,
        exact,
        "a save taken before the game touched its chip left every later state {} bytes too big",
        after - exact
    );
}

#[test]
fn on_mgba_a_save_chip_that_grows_mid_game_still_saves_whole() {
    let _g = core_lock();
    let Some(dylib) = common::vendored_core() else {
        eprintln!("no vendored mGBA core on this host, skipping");
        return;
    };
    let rom = growing_eeprom_rom();
    let mut core = mgba_with(&dylib, &rom);
    play(&mut core, 5, 0);
    let small = core.serialize().expect("mGBA would not save").len();
    play(&mut core, 1, ButtonMask::A);
    play(&mut core, 10, 0);
    let grown = core.serialize().expect("mGBA would not save a grown chip");
    assert!(
        grown.len() > small,
        "the chip never grew, so this proves nothing"
    );
    core.unserialize(&grown)
        .expect("mGBA would not load the state of its grown chip");
    drop(core);

    let mut fresh = mgba_with(&dylib, &rom);
    play(&mut fresh, 5, 0);
    play(&mut fresh, 1, ButtonMask::A);
    play(&mut fresh, 10, 0);
    assert_eq!(
        grown.len(),
        fresh.serialize().expect("mGBA would not save").len(),
        "the state after the chip grew is not the size the core asks for"
    );
}

#[test]
fn on_mgba_a_state_an_older_slot_saved_with_slack_still_loads() {
    let _g = core_lock();
    let Some(dylib) = common::vendored_core() else {
        eprintln!("no vendored mGBA core on this host, skipping");
        return;
    };
    let rom = sram_rom();
    let undetected = mgba_with(&dylib, &rom)
        .serialize()
        .expect("mGBA would not save")
        .len();
    let mut core = mgba_with(&dylib, &rom);
    play(&mut core, 30, 0);
    let exact = core.serialize().expect("mGBA would not save");
    let mut slack = exact.clone();
    slack.resize(undetected, 0);

    core.unserialize(&exact)
        .expect("mGBA would not load its own state");
    let expected = play(&mut core, 60, 0);
    core.unserialize(&slack)
        .expect("mGBA refused a state with the slack an older slot left after it");
    assert_eq!(
        play(&mut core, 60, 0),
        expected,
        "a state with slack after it plays on differently from the same state without"
    );
}
