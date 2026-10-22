use std::path::PathBuf;
use std::time::{Duration, Instant};

use slot::frontend::Frontend;
use slot::input::DeviceInput;
use slot::pacing::FramePacer;
use slot::panel_turn::{panel_turn, PanelTurn};
use slot::standby::{Standby, Step, STANDBY_REST};
use slot_gfx::{Compositor, FbdevSurface, Surface};
use slot_power::{trace_first_frame, DevicePlatform};

const CARD: &str = "/mnt/sdcard";

const STEP_TIMEOUT: Duration = Duration::from_millis(12);

pub fn run() {
    slot::boot_time::mark("start");
    let _ = std::thread::Builder::new()
        .name("slot-fonts".into())
        .spawn(slot_ui::warm_fonts);
    let audio = std::thread::Builder::new()
        .name("slot-audio-open".into())
        .spawn(slot::audio::opened_sink);
    let root = std::env::var_os("SLOT_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(CARD));
    let mut surface = match FbdevSurface::new() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("slot: {e}");
            return;
        }
    };
    slot::boot_time::mark("surface");
    let mut compositor = match Compositor::new(&surface) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("slot: {e}");
            return;
        }
    };
    slot::boot_time::mark("compositor");
    let panel = surface.window_size();
    let env = |key| std::env::var(key).ok();
    let turn = panel_turn(
        panel,
        slot::baseos::var("BASEOS_PANEL_ROTATION_CCW").as_deref(),
        env("SLOT_ROTATE").as_deref(),
    );
    let turned = turn != PanelTurn::None;
    let window = if turned { (panel.1, panel.0) } else { panel };
    eprintln!(
        "slot: baseos target {}, model {}, rotation ccw {}",
        slot::baseos::var("BASEOS_TARGET").unwrap_or_default(),
        slot::baseos::var("BASEOS_MODEL").unwrap_or_default(),
        slot::baseos::var("BASEOS_PANEL_ROTATION_CCW").unwrap_or_default()
    );
    eprintln!(
        "slot: panel {}x{}, canvas {}x{}, turn {turn:?}",
        panel.0, panel.1, window.0, window.1
    );
    if turned {
        if let Err(e) = compositor.set_panel_turn(panel, turn == PanelTurn::Ccw) {
            eprintln!("slot: {e}");
            return;
        }
    }
    let platform = DevicePlatform::new(root.clone());
    eprintln!("slot: {}", platform.report());
    platform.trace_boot();
    slot::boot_time::mark("platform");
    let sink = match audio.map(|h| h.join()) {
        Ok(Ok(sink)) => sink,
        _ => slot::audio::opened_sink(),
    };
    slot::boot_time::mark("audio");
    let mut frontend = Frontend::boot_with(Box::new(platform), sink);
    slot::boot_time::mark("frontend");
    let target = slot::baseos::var("BASEOS_TARGET").unwrap_or_default();
    frontend.set_left_stick(slot::input::evdev::has_left_stick(&target));
    frontend.upload_boot_faces(&mut compositor);
    slot::boot_time::mark("faces");
    frontend.upload_bezel(&mut compositor, window);
    slot::boot_time::mark("bezel");
    let mut input = DeviceInput::open(&root);
    slot::boot_time::mark("input");
    let card = root.clone();
    let _ = std::thread::Builder::new()
        .name("slot-bootlogo".into())
        .spawn(move || {
            slot::bootlogo::refresh(&card);
            slot::root::sweep_temps(&card);
        });
    let mut drawn = false;
    frontend.drive_emulator();
    let mut pacer = FramePacer::new(Instant::now());
    let mut standby = Standby::new();
    let (mut presents, mut steps, mut swaps, mut max_work) =
        (0u32, 0u32, Duration::ZERO, Duration::ZERO);
    let mut last_dropped = 0u64;
    loop {
        if standby.step(frontend.standing_by()) == Step::Rest {
            std::thread::sleep(STANDBY_REST);
            input.steer_with_stick(frontend.app().left_stick_dpad());
            frontend.advance(&mut input);
            if frontend.restarting() {
                frontend.restart();
            }
            if frontend.repairing() {
                frontend.repair();
            }
            if frontend.powering_off() {
                frontend.poweroff();
                return;
            }
            if slot::crash::stopping() {
                eprintln!("slot: told to stop, saving first");
                frontend.stop();
                return;
            }
            if !frontend.standing_by() {
                pacer.resume(Instant::now());
            }
            continue;
        }
        let clock = frontend.frame_clock();
        if let Some(wait) = pacer
            .deadline(clock, Instant::now())
            .checked_duration_since(Instant::now())
        {
            std::thread::sleep(wait);
        }
        let began = Instant::now();
        input.steer_with_stick(frontend.app().left_stick_dpad());
        frontend.advance(&mut input);
        if frontend.restarting() {
            frontend.restart();
        }
        if frontend.repairing() {
            frontend.repair();
        }
        if frontend.powering_off() {
            frontend.poweroff();
            return;
        }
        if slot::crash::stopping() {
            eprintln!("slot: told to stop, saving first");
            frontend.stop();
            return;
        }
        let clock = frontend.frame_clock();
        let due = pacer.due(clock, Instant::now());
        if due == 0 {
            continue;
        }
        for _ in 0..due {
            frontend.step_emulator(STEP_TIMEOUT);
            steps += 1;
            if !pacer.stepped(frontend.frame_clock(), Instant::now()) {
                break;
            }
        }
        frontend.render(&mut compositor, window);
        let swap = Instant::now();
        let work = swap - began;
        if let Err(e) = surface.swap() {
            eprintln!("slot: {e}");
            frontend.stop();
            return;
        }
        let swap_took = swap.elapsed();
        slot::latency::swapped(swap_took.as_secs_f64() * 1000.0);
        if slot::latency::tracing() {
            presents += 1;
            swaps += swap_took;
            max_work = max_work.max(work);
            if presents == 600 {
                let dropped = pacer.dropped - last_dropped;
                last_dropped = pacer.dropped;
                eprintln!(
                    "slot: pace: 600 presents, {steps} steps, {dropped} dropped, max work {:.1} ms, avg swap {:.2} ms, period {:.2} ms",
                    max_work.as_secs_f64() * 1e3,
                    swaps.as_secs_f64() * 1e3 / 600.0,
                    clock.period.as_secs_f64() * 1e3
                );
                (presents, steps, swaps, max_work) = (0, 0, Duration::ZERO, Duration::ZERO);
            }
        }
        if !drawn {
            drawn = true;
            slot::boot_time::mark("first frame");
            trace_first_frame();
        }
    }
}
