use std::fmt;
use std::path::Path;

use crate::link::Link;
use crate::rumble::Rumble;

pub const GBA_W: u32 = 240;
pub const GBA_H: u32 = 160;

#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct ButtonMask(pub u16);

impl ButtonMask {
    pub const B: u16 = 1 << 0;
    pub const Y: u16 = 1 << 1;
    pub const SELECT: u16 = 1 << 2;
    pub const START: u16 = 1 << 3;
    pub const UP: u16 = 1 << 4;
    pub const DOWN: u16 = 1 << 5;
    pub const LEFT: u16 = 1 << 6;
    pub const RIGHT: u16 = 1 << 7;
    pub const A: u16 = 1 << 8;
    pub const X: u16 = 1 << 9;
    pub const L: u16 = 1 << 10;
    pub const R: u16 = 1 << 11;

    pub fn without_turbo(self) -> ButtonMask {
        ButtonMask(self.0 & !(Self::X | Self::Y))
    }

    pub fn turbo(self, frame: u32) -> ButtonMask {
        let mut mask = self.0 & !(Self::X | Self::Y);
        if (frame / 3).is_multiple_of(2) {
            if self.0 & Self::X != 0 {
                mask |= Self::A;
            }
            if self.0 & Self::Y != 0 {
                mask |= Self::B;
            }
        }
        ButtonMask(mask)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct AvInfo {
    pub fps: f64,
    pub sample_rate: f64,
}

#[derive(Debug)]
pub enum CoreError {
    Io(std::io::Error),
    Load(String),
    Unsupported(String),
    State(String),
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreError::Io(e) => write!(f, "io: {e}"),
            CoreError::Load(m) => write!(f, "load: {m}"),
            CoreError::Unsupported(m) => write!(f, "unsupported: {m}"),
            CoreError::State(m) => write!(f, "state: {m}"),
        }
    }
}

impl std::error::Error for CoreError {}

impl From<std::io::Error> for CoreError {
    fn from(e: std::io::Error) -> Self {
        CoreError::Io(e)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum LightColour {
    Gba,
    Gbc,
}

// Built at compile time by build.rs from src/light_lut.rs, so picking Light never stalls a frame.
include!(concat!(env!("OUT_DIR"), "/light_lut.rs"));

impl LightColour {
    pub fn lut(self) -> &'static [u32; 65536] {
        match self {
            LightColour::Gba => &GBA_LIGHT_LUT,
            LightColour::Gbc => &GBC_LIGHT_LUT,
        }
    }
}

pub trait RetroCore: Send {
    fn load(&mut self, rom: &Path) -> Result<(), CoreError>;
    fn run_frame(&mut self, input: ButtonMask);
    fn run_frame_linked(&mut self, p1: ButtonMask, _p2: ButtonMask) {
        self.run_frame(p1);
    }
    fn set_option(&mut self, _key: &str, _value: &str) {}
    fn set_light_colour(&mut self, _light: Option<LightColour>) {}
    fn set_frame_skip(&mut self, _skip: bool) {}
    fn video_xrgb8888(&self) -> &[u8];
    fn take_audio(&mut self) -> Vec<i16>;
    /// Hands a buffer from `take_audio` back once its samples have been used, so the core can
    /// fill it again instead of allocating a fresh one every frame batch. A core that does not
    /// reuse buffers drops it, which is what the default does.
    fn recycle_audio(&mut self, _buf: Vec<i16>) {}
    fn serialize(&mut self) -> Result<Vec<u8>, CoreError>;
    fn unserialize(&mut self, data: &[u8]) -> Result<(), CoreError>;
    fn save_ram(&self) -> Option<Vec<u8>>;
    fn load_save_ram(&mut self, data: &[u8]) -> Result<(), CoreError>;
    fn av_info(&self) -> AvInfo;
    fn rumble(&self) -> Rumble {
        Rumble::default()
    }
    fn net(&self) -> Link {
        Link::default()
    }
    fn start_link(&mut self, _client_id: u16) {}
    fn pump_link(&mut self) {}
    fn stop_link(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Recorder(Vec<ButtonMask>);

    impl RetroCore for Recorder {
        fn load(&mut self, _rom: &Path) -> Result<(), CoreError> {
            Ok(())
        }
        fn run_frame(&mut self, input: ButtonMask) {
            self.0.push(input);
        }
        fn video_xrgb8888(&self) -> &[u8] {
            &[]
        }
        fn take_audio(&mut self) -> Vec<i16> {
            Vec::new()
        }
        fn serialize(&mut self) -> Result<Vec<u8>, CoreError> {
            Ok(Vec::new())
        }
        fn unserialize(&mut self, _data: &[u8]) -> Result<(), CoreError> {
            Ok(())
        }
        fn save_ram(&self) -> Option<Vec<u8>> {
            None
        }
        fn load_save_ram(&mut self, _data: &[u8]) -> Result<(), CoreError> {
            Ok(())
        }
        fn av_info(&self) -> AvInfo {
            AvInfo {
                fps: 60.0,
                sample_rate: 48_000.0,
            }
        }
    }

    #[test]
    fn a_core_without_link_mode_runs_player_1_alone() {
        let mut core = Recorder::default();
        core.run_frame_linked(ButtonMask(ButtonMask::A), ButtonMask(ButtonMask::B));
        assert_eq!(core.0, vec![ButtonMask(ButtonMask::A)]);
    }

    #[test]
    fn light_colour_lut_sits_between_raw_and_full_correction() {
        for mode in [LightColour::Gba, LightColour::Gbc] {
            let lut = mode.lut();
            assert_eq!(lut[0], 0, "black stays black");
            let pure_green = 0x07e0usize;
            let px = lut[pure_green];
            let r = (px >> 16) & 0xff;
            let g = (px >> 8) & 0xff;
            let b = px & 0xff;
            assert!(
                g > 200 && g < 255,
                "{mode:?}: pure green channel should be tamed below 255 but stay bright: {g}"
            );
            assert!(
                r > 0 && b > 0,
                "{mode:?}: pure green should bleed slightly into r ({r}) and b ({b})"
            );
        }
        assert_ne!(
            LightColour::Gba.lut()[0x4210],
            LightColour::Gbc.lut()[0x4210],
            "GBA and GBC light LUTs should reflect their distinct gamma curves at mid-grey"
        );
    }

    #[test]
    fn the_baked_light_tables_match_the_maths_they_were_built_from() {
        // Within one step per channel, in case a target's powf rounds differently from the
        // build machine's.
        use crate::light_lut::{light_entry, GBA_GAMMA_ADJ, GBC_GAMMA_ADJ};
        for (mode, adj) in [
            (LightColour::Gba, GBA_GAMMA_ADJ),
            (LightColour::Gbc, GBC_GAMMA_ADJ),
        ] {
            let lut = mode.lut();
            for (color, &baked) in lut.iter().enumerate() {
                let want = light_entry(color, adj);
                for shift in [0, 8, 16] {
                    let a = (baked >> shift) & 0xff;
                    let b = (want >> shift) & 0xff;
                    assert!(
                        a.abs_diff(b) <= 1,
                        "{mode:?} entry {color:#06x}: baked {baked:#08x}, maths {want:#08x}"
                    );
                }
            }
        }
    }
}
